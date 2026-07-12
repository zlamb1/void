use core::{bstr::ByteStr, mem::MaybeUninit, pin::Pin};

use crate::{
    boot::cmdline::CMDLINE,
    date,
    gfx::{fb, font},
    log,
    mem::{MemoryRegion, VADDR},
    println,
    sync::SpinLock,
};

pub mod cmdline;

mod limine;

pub trait BootInfo {
    fn cmdline(&self) -> Option<&'static [u8]>;
    fn hhdm(&self) -> Option<usize>;
    fn boot_time(&self) -> Option<i64>;
    fn mmap_iter(&self) -> impl Iterator<Item = MemoryRegion>;
    fn fb_iter(&self) -> impl Iterator<Item = fb::Desc>;
}

struct Consoles {
    count: usize,
    array: [MaybeUninit<fb::Console<'static>>; 8],
}

static CONSOLES: SpinLock<Consoles> = SpinLock::new(Consoles {
    count: 0,
    array: [const { MaybeUninit::uninit() }; _],
});

fn log_boot_time(bt: &Option<i64>) {
    if let &Some(bt) = bt {
        let bt = date::Date::from_utime(bt);
        log::println!(
            "boot time: {} {} {} {:02}:{:02}:{:02} {}",
            bt.day_of_month(),
            bt.month(),
            bt.year(),
            bt.hour_12(),
            bt.minute(),
            bt.second(),
            bt.period(),
        );
    }
}

fn log_framebuffer(number: usize, fb: &fb::Desc) {
    println!(
        "fb{} detected [addr={:?}, width={}, height={}, bpp={}]",
        number, fb.address, fb.width, fb.height, fb.bpp
    );
}

fn try_create_fb(fb: &fb::Desc) -> Option<fb::Fb> {
    fb::Fb::try_new(
        fb.address.cast(),
        fb.width.try_into().ok()?,
        fb.height.try_into().ok()?,
        fb.pitch.try_into().ok()?,
        fb.bpp,
        fb::Mask::new(fb.red_mask.size(), fb.red_mask.shift()),
        fb::Mask::new(fb.green_mask.size(), fb.green_mask.shift()),
        fb::Mask::new(fb.blue_mask.size(), fb.blue_mask.shift()),
    )
}

fn find_console(boot_info: &impl BootInfo) {
    let cmdline = CMDLINE.acquire();
    let mut mask: u8 = 0;

    if let Some(values) = cmdline.get_value_array(b"console", b',') {
        for value in values {
            if value == b"fb*" {
                mask = u8::MAX;
            } else if value.len() == 3 && &value[..2] == b"fb" {
                let digit = value[2];
                if digit >= b'0' && digit <= b'7' {
                    mask |= 1u8 << (digit - b'0');
                }
            } else {
                println!("warn: unknown requested console '{}'", ByteStr::new(value));
            }
        }
    }

    let mut number: usize = 0;

    if mask != 0 {
        for fb in boot_info.fb_iter() {
            log_framebuffer(number, &fb);

            if mask & 1 == 1 {
                if let Some(fb) = try_create_fb(&fb) {
                    let mut consoles = CONSOLES.acquire();
                    let count = consoles.count;
                    let console = fb::Console::new(fb, font::terminus16_8::FONT);
                    let console = consoles.array[count].write(console);
                    let base = unsafe { Pin::new_unchecked(console.base()) };
                    crate::log::register(base);
                    consoles.count += 1;
                }
            }
            mask >>= 1;

            number += 1;
        }
    } else {
        let mut found = false;
        for fb in boot_info.fb_iter() {
            log_framebuffer(number, &fb);

            if !found {
                if let Some(fb) = try_create_fb(&fb) {
                    let mut consoles = CONSOLES.acquire();
                    let console = fb::Console::new(fb, font::terminus16_8::FONT);
                    let console = consoles.array[0].write(console);
                    let base = unsafe { Pin::new_unchecked(console.base()) };
                    crate::log::register(base);
                    found = true;
                }
            }

            number += 1;
        }
    }
}

pub fn init() -> impl BootInfo {
    log::println!("booting...");

    let bi = limine::init();

    bi.cmdline().inspect(|&cmdline| {
        println!("kernel command line: {}", ByteStr::new(cmdline));
        CMDLINE.acquire().set(cmdline);
    });

    find_console(&bi);

    log_boot_time(&bi.boot_time());

    let hhdm = bi
        .hhdm()
        .expect("linear physical memory not mapped by bootloader");

    assert_eq!(
        hhdm, VADDR,
        "bad linear physical memory mapping at 0x{:x}",
        hhdm
    );
    println!("linear physical memory mapped at 0x{:x}", hhdm);

    bi
}
