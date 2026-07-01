use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::arch;

pub mod raw;
pub mod ticket;

#[derive(Debug, Default)]
pub struct SpinLock<T> {
    raw: raw::SpinLock,
    value: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            raw: raw::SpinLock::new(),
            value: UnsafeCell::new(value),
        }
    }

    pub fn acquire(&self) -> LockGuard<'_, T> {
        LockGuard::with_lock(self)
    }

    pub fn with<R>(&self, function: impl FnOnce(&T) -> R) -> R {
        let guard = LockGuard::with_lock(self);
        function(&*guard)
    }

    pub fn with_mut<R>(&self, function: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = LockGuard::with_lock(self);
        function(&mut *guard)
    }

    pub const fn as_raw(&self) -> &raw::SpinLock {
        &self.raw
    }
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

pub struct LockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    flags: usize,
    _not_send: PhantomData<*mut ()>,
}

impl<'a, T> LockGuard<'a, T> {
    pub fn with_lock(lock: &'a SpinLock<T>) -> Self {
        let flags = arch::irq_save_and_disable();
        unsafe {
            lock.raw.acquire();
        }
        Self {
            lock,
            flags,
            _not_send: PhantomData,
        }
    }
}

impl<T> Deref for LockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for LockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.raw.release();
        }
        arch::irq_restore(self.flags);
    }
}

pub mod pin {
    use core::pin::Pin;

    pub struct SpinLock<T> {
        inner: super::SpinLock<T>,
    }

    impl<T> SpinLock<T> {
        pub const fn new(value: T) -> Self {
            Self {
                inner: super::SpinLock::new(value),
            }
        }

        pub fn acquire(&'static self) -> LockGuard<'static, T> {
            LockGuard {
                inner: self.inner.acquire(),
            }
        }

        pub fn with<R>(&'static self, function: impl FnOnce(Pin<&T>) -> R) -> R {
            let guard = self.acquire();
            function(guard.as_pin())
        }

        pub fn with_mut<R>(&'static self, function: impl FnOnce(Pin<&mut T>) -> R) -> R {
            let mut guard = self.acquire();
            function(guard.as_pin_mut())
        }
    }

    pub struct LockGuard<'a, T> {
        inner: super::LockGuard<'a, T>,
    }

    impl<'a, T> LockGuard<'a, T> {
        pub fn as_pin(&self) -> Pin<&T> {
            unsafe { Pin::new_unchecked(&*self.inner) }
        }

        pub fn as_pin_mut(&mut self) -> Pin<&mut T> {
            unsafe { Pin::new_unchecked(&mut *self.inner) }
        }
    }
}
