use core::{cell::OnceCell, pin::Pin};

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

static FB_CONSOLE: SpinLock<OnceCell<fb::Console>> = SpinLock::new(OnceCell::new());

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

fn find_framebuffer(boot_info: &impl BootInfo) -> Option<fb::Fb> {
    let mut found: Option<fb::Desc> = None;

    for fb in boot_info.fb_iter() {
        if found.is_none() {
            found.replace(fb);
        }
        println!(
            "framebuffer detected [addr={:?}, width={}, height={}, bpp={}]",
            fb.address, fb.width, fb.height, fb.bpp
        );
    }

    let fb = found?;
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

pub fn init() -> impl BootInfo {
    log::println!("booting...");

    let bi = limine::init();

    bi.cmdline().inspect(|&cmdline| {
        println!("kernel command line: {:?}", cmdline);
        CMDLINE.acquire().set(cmdline);
    });

    let fb = find_framebuffer(&bi);

    if let Some(fb) = fb {
        let console = FB_CONSOLE.acquire();
        console
            .set(fb::Console::new(fb, font::terminus16_8::FONT))
            .unwrap();
        let console = unsafe { Pin::new_unchecked(console.get().unwrap_unchecked().base()) };
        crate::log::register(console);
        println!("framebuffer console registered");
    } else {
        println!("framebuffer console not supported");
    }

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
