use core::cell::OnceCell;
use core::pin::Pin;
use core::ptr::NonNull;

use crate::boot::BootInfo;
use crate::mem::{MemoryRegion, MemoryType, VADDR};
use crate::sync::SpinLock;
use crate::{gfx, println};

pub mod fb;

#[repr(C)]
pub struct Request<const M0: u64, const M1: u64, R> {
    id: [u64; 4],
    rev: u64,
    pub response: Option<NonNull<R>>,
}

impl<const M0: u64, const M1: u64, R> Request<M0, M1, R> {
    pub const fn new() -> Self {
        Self {
            id: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, M0, M1],
            rev: 0,
            response: None,
        }
    }

    fn response(&self) -> Option<&R> {
        Some(unsafe { self.response?.as_ref() })
    }
}

unsafe impl<const M0: u64, const M1: u64, R> Send for Request<M0, M1, R> {}

pub type HHDMRequest = Request<0x48dcf1cb8ad2b852, 0x63984e959a98244b, HHDMResponse>;
pub type FbRequest = Request<0x9d5827dcd881dd75, 0xa3148604f6fab11b, FbResponse>;
pub type MemMapRequest = Request<0x67cf3d9d378a806f, 0xe304acdfc50c3c62, MemMapResponse>;
pub type DateAtBootRequest = Request<0x502746e184c088aa, 0xfbc5ec83e6327893, DateAtBootResponse>;

#[repr(C)]
pub struct HHDMResponse {
    rev: u64,
    offset: u64,
}

#[repr(C)]
pub struct FbResponse {
    rev: u64,
    pub count: u64,
    pub framebuffers: Option<NonNull<Option<NonNull<fb::Fb>>>>,
}

pub enum MemMapType {
    Usable = 0,
    Reserved = 1,
    AcpiReclaimable = 2,
    AcpiNvs = 3,
    BadMemory = 4,
    Reclaimable = 5,
    Executable = 6,
    Framebuffer = 7,
}

pub struct MemMapEntry {
    pub addr: u64,
    pub len: u64,
    pub mem_type: u64,
}

#[repr(C)]
pub struct MemMapResponse {
    rev: u64,
    pub count: u64,
    pub entries: Option<NonNull<Option<NonNull<MemMapEntry>>>>,
}

#[repr(C)]
pub struct DateAtBootResponse {
    rev: u64,
    timestamp: i64,
}

static FB_REQUEST: SpinLock<FbRequest> = SpinLock::new(FbRequest::new());
static HHDM_REQUEST: SpinLock<HHDMRequest> = SpinLock::new(HHDMRequest::new());
static MEMMAP_REQUEST: SpinLock<MemMapRequest> = SpinLock::new(MemMapRequest::new());
static DATE_AT_BOOT_REQUEST: SpinLock<DateAtBootRequest> = SpinLock::new(DateAtBootRequest::new());

static FB_CONSOLE: SpinLock<OnceCell<gfx::fb::Console>> = SpinLock::new(OnceCell::new());

pub struct MmapIterator {
    index: usize,
}

impl Iterator for MmapIterator {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        MEMMAP_REQUEST.with(|request| {
            let response = request.response()?;
            let entries = unsafe {
                core::slice::from_raw_parts(
                    response.entries?.as_ptr(),
                    response.count.try_into().unwrap(),
                )
            };
            if self.index < entries.len() {
                let entry = unsafe { entries[self.index]?.as_ref() };
                self.index += 1;
                let memory_type = if entry.mem_type == MemMapType::Usable as u64 {
                    MemoryType::Free
                } else if entry.mem_type == MemMapType::AcpiReclaimable as u64 {
                    MemoryType::AcpiReclaimable
                } else if entry.mem_type == MemMapType::AcpiNvs as u64 {
                    MemoryType::AcpiNvs
                } else if entry.mem_type == MemMapType::Reclaimable as u64 {
                    MemoryType::Reclaimable
                } else if entry.mem_type == MemMapType::BadMemory as u64 {
                    MemoryType::BadMemory
                } else if entry.mem_type == MemMapType::Executable as u64 {
                    MemoryType::Kernel
                } else if entry.mem_type == MemMapType::Framebuffer as u64 {
                    MemoryType::Framebuffer
                } else {
                    MemoryType::Reserved
                };
                Some(MemoryRegion::new(entry.addr, entry.len, memory_type))
            } else {
                None
            }
        })
    }
}

pub fn mmap_iter() -> MmapIterator {
    MmapIterator { index: 0 }
}

pub fn init() -> BootInfo<MmapIterator> {
    let fb = FB_REQUEST.with(|request| {
        let response = request.response()?;
        if response.count == 0 {
            return None;
        }
        let fb = unsafe { (*response.framebuffers?.as_ptr())?.as_ref() };
        println!(
            "framebuffer detected [addr={:?}, width={}, height={}, bpp={}]",
            fb.addr, fb.width, fb.height, fb.bpp
        );
        gfx::fb::Fb::try_new(
            fb.addr.cast(),
            fb.width.try_into().ok()?,
            fb.height.try_into().ok()?,
            fb.pitch.try_into().ok()?,
            fb.bpp,
            gfx::fb::Mask::new(fb.red_mask.size, fb.red_mask.shift),
            gfx::fb::Mask::new(fb.green_mask.size, fb.green_mask.shift),
            gfx::fb::Mask::new(fb.blue_mask.size, fb.blue_mask.shift),
        )
    });
    if let Some(fb) = fb {
        let console = FB_CONSOLE.acquire();
        console
            .set(gfx::fb::Console::new(fb, gfx::font::terminus16_8::FONT))
            .unwrap();
        let console = unsafe { Pin::new_unchecked(console.get().unwrap_unchecked().base()) };
        crate::log::register(console);
        println!("framebuffer console registered");
    } else {
        println!("framebuffer console not supported");
    }
    HHDM_REQUEST.with(|request| {
        let response = request
            .response()
            .expect("linear physical memory not mapped by bootloader");
        assert_eq!(
            response.offset, VADDR,
            "bad linear physical memory mapping at 0x{:x}",
            response.offset
        );
        println!("linear physical memory mapped at 0x{:x}", response.offset);
    });
    let boot_time = DATE_AT_BOOT_REQUEST.with(|request| {
        let response = request.response()?;
        Some(response.timestamp)
    });
    BootInfo {
        boot_time,
        mmap_iter,
    }
}
