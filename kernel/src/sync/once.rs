use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Once<T> {
    run: AtomicBool,
    context: UnsafeCell<T>,
}

impl<T> Once<T> {
    pub const fn new(x: T) -> Self {
        Self {
            run: AtomicBool::new(false),
            context: UnsafeCell::new(x),
        }
    }

    pub fn call_once<'a, R>(&'a self, f: impl FnOnce(&'a mut T) -> R) -> Option<R> {
        if self
            .run
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            Some(f(unsafe { &mut *self.context.get() }))
        } else {
            None
        }
    }

    pub fn is_completed(&self) -> bool {
        self.run.load(Ordering::Relaxed)
    }
}

unsafe impl<T: Send> Send for Once<T> {}
unsafe impl<T: Send> Sync for Once<T> {}
