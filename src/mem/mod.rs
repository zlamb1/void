pub mod fmt;

use core::fmt::Display;

use crate::{boot::BootInfo, println, sync::SpinLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryType {
    Free,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    Reclaimable,
    Kernel,
    BadMemory,
    Framebuffer,
}

impl Display for MemoryType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            MemoryType::Free => "free",
            MemoryType::Reserved => "reserved",
            MemoryType::AcpiReclaimable => "acpi reclaimable",
            MemoryType::AcpiNvs => "acpi nvs",
            MemoryType::Reclaimable => "reclaimable",
            MemoryType::Kernel => "kernel",
            MemoryType::BadMemory => "bad memory",
            MemoryType::Framebuffer => "framebuffer",
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MemoryRegion {
    addr: u64,
    len: u64,
    memory_type: MemoryType,
}

impl MemoryRegion {
    pub const fn new(addr: u64, len: u64, memory_type: MemoryType) -> Self {
        Self {
            addr,
            len,
            memory_type,
        }
    }

    pub fn addr(&self) -> u64 {
        self.addr
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn memory_type(&self) -> MemoryType {
        self.memory_type
    }
}

struct MemoryMap {
    count: usize,
    regions: [MemoryRegion; 64],
}

static MMAP: SpinLock<MemoryMap> = SpinLock::new(MemoryMap {
    count: 0,
    regions: [MemoryRegion::new(0, 0, MemoryType::Reserved); 64],
});

pub fn init<I: Iterator<Item = MemoryRegion>>(bi: &BootInfo<I>) {
    let mut mmap = MMAP.acquire();
    let mut max_free_addr: u64 = 0;
    let mut free_bytes: u64 = 0;
    for entry in (bi.mmap_iter)() {
        println!(
            "firmware reported memory region [addr=0x{:x}, len=0x{:x}, type={}]",
            entry.addr,
            entry.len,
            entry.memory_type(),
        );
        if entry.len() == 0 {
            continue;
        }
        if entry.memory_type() == MemoryType::Free
            || entry.memory_type() == MemoryType::Reclaimable
            || entry.memory_type() == MemoryType::AcpiReclaimable
        {
            assert!(mmap.count < mmap.regions.len(), "memory map full");
            let count = mmap.count;
            mmap.regions[count] = entry;
            mmap.count += 1;

            let end_addr = entry.addr + entry.len - 1;
            if end_addr > max_free_addr {
                max_free_addr = end_addr;
            }

            free_bytes += entry.len();
        }
    }
    println!("max free address at 0x{:x}", max_free_addr);
    println!("available free memory: {}", fmt::Memory::new(free_bytes));
}
