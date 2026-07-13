pub mod terminus16_8;

use core::cmp::min;

#[derive(Clone, Copy, Debug)]
pub struct Font<'a> {
    width: usize,
    height: usize,
    count: usize,
    data: &'a [u8],
}

impl<'a> Font<'a> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn glyph_count(&self) -> usize {
        self.count
    }

    pub fn data(&self) -> &[u8] {
        self.data
    }

    pub fn with_pixels(&self, glyph: usize, mut for_each: impl FnMut(usize, usize, bool)) {
        assert!(glyph < self.count);
        let pitch = (self.width + 7) / 8;
        let len = pitch * self.height;
        let bytes = &self.data[len * glyph..][..len];
        for fy in 0..self.height {
            let mut index = pitch * fy;
            let mut fx: usize = 0;
            while fx < self.width {
                let mut v = bytes[index];
                for fx in fx..min(self.width, fx + 8) {
                    for_each(fx, fy, v & 0x80 == 0x80);
                    v <<= 1;
                }
                fx += 8;
                index += 1;
            }
        }
    }
}
