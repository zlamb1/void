use core::ptr::null_mut;

#[derive(Debug)]
#[repr(C)]
pub struct PerCpu {
    bp: *const PerCpu,
    cpu_id: usize,
}

impl PerCpu {
    pub const fn new(cpu_id: usize) -> Self {
        Self {
            bp: null_mut(),
            cpu_id,
        }
    }

    pub fn init(&mut self) {
        self.bp = &raw const *self;
    }

    pub fn cpu_id(&self) -> usize {
        self.cpu_id
    }
}

unsafe impl Sync for PerCpu {}
