use crate::mem::MemoryRegion;

mod limine;

pub struct BootInfo<I: Iterator<Item = MemoryRegion>> {
    pub boot_time: Option<i64>,
    pub mmap_iter: fn() -> I,
}

pub fn init() -> BootInfo<limine::MmapIter> {
    limine::init()
}
