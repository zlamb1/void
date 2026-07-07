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
