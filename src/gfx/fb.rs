use core::cmp::min;
use core::{cell::UnsafeCell, pin::Pin, ptr::null_mut};

use super::font::Font;
use crate::arch;
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

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn bpp(&self) -> u16 {
        self.bpp
    }
}

#[derive(Debug)]
struct Grid {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

type Color = (u8, u8, u8);

#[derive(Debug)]
struct State<'a> {
    fb: Fb,
    font: Font<'a>,
    grid: Grid,
    fg: Color,
    bg: Color,
}

impl<'a> State<'a> {
    fn reverse(&mut self) {
        if self.grid.x > 0 {
            self.grid.x -= 1;
        } else if self.grid.y > 0 {
            self.grid.x = self.grid.width - 1;
            self.grid.y -= 1;
        }
    }

    fn advance(&mut self) {
        self.grid.x += 1;
        if self.grid.x == self.grid.width {
            self.newline();
        }
    }

    fn newline(&mut self) {
        self.grid.x = 0;
        self.grid.y += 1;
        if self.grid.y == self.grid.height {
            self.grid.y = self.grid.height - 1;
            self.scroll();
        }
    }

    fn color_word(&self, color: Color) -> u32 {
        let fb = &self.fb;
        let mut x: u32 = 0;
        x |= (color.0 as u32) << fb.red_mask.shift;
        x |= (color.1 as u32) << fb.green_mask.shift;
        x |= (color.2 as u32) << fb.blue_mask.shift;
        x
    }

    fn color_bytes(&self, color: Color) -> [u8; 4] {
        self.color_word(color).to_ne_bytes()
    }

    fn clear(&mut self) {
        if self.grid.width == 0 || self.grid.height == 0 {
            return;
        }

        self.grid.x = 0;
        self.grid.y = 0;

        let bg = self.color_word(self.bg);
        let words = self.font.width() * self.grid.width;

        for y in 0..self.font.height() * self.grid.height {
            let pixels: *mut u32 = unsafe { self.fb.addr.add(self.fb.pitch * y).cast() };
            for x in 0..words {
                unsafe {
                    pixels.add(x).write_volatile(bg);
                }
            }
        }

        self.sync();
    }

    fn scroll(&mut self) {
        let pitch = self.fb.pitch;
        let pixel_bytes = self.fb.bpp as usize / 8;
        let bg = self.color_bytes(self.bg);

        let mut dst = self.fb.addr;
        let mut src = unsafe { dst.add(self.font.height() * pitch) };

        for _ in 0..self.fb.height - self.font.height() {
            for x in 0..self.fb.width * pixel_bytes {
                unsafe {
                    dst.add(x).write_volatile(src.add(x).read_volatile());
                }
            }

            unsafe {
                dst = dst.add(pitch);
                src = src.add(pitch);
            }
        }

        for _ in 0..self.font.height() {
            let mut pixel = dst;

            for _ in 0..self.fb.width {
                for i in 0..pixel_bytes {
                    unsafe {
                        pixel.add(i).write_volatile(bg[i]);
                    }
                }
                unsafe {
                    pixel = pixel.add(pixel_bytes);
                }
            }

            unsafe {
                dst = dst.add(pitch);
            }
        }
    }

    fn write_str(&mut self, buf: &[u8]) {
        if self.grid.width == 0 || self.grid.height == 0 {
            return;
        }

        let len = buf.len();

        let p_bytes = self.fb.bpp as usize / 8;
        let fg = self.color_bytes(self.fg);
        let bg = self.color_bytes(self.bg);
        let glyph_stride = self.font.width() * p_bytes;

        let mut index: usize = 0;

        'outer: while index < len {
            let run = min(self.grid.width - self.grid.x, len - index);
            let mut pixel = unsafe {
                self.fb.addr.add(
                    self.grid.y * self.fb.pitch * self.font.height() + self.grid.x * glyph_stride,
                )
            };
            for _ in 0..run {
                let c = buf[index];
                index += 1;
                match c {
                    0x8 => {
                        self.reverse();
                        continue 'outer;
                    }
                    b'\r' => {
                        self.grid.x = 0;
                        continue 'outer;
                    }
                    b'\n' => {
                        self.newline();
                        continue 'outer;
                    }
                    _ => {}
                }
                self.font.with_pixels(c as usize, |fx, fy, on| {
                    let pixel = unsafe { pixel.add(self.fb.pitch * fy + fx * p_bytes) };
                    let color = if on { fg } else { bg };
                    unsafe {
                        pixel.write_volatile(color[0]);
                        pixel.add(1).write_volatile(color[1]);
                        pixel.add(2).write_volatile(color[2]);
                    }
                });
                pixel = unsafe { pixel.add(glyph_stride) };
                self.advance();
            }
        }
        self.sync();
    }

    fn sync(&self) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        // NOTE: Write combining memory needs flushing.
        arch::sfence();
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
            state: UnsafeCell::new(State {
                fb,
                font,
                grid,
                fg: (255, 255, 255),
                bg: (0, 0, 0),
            }),
            base: crate::log::Console::new(Self::clear, Self::write_str),
        }
    }

    pub fn base(&self) -> &crate::log::Console {
        &self.base
    }

    pub fn clear(console: Pin<&crate::log::Console>) {
        let console = unsafe { &*container_of!(&raw const *console, Self, base) };
        // SAFETY: Access is serialized by the log subsystem via CONSOLES.
        let state = unsafe { &mut *console.state.get() };
        state.clear();
    }

    pub fn write_str(console: Pin<&crate::log::Console>, buf: &[u8]) {
        let console = unsafe { &*container_of!(&raw const *console, Self, base) };
        // SAFETY: Access is serialized by the log subsystem via CONSOLES.
        let state = unsafe { &mut *console.state.get() };
        state.write_str(buf);
    }
}

unsafe impl<'a> Send for Console<'a> {}
