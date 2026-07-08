use crate::mem::MemoryRegion;

pub struct BootInfo<I: Iterator<Item = MemoryRegion>> {
    pub boot_time: Option<i64>,
    pub mmap_iter: fn() -> I,
}
