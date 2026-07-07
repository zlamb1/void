use core::{cell::UnsafeCell, pin::Pin, ptr::null_mut};

use super::font::Font;
use crate::container_of;

#[derive(Debug)]
pub struct Mask {
    size: u8,
    shift: u8,
}

impl Mask {
    pub const fn new(size: u8, shift: u8) -> Self {
        Self { size, shift }
    }

    fn check(&self) -> bool {
        self.size == 8 && self.shift <= 24
    }
}

#[derive(Debug)]
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
        if addr == null_mut()
            || bpp != 32
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

#[derive(Debug)]
struct Grid {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Grid {
    fn newline(&mut self) {
        self.x = 0;
        self.y += 1;
        if self.y == self.height {
            self.y = self.height - 1;
        }
    }
}

#[derive(Debug)]
struct State<'a> {
    fb: Fb,
    font: Font<'a>,
    grid: Grid,
}

impl<'a> State<'a> {
    fn color(&self, rgb: (u8, u8, u8)) -> [u8; 4] {
        let fb = &self.fb;
        let mut color: u32 = 0;
        color |= (rgb.0 as u32) << fb.red_mask.shift;
        color |= (rgb.1 as u32) << fb.green_mask.shift;
        color |= (rgb.2 as u32) << fb.blue_mask.shift;
        color.to_ne_bytes()
    }

    fn write_str(&mut self, buf: &[u8]) {
        let fb = &self.fb;
        let font = &self.font;
        let p_bytes = fb.bpp as usize / 8;
        let fg = self.color((255u8, 255u8, 255u8));
        let bg = self.color((0u8, 0u8, 0u8));
        if self.grid.width == 0 || self.grid.height == 0 {
            return;
        }
        for &c in buf {
            match c {
                0x8 => {
                    if self.grid.x > 0 {
                        self.grid.x -= 1;
                    } else if self.grid.y > 0 {
                        self.grid.x = self.grid.width - 1;
                        self.grid.y -= 1;
                    }
                    continue;
                }
                b'\t' => {
                    continue;
                }
                b'\r' | b'\n' => {
                    self.grid.newline();
                    continue;
                }
                _ => {}
            }

            let pixel = unsafe {
                fb.addr.add(
                    fb.pitch * self.grid.y * font.height() + p_bytes * self.grid.x * font.width(),
                )
            };
            self.font.with_pixels(c as usize, |fx, fy, on| {
                let pixel = unsafe { pixel.add(fb.pitch * fy + fx * p_bytes) };
                let color = if on { fg } else { bg };
                unsafe {
                    pixel.write_volatile(color[0]);
                    pixel.add(1).write_volatile(color[1]);
                    pixel.add(2).write_volatile(color[2]);
                }
            });
            self.grid.x += 1;
            if self.grid.x == self.grid.width {
                self.grid.newline();
            }
        }
    }
}

#[derive(Debug)]
pub struct Console<'a> {
    state: UnsafeCell<State<'a>>,
    base: crate::log::Console,
}

impl<'a> Console<'a> {
    pub fn new(fb: Fb, font: Font<'a>) -> Self {
        let grid = Grid {
            x: 0,
            y: 0,
            width: fb.width / font.width(),
            height: fb.height / font.height(),
        };
        Self {
            state: UnsafeCell::new(State { fb, font, grid }),
            base: crate::log::Console::new(Self::write_str),
        }
    }

    pub fn base(&self) -> &crate::log::Console {
        &self.base
    }

    pub fn write_str(console: Pin<&crate::log::Console>, buf: &[u8]) {
        let console = unsafe { &*container_of!(&raw const *console, Self, base) };
        // SAFETY: Access is serialized by the log subsystem via CONSOLES.
        let state = unsafe { &mut *console.state.get() };
        state.write_str(buf);
    }
}

unsafe impl<'a> Send for Console<'a> {}
