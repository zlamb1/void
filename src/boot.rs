use crate::mem::MemoryRegion;

pub struct BootInfo<I: Iterator<Item = MemoryRegion>> {
    pub mmap_iter: fn() -> I,
}
