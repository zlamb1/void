use crate::{boot::BootInfo, println};

#[derive(Clone, Copy, Debug)]
pub enum MemoryType {
    Free,
    Reserved,
}

impl Into<&'static str> for MemoryType {
    fn into(self) -> &'static str {
        match self {
            MemoryType::Free => "free",
            MemoryType::Reserved => "reserved",
        }
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

pub fn init<I: Iterator<Item = MemoryRegion>>(bi: &BootInfo<I>) {
    for entry in (bi.mmap_iter)() {
        let memory_type: &'static str = entry.memory_type().into();
        println!(
            "firmware reported memory region [addr=0x{:x}, len=0x{:x}, type={}]",
            entry.addr, entry.len, memory_type,
        );
    }
}
