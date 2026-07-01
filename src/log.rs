use core::fmt::{self, Write};
use core::pin::Pin;

use crate::container_of;
use crate::lending::Iterator;
use crate::list::{self, ListOwned};
use crate::sync::{self, SpinLock};

pub struct Console {
    write_str: for<'a> fn(state: Pin<&'a mut Console>, s: &[u8]),
    node: list::Links,
}

unsafe impl ListOwned for Console {}

struct Adapter;

unsafe impl list::Adapter<Console> for Adapter {
    fn from_links(links: *const list::Links) -> *const Console {
        container_of!(links, Console, node)
    }

    fn to_links(obj: *const Console) -> *const list::Links {
        unsafe { &raw const (*obj).node }
    }
}

struct Log {
    _head: usize,
    tail: usize,
    written: usize,
    buf: [u8; Self::CAP],
}

impl Log {
    const CAP: usize = 8192;
    const _CAP_IS_POWER_OF_2: () = assert!(Self::CAP.is_power_of_two());

    pub const fn new() -> Self {
        Self {
            _head: 0,
            tail: 0,
            written: 0,
            buf: [0; _],
        }
    }
}

impl Write for Log {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        self.written = core::cmp::min(Self::CAP, bytes.len());
        for &c in &bytes[bytes.len() - self.written..] {
            self.buf[self.tail as usize] = c;
            self.tail = (self.tail + 1) & (Self::CAP - 1);
        }
        Ok(())
    }
}

static LOG: SpinLock<Log> = SpinLock::new(Log::new());
static CONSOLES: sync::pin::SpinLock<list::List<Console, Adapter>> =
    sync::pin::SpinLock::new(list::List::new());

pub fn init() {
    CONSOLES.acquire().as_pin().init();
}

pub fn register(console: Pin<&Console>) {
    CONSOLES.with(|consoles| consoles.add(console));
}

pub fn write(args: fmt::Arguments) {
    let mut log = LOG.acquire();
    let read = log.tail;
    let _ = log.write_fmt(args);
    if log.written == 0 {
        return;
    }
    CONSOLES.with_mut(|consoles| {
        let mut iter = consoles.iter_mut();
        while let Some(mut console) = iter.next() {
            let write_str = console.write_str;
            let end = read + log.written;
            if end >= Log::CAP {
                (write_str)(console.as_mut(), &log.buf[read..]);
                (write_str)(console, &log.buf[..log.tail]);
            } else {
                (write_str)(console, &log.buf[read..log.tail]);
            }
        }
    });
    log.written = 0;
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        let args = core::format_args!($($arg)*);
        crate::log::write(args);
    };
}

pub(crate) use print;
