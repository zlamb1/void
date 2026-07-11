use crate::list::{Adapter, Links, List};
use crate::sync::{self, SpinLock};
use core::fmt::{self, Write};
use core::mem::offset_of;
use core::pin::Pin;

pub type Clear = fn(console: Pin<&Console>);
pub type WriteStr = fn(console: Pin<&Console>, buf: &[u8]);

#[derive(Debug)]
pub struct Console {
    clear: Clear,
    write_str: WriteStr,
    node: Links,
}

impl Console {
    pub fn new(clear: Clear, write_str: WriteStr) -> Self {
        Self {
            clear,
            write_str,
            node: Links::new(),
        }
    }

    fn write(self: Pin<&Console>, head: usize, tail: usize, buf: &[u8]) {
        let head = Log::mask(head);
        let tail = Log::mask(tail);
        if head < tail {
            (self.write_str)(self, &buf[head..tail]);
        } else {
            (self.write_str)(self, &buf[head..]);
            (self.write_str)(self, &buf[..tail]);
        }
    }
}

struct ConsoleAdapter;

unsafe impl Adapter<Console> for ConsoleAdapter {
    unsafe fn from_links(links: *const crate::list::Links) -> *const Console {
        unsafe {
            let p: *const u8 = links.cast();
            p.sub(offset_of!(Console, node)).cast()
        }
    }

    unsafe fn to_links(obj: *const Console) -> *const crate::list::Links {
        unsafe { &raw const (*obj).node }
    }
}

struct Log {
    head: usize,
    tail: usize,
    pending: usize,
    buf: [u8; Self::CAP],
}

impl Log {
    const CAP: usize = 8192;
    const _CAP_IS_POWER_OF_2: () = assert!(Self::CAP.is_power_of_two());

    pub const fn new() -> Self {
        Self {
            head: 0,
            tail: 0,
            pending: 0,
            buf: [0; _],
        }
    }

    fn mask(n: usize) -> usize {
        n & (Self::CAP - 1)
    }
}

impl Write for Log {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        for &c in bytes {
            if self.tail.wrapping_sub(self.head) == Self::CAP {
                self.head = self.head.wrapping_add(1);
            }
            self.buf[Log::mask(self.tail)] = c;
            self.tail = self.tail.wrapping_add(1);
        }
        self.pending += bytes.len();
        Ok(())
    }
}

static LOG: SpinLock<Log> = SpinLock::new(Log::new());
static CONSOLES: sync::pin::SpinLock<List<Console, ConsoleAdapter>> =
    sync::pin::SpinLock::new(List::new());

pub fn init() {
    CONSOLES.with_mut(|consoles| consoles.init());
}

pub fn register(console: Pin<&Console>) {
    (console.clear)(console);
    let log = LOG.acquire();
    CONSOLES.with_mut(|consoles| {
        if log.head != log.tail {
            console.write(log.head, log.tail, &log.buf);
        }
        unsafe {
            consoles.push_front(console);
        }
    });
}

pub fn clear() {
    CONSOLES.with(|consoles| {
        for console in consoles {
            (console.clear)(console);
        }
    });
}

pub fn write(args: fmt::Arguments, newline: bool) {
    let mut log = LOG.acquire();
    log.pending = 0;
    let mut head = log.tail;
    let _ = log.write_fmt(args);
    if newline {
        let _ = log.write_str("\n");
    }
    if log.pending == 0 {
        return;
    }
    if log.pending > Log::CAP {
        head = log.head;
    }
    let tail = log.tail;
    CONSOLES.with(|consoles| {
        for console in consoles {
            console.write(head, tail, &log.buf);
        }
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let args = core::format_args!($($arg)*);
        $crate::log::write(args, false);
    }};
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        let args = core::format_args!($($arg)*);
        $crate::log::write(args, true);
    }};
}

#[allow(unused_imports)]
pub(crate) use print;
pub(crate) use println;
