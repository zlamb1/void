use crate::sync::SpinLock;

pub struct CmdLine {
    cmdline: Option<&'static [u8]>,
}

impl CmdLine {
    pub const fn new() -> Self {
        Self { cmdline: None }
    }

    pub fn set(&mut self, cmdline: &'static [u8]) {
        self.cmdline = Some(cmdline);
    }

    pub fn get_value(&self, key: &[u8]) -> Option<Option<&[u8]>> {
        self.iter().find(|&(k, _)| key == k).map(|(_, v)| v)
    }

    pub fn get_value_array(&self, key: &[u8], delimiter: u8) -> Option<DelimiterIter<'_>> {
        self.get_value(key)?
            .map(|v| DelimiterIter::new(delimiter, v))
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self.cmdline.unwrap_or(&[]))
    }
}

pub static CMDLINE: SpinLock<CmdLine> = SpinLock::new(CmdLine::new());

pub struct Iter<'a> {
    index: usize,
    cmdline: &'a [u8],
}

impl<'a> Iter<'a> {
    pub fn new(cmdline: &'a [u8]) -> Self {
        Self { index: 0, cmdline }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a [u8], Option<&'a [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.cmdline.len();
        if self.index >= len {
            return None;
        }

        let mut k: (usize, usize) = (0, 0);
        let mut v: (usize, usize) = (0, 0);

        while self.index < len {
            let c = self.cmdline[self.index];
            self.index += 1;

            if c == b' ' {
                if k.1 > 0 {
                    break;
                }
                k = (0, 0);
                v = (0, 0);
            } else if c == b'=' && v.0 == 0 {
                v.0 = self.index;
            } else if v.0 > 0 {
                v.1 += 1;
            } else if k.1 == 0 {
                k.0 = self.index - 1;
                k.1 = 1;
            } else {
                k.1 += 1;
            }
        }

        if k.1 > 0 {
            let key = &self.cmdline[k.0..][..k.1];
            let value = if v.0 > 0 {
                Some(&self.cmdline[v.0..][..v.1])
            } else {
                None
            };
            Some((key, value))
        } else {
            None
        }
    }
}

pub struct DelimiterIter<'a> {
    index: usize,
    delimiter: u8,
    v: &'a [u8],
}

impl<'a> DelimiterIter<'a> {
    pub fn new(delimiter: u8, v: &'a [u8]) -> Self {
        Self {
            index: 0,
            delimiter,
            v,
        }
    }
}

impl<'a> Iterator for DelimiterIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.v.len();
        let mut start = self.index;
        let mut run: usize = 0;

        while self.index < len {
            let c = self.v[self.index];
            self.index += 1;

            if c == self.delimiter {
                if run > 0 {
                    break;
                }
                start = self.index;
                run = 0;
            } else {
                run += 1;
            }
        }

        if run > 0 {
            Some(&self.v[start..][..run])
        } else {
            None
        }
    }
}
