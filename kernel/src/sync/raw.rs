use crate::arch;

pub type SpinLock = crate::sync::ticket::SpinLock;

pub struct LockGuard<'a> {
    flags: usize,
    lock: &'a SpinLock,
}

impl<'a> LockGuard<'a> {
    pub fn with_lock(lock: &'a SpinLock) -> Self {
        let flags = arch::irq_save_and_disable();
        unsafe {
            lock.acquire();
        }
        Self { flags, lock }
    }
}

impl<'a> Drop for LockGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.lock.release();
        }
        arch::irq_restore(self.flags);
    }
}

impl SpinLock {
    pub fn guard(&self) -> LockGuard<'_> {
        LockGuard::with_lock(self)
    }
}
