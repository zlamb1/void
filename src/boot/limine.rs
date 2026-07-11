use limine::{date_at_boot, executable_cmdline, framebuffer, hhdm, memmap};

use crate::{
    boot,
    gfx::fb::{self, Mask},
    mem::{MemoryRegion, MemoryType},
    sync::SpinLock,
};

#[repr(C)]
struct Requests {
    start_marker: [u64; 4],
    fb: framebuffer::Request,
    hhdm: hhdm::Request,
    memmap: memmap::Request,
    date_at_boot: date_at_boot::Request,
    cmdline: executable_cmdline::Request,
    end_marker: [u64; 2],
}

static REQUESTS: SpinLock<Requests> = SpinLock::new(Requests {
    start_marker: limine::requests_start_marker!(),
    cmdline: executable_cmdline::Request::new(),
    fb: framebuffer::Request::new(),
    hhdm: hhdm::Request::new(),
    memmap: memmap::Request::new(),
    date_at_boot: date_at_boot::Request::new(),
    end_marker: limine::requests_end_marker!(),
});

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

pub struct FbIter {
    index: usize,
}

impl FbIter {
    pub fn new() -> Self {
        Self { index: 0 }
    }
}

impl Iterator for FbIter {
    type Item = fb::Desc;

    fn next(&mut self) -> Option<Self::Item> {
        let requests = REQUESTS.acquire();
        let framebuffers = requests.fb.response()?.framebuffers()?;

        if self.index < framebuffers.len() {
            let fb = unsafe { framebuffers[self.index].as_ref().unwrap() };
            self.index += 1;
            Some(fb::Desc {
                address: fb.address.cast(),
                width: fb.width.try_into().unwrap(),
                height: fb.height.try_into().unwrap(),
                pitch: fb.pitch.try_into().unwrap(),
                bpp: fb.bpp,
                red_mask: Mask::new(fb.red_mask.size, fb.red_mask.shift),
                green_mask: Mask::new(fb.green_mask.size, fb.green_mask.shift),
                blue_mask: Mask::new(fb.blue_mask.size, fb.blue_mask.shift),
            })
        } else {
            None
        }
    }
}

pub struct BootInfo {
    cmdline: Option<&'static [u8]>,
    hhdm: Option<usize>,
    boot_time: Option<i64>,
}

impl boot::BootInfo for BootInfo {
    fn cmdline(&self) -> Option<&'static [u8]> {
        self.cmdline
    }

    fn hhdm(&self) -> Option<usize> {
        self.hhdm
    }

    fn boot_time(&self) -> Option<i64> {
        self.boot_time
    }

    fn mmap_iter(&self) -> impl Iterator<Item = MemoryRegion> {
        MmapIter::new()
    }

    fn fb_iter(&self) -> impl Iterator<Item = fb::Desc> {
        FbIter::new()
    }
}

pub fn init() -> BootInfo {
    let requests = REQUESTS.acquire();

    let cmdline = requests
        .cmdline
        .response()
        .and_then(|response| response.cmdline())
        .map(|cmdline| cmdline.to_bytes());

    let hhdm: Option<usize> = requests
        .hhdm
        .response()
        .and_then(|response| response.offset().try_into().ok());

    let boot_time = requests
        .date_at_boot
        .response()
        .map(|response| response.timestamp());

    BootInfo {
        cmdline,
        hhdm,
        boot_time,
    }
}
