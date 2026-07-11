use core::{cell::OnceCell, pin::Pin};

use limine::{date_at_boot, framebuffer, hhdm, memmap};

use crate::{
    boot::BootInfo,
    gfx::{self, fb::Fb},
    mem::{MemoryRegion, MemoryType, VADDR},
    println,
    sync::SpinLock,
};

#[repr(C)]
struct Requests {
    start_marker: [u64; 4],
    fb: framebuffer::Request,
    hhdm: hhdm::Request,
    memmap: memmap::Request,
    date_at_boot: date_at_boot::Request,
    end_marker: [u64; 2],
}

static REQUESTS: SpinLock<Requests> = SpinLock::new(Requests {
    start_marker: limine::requests_start_marker!(),
    fb: framebuffer::Request::new(),
    hhdm: hhdm::Request::new(),
    memmap: memmap::Request::new(),
    date_at_boot: date_at_boot::Request::new(),
    end_marker: limine::requests_end_marker!(),
});

static FB_CONSOLE: SpinLock<OnceCell<gfx::fb::Console>> = SpinLock::new(OnceCell::new());

pub struct MmapIter {
    index: usize,
}

impl MmapIter {
    pub fn new() -> Self {
        Self { index: 0 }
    }
}

impl Iterator for MmapIter {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        REQUESTS.with(|requests| {
            let response = requests.memmap.response()?;
            let entries = response.entries()?;
            if self.index < entries.len() {
                let entry = unsafe { entries[self.index].as_ref()? };
                self.index += 1;

                let memory_type = match entry.mem_type() {
                    memmap::Type::Usable => MemoryType::Free,
                    memmap::Type::Reserved => MemoryType::Reserved,
                    memmap::Type::AcpiReclaimable => MemoryType::AcpiReclaimable,
                    memmap::Type::AcpiNvs => MemoryType::AcpiNvs,
                    memmap::Type::BadMemory => MemoryType::BadMemory,
                    memmap::Type::BootloaderReclaimable => MemoryType::Reclaimable,
                    memmap::Type::ExecutableAndModules => MemoryType::Kernel,
                    memmap::Type::Framebuffer => MemoryType::Framebuffer,
                    memmap::Type::ReservedMapped => MemoryType::Reserved,
                    memmap::Type::Unknown => MemoryType::Reserved,
                };

                Some(MemoryRegion::new(
                    entry.base().try_into().unwrap(),
                    entry.len().try_into().unwrap(),
                    memory_type,
                ))
            } else {
                None
            }
        })
    }
}

fn find_framebuffer(response: Option<&framebuffer::Response>) -> Option<Fb> {
    let mut found: Option<&framebuffer::Framebuffer> = None;

    for fb in response?.iter() {
        if found == None {
            found.replace(fb);
        }
        println!(
            "framebuffer detected [addr={:?}, width={}, height={}, bpp={}]",
            fb.address, fb.width, fb.height, fb.bpp
        );
    }

    let fb = found?;
    gfx::fb::Fb::try_new(
        fb.address.cast(),
        fb.width.try_into().ok()?,
        fb.height.try_into().ok()?,
        fb.pitch.try_into().ok()?,
        fb.bpp,
        gfx::fb::Mask::new(fb.red_mask.size, fb.red_mask.shift),
        gfx::fb::Mask::new(fb.green_mask.size, fb.green_mask.shift),
        gfx::fb::Mask::new(fb.blue_mask.size, fb.blue_mask.shift),
    )
}

pub fn init() -> BootInfo<MmapIter> {
    let requests = REQUESTS.acquire();

    let fb = find_framebuffer(requests.fb.response());

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

    let response = requests
        .hhdm
        .response()
        .expect("linear physical memory not mapped by bootloader");
    let offset: usize = response.offset().try_into().unwrap();

    assert_eq!(
        offset, VADDR,
        "bad linear physical memory mapping at 0x{:x}",
        offset
    );
    println!("linear physical memory mapped at 0x{:x}", offset);

    let boot_time = requests
        .date_at_boot
        .response()
        .map(|response| response.timestamp());

    BootInfo {
        boot_time,
        mmap_iter: MmapIter::new,
    }
}
