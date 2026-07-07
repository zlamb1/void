use core::{cell::UnsafeCell, pin::Pin};

use crate::container_of;

pub struct Mask {
    size: u8,
    shift: u8,
}

impl Mask {
    pub const fn new(size: u8, shift: u8) -> Self {
        Self { size, shift }
    }

    fn check(&self) -> bool {
        self.size == 8 && (self.shift == 0 || self.shift == 8 || self.shift == 16)
    }
}

pub struct Fb {
    addr: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    bpp: u16,
    red_mask: Mask,
    green_mask: Mask,
    blue_mask: Mask,
}

impl Fb {
    pub fn try_new(
        addr: *mut u8,
        width: usize,
        height: usize,
        pitch: usize,
        bpp: u16,
        red_mask: Mask,
        green_mask: Mask,
        blue_mask: Mask,
    ) -> Option<Fb> {
        if bpp != 32
            || !red_mask.check()
            || !green_mask.check()
            || !blue_mask.check()
            || width.checked_mul(bpp as usize / 8)? > pitch
        {
            return None;
        }
        Some(Fb {
            addr,
            width,
            height,
            pitch,
            bpp,
            red_mask,
            green_mask,
            blue_mask,
        })
    }
}

struct State {
    fb: Fb,
}

impl State {
    fn write_str(&mut self, _: &[u8]) {}
}

pub struct Console {
    state: UnsafeCell<State>,
    _super: crate::log::Console,
}

impl Console {
    pub fn new(fb: Fb) -> Self {
        Self {
            state: UnsafeCell::new(State { fb }),
            _super: crate::log::Console::new(Self::write_str),
        }
    }

    pub fn write_str(console: Pin<&crate::log::Console>, buf: &[u8]) {
        let console = unsafe { &*container_of!(&raw const *console, Self, _super) };
        let state = unsafe { &mut *console.state.get() };
        state.write_str(buf);
    }
}
