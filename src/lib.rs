#![no_main]
#![no_std]
#![feature(unsafe_pinned)]

use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
pub mod arch;
pub mod boot;
pub mod date;
pub mod gfx;
pub mod lending;
pub mod list;
pub mod log;
pub mod mem;
pub mod ptr;
pub mod sync;

static HAS_PANICKED: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(pi: &PanicInfo) -> ! {
    arch::irq_disable();
    if HAS_PANICKED.swap(true, Ordering::SeqCst) {
        loop {
            arch::halt();
        }
    }
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
    loop {
        arch::halt();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    log::init();
    let bi = boot::init();
    mem::init(&bi);
    loop {}
}
