use crate::{arch, mp};

#[cfg(debug_assertions)]
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering, fence};

#[derive(Debug, Default)]
pub struct SpinLock {
    owner: AtomicU32,
    next: AtomicU32,
    #[cfg(debug_assertions)]
    cpu_id: AtomicUsize,
}

impl SpinLock {
    #[cfg(debug_assertions)]
    const NO_OWNER: usize = usize::MAX;

    #[cfg(debug_assertions)]
    fn deadlock_check(&self) {
        let cpu_id = self.cpu_id.load(Ordering::Relaxed);
        debug_assert_ne!(cpu_id, mp::cpu_id(), "deadlock detected");
    }

    #[cfg(debug_assertions)]
    fn unlock_check(&self) {
        let cpu_id = self.cpu_id.load(Ordering::Relaxed);
        debug_assert_eq!(cpu_id, mp::cpu_id(), "wrong cpu unlocked");
    }

    #[inline]
    fn do_acquire(&self) {
        // Safety: Must synchronize with the release on owner.
        // Atomic - Fence
        fence(Ordering::Acquire);
        #[cfg(debug_assertions)]
        self.cpu_id.store(mp::cpu_id(), Ordering::Relaxed);
    }

    /// # Safety:
    /// Each acquire must be paired with one release.
    /// Cannot be called again until release is called.
    pub unsafe fn acquire(&self) {
        #[cfg(debug_assertions)]
        self.deadlock_check();

        let ticket = self.next.fetch_add(1, Ordering::Relaxed);

        while ticket != self.owner.load(Ordering::Relaxed) {
            arch::spin_hint();
        }

        self.do_acquire();
    }

    pub const fn new() -> Self {
        Self {
            owner: AtomicU32::new(0),
            next: AtomicU32::new(0),
            #[cfg(debug_assertions)]
            cpu_id: AtomicUsize::new(Self::NO_OWNER),
        }
    }

    /// # Safety:
    /// See [`SpinLock::acquire`].
    pub unsafe fn release(&self) {
        #[cfg(debug_assertions)]
        self.unlock_check();

        #[cfg(debug_assertions)]
        self.cpu_id.store(Self::NO_OWNER, Ordering::Relaxed);

        self.owner.fetch_add(1, Ordering::Release);
    }

    /// # Safety:
    /// If successful, has the same invariants as [`SpinLock::acquire`].
    pub unsafe fn try_acquire(&self) -> bool {
        #[cfg(debug_assertions)]
        self.deadlock_check();

        let owner = self.owner.load(Ordering::Relaxed);
        let ticket = self.next.load(Ordering::Relaxed);

        if ticket != owner {
            return false;
        }

        if self
            .next
            .compare_exchange(
                ticket,
                ticket.wrapping_add(1),
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_err()
        {
            return false;
        }

        self.do_acquire();
        true
    }
}

unsafe impl Send for SpinLock {}
unsafe impl Sync for SpinLock {}
