use core::sync::atomic::{AtomicU32, Ordering, fence};

use crate::arch;

pub struct SpinLock {
    owner: AtomicU32,
    next: AtomicU32,
}

impl SpinLock {
    /// # Safety:
    /// Each acquire must be paired with one release.
    /// Cannot be called again until release is called.
    pub unsafe fn acquire(&self) {
        let ticket = self.next.fetch_add(1, Ordering::Relaxed);
        while ticket != self.owner.load(Ordering::Relaxed) {
            arch::spin_hint();
        }
        // Safety: Must synchronize with the release on owner.
        // Atomic - Fence
        fence(Ordering::Acquire);
    }

    /// # Safety:
    /// See [`SpinLock::acquire`].
    pub unsafe fn release(&self) {
        self.owner.fetch_add(1, Ordering::Release);
    }

    /// # Safety:
    /// If successful, has the same requirements as [`SpinLock::acquire`].
    pub unsafe fn try_acquire(&self) -> bool {
        // Safety: Must synchronize with the release on owner.
        let owner = self.owner.load(Ordering::Acquire);
        let ticket = self.next.load(Ordering::Relaxed);
        if ticket != owner {
            return false;
        }
        // Safety: Must prevent local reordering within core/compiler before the ticket is claimed.
        if self
            .next
            .compare_exchange(
                ticket,
                ticket.wrapping_add(1),
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_err()
        {
            return false;
        }
        true
    }
}

unsafe impl Send for SpinLock {}
unsafe impl Sync for SpinLock {}
