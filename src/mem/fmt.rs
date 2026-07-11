use core::fmt::Display;

pub struct Memory {
    bytes: usize,
}

impl Memory {
    pub const fn new(bytes: usize) -> Self {
        Self { bytes }
    }
}

impl Display for Memory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.bytes < 1024 {
            f.write_fmt(format_args!("{} B", self.bytes))
        } else if self.bytes < 1024 * 1024 {
            let kb = self.bytes / 1024;
            f.write_fmt(format_args!("{} KiB", kb))
        } else if self.bytes < 1024 * 1024 * 1024 {
            let mb = self.bytes / 1024 / 1024;
            f.write_fmt(format_args!("{} MiB", mb))
        } else {
            let gb = self.bytes / 1024 / 1024 / 1024;
            f.write_fmt(format_args!("{} GiB", gb))
        }
    }
}
