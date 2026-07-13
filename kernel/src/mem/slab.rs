use core::{
    alloc::{GlobalAlloc, Layout},
    cell::{Cell, UnsafeCell},
};

use crate::mem::alloc::PageAlloc;

use super::page;

pub struct Slab {
    layout: Layout,
    stride: usize,
    capacity: usize,
    available: Cell<usize>,
    page: *mut (),
    free: UnsafeCell<[u64; 2]>,
}

impl Slab {
    pub fn try_new(layout: Layout) -> Option<Slab> {
        assert!(layout.size() > 0);

        let stride = layout.size().next_multiple_of(layout.align());
        let capacity = page::SIZE / stride;

        if capacity > 128 {
            return None;
        }

        let page_alloc = PageAlloc;
        let page = unsafe { page_alloc.alloc(page::LAYOUT) };

        if page.is_null() {
            return None;
        }

        let mut free: [u64; 2] = [0; 2];
        for i in 0..capacity {
            let index = i / 64;
            let bit = i % 64;
            free[index] |= 1u64 << bit;
        }
        Some(Slab {
            layout,
            stride,
            capacity,
            available: Cell::new(capacity),
            page: page.cast(),
            free: UnsafeCell::new(free),
        })
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn stride(&self) -> usize {
        self.stride
    }

    pub fn available(&self) -> usize {
        self.available.get()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub unsafe fn alloc(&self) -> Option<*mut ()> {
        let free = unsafe { &mut *self.free.get() };

        let index = if free[0] != 0 {
            let index = free[0].trailing_zeros();
            free[0] &= !(1u64 << index);
            index
        } else if free[1] != 0 {
            let index = free[1].trailing_zeros();
            free[1] &= !(1u64 << index);
            index
        } else {
            return None;
        };

        let available = self.available();
        assert_ne!(available, 0, "slab: attempted alloc while empty");
        self.available.set(available - 1);
        let ptr = unsafe { self.page.add(self.stride * index as usize) };

        Some(ptr)
    }

    pub unsafe fn dealloc(&self, ptr: *mut ()) {
        assert!(self.page <= ptr, "slab: invalid free detected");

        let offset = ptr.addr() - self.page.addr();

        assert!(
            offset.is_multiple_of(self.stride),
            "slab: invalid free detected"
        );

        let obj_index = offset / self.stride;

        assert!(obj_index < self.capacity, "slab: invalid free detected");

        let free = unsafe { &mut *self.free.get() };
        let index = obj_index / 64;
        let bit = obj_index % 64;

        assert_eq!(free[index] & (1u64 << bit), 0, "slab: double free detected");
        free[index] |= 1u64 << bit;

        let available = self.available();
        assert!(
            available < self.capacity,
            "slab: attempted dealloc while full"
        );
        self.available.set(available + 1);
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        assert!(self.available() == self.capacity);

        let page_alloc = PageAlloc;
        unsafe {
            page_alloc.dealloc(self.page.cast(), page::LAYOUT);
        }
    }
}
