use core::{
    cell::Cell,
    ptr::null_mut,
    sync::atomic::{AtomicU8, Ordering},
};

pub const SIZE: usize = 4096;
pub const LOG2_SIZE: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Type {
    Frame = 0,
    Slab = 1,
}

#[derive(Debug)]
pub struct Flags<'a> {
    flags: &'a AtomicU8,
}

impl<'a> Flags<'a> {
    pub fn new(flags: &'a AtomicU8) -> Self {
        Self { flags }
    }

    fn flags(&self) -> u8 {
        self.flags.load(Ordering::Acquire)
    }

    pub fn page_type(&self) -> Type {
        unsafe { core::mem::transmute(self.flags() & 1) }
    }

    fn swap(&self, compute: impl Fn(u8) -> u8) {
        let mut old_flags = self.flags();
        loop {
            let new_flags = compute(old_flags);
            if let Err(current) = self.flags.compare_exchange(
                old_flags,
                new_flags,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                old_flags = current;
            } else {
                break;
            }
        }
    }

    pub fn set_page_type(&mut self, page_type: Type) {
        let page_type = page_type as u8;
        self.swap(|flags| (flags & !1u8) | page_type);
    }
}

#[derive(Debug)]
pub struct Page {
    flags: AtomicU8,
    /// Used by owning subsystem.
    /// Ex:
    /// PMM uses this for the freelist.
    /// Slab uses this to point back at the owning slab.
    pointer: Cell<*mut ()>,
}

impl Page {
    pub const fn new() -> Self {
        Self {
            flags: AtomicU8::new(0),
            pointer: Cell::new(null_mut()),
        }
    }

    pub fn flags(&self) -> Flags<'_> {
        Flags::new(&self.flags)
    }

    /// ## SAFETY:
    /// Access should be serialized by the owning subsystem.
    /// A happens-before edge must occur between set_pointer
    /// and any call to pointer.
    pub unsafe fn pointer(&self) -> *mut () {
        self.pointer.get()
    }

    /// SAFETY:
    /// See [`Page::pointer`].
    pub unsafe fn set_pointer(&self, new_pointer: *mut ()) {
        self.pointer.set(new_pointer);
    }
}

unsafe impl Send for Page {}
unsafe impl Sync for Page {}
