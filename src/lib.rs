#![no_main]
#![no_std]
#![feature(unsafe_pinned)]

use core::panic::PanicInfo;

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
pub mod arch;
pub mod boot;
pub mod date;
pub mod gfx;
pub mod lending;
pub mod limine;
pub mod list;
pub mod log;
pub mod mem;
pub mod ptr;
pub mod sync;

#[panic_handler]
fn panic(pi: &PanicInfo) -> ! {
    log::clear();
    println!("panic: {}", pi.message());
    if let Some(location) = pi.location() {
        println!(
            "location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }
    loop {}
}

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

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    log::init();
    log::println!("booting...");
    let bi = limine::init();
    log_boot_time(&bi.boot_time);
    mem::init(&bi);
    loop {}
}
