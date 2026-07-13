use core::{
    alloc::{GlobalAlloc, Layout},
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    mem::forget,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::mem::alloc::PageAlloc;

#[repr(transparent)]
pub struct Box<T> {
    non_null: NonNull<T>,
    _data: PhantomData<T>,
}

impl<T> Box<T> {
    pub fn new(x: T) -> Box<T> {
        Self::try_new(x).unwrap()
    }

    fn layout() -> Layout {
        Layout::new::<T>()
    }

    pub fn try_new(x: T) -> Option<Box<T>> {
        let layout = Self::layout();

        let non_null = if layout.size() > 0 {
            let alloc = PageAlloc;
            unsafe {
                let ptr: *mut T = alloc.alloc(layout).cast();
                if ptr.is_null() {
                    return None;
                }
                NonNull::new_unchecked(ptr)
            }
        } else {
            NonNull::dangling()
        };

        unsafe {
            non_null.write(x);
        }

        Some(Self {
            non_null,
            _data: PhantomData,
        })
    }

    pub fn leak<'a>(b: Box<T>) -> &'a mut T {
        let mut non_null = b.non_null;
        forget(b);
        unsafe { non_null.as_mut() }
    }

    pub fn into_raw(b: Box<T>) -> *mut T {
        let non_null = b.non_null;
        forget(b);
        non_null.as_ptr()
    }

    pub fn into_non_null(b: Box<T>) -> NonNull<T> {
        let non_null = b.non_null;
        forget(b);
        non_null
    }

    pub fn as_ptr(b: &Box<T>) -> *const T {
        b.non_null.as_ptr().cast_const()
    }

    pub fn as_mut_ptr(b: &Box<T>) -> *mut T {
        b.non_null.as_ptr()
    }
}

impl<T: Clone> Clone for Box<T> {
    fn clone(&self) -> Self {
        Self::new(self.deref().clone())
    }
}

impl<T: Debug> Debug for Box<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.deref(), f)
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.non_null.as_ref() }
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.non_null.as_mut() }
    }
}

impl<T: Display> Display for Box<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.deref(), f)
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        let layout = Self::layout();

        unsafe {
            self.non_null.drop_in_place();
            if layout.size() > 0 {
                let alloc = PageAlloc;
                alloc.dealloc(self.non_null.as_ptr().cast(), layout);
            }
        }
    }
}

impl<T: Eq> Eq for Box<T> {}

impl<T: Hash> Hash for Box<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(self.deref(), state)
    }
}

impl<T: Ord> Ord for Box<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(self.deref(), other.deref())
    }
}

impl<T: PartialEq> PartialEq for Box<T> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.deref(), other.deref())
    }
}

impl<T: PartialOrd> PartialOrd for Box<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(self.deref(), other.deref())
    }
}

unsafe impl<T: Send> Send for Box<T> {}
unsafe impl<T: Sync> Sync for Box<T> {}
