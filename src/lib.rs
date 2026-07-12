#![no_main]
#![no_std]
#![feature(unsafe_pinned)]
#![feature(bstr)]

use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::boot::BootInfo;

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
pub mod arch;
pub mod boot;
pub mod date;
pub mod gfx;
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

fn mp_main(processor_id: u64) -> ! {
    println!("running mp{}", processor_id);
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    log::init();
    let boot_info = boot::init();
    mem::init(&boot_info);
    boot_info.mp_start(mp_main);
    loop {}
}
