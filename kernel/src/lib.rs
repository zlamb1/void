#![no_main]
#![no_std]
#![feature(unsafe_pinned)]
#![feature(bstr)]

use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
pub mod arch;
pub mod backtrace;
pub mod boot;
pub mod date;
pub mod gfx;
pub mod list;
pub mod log;
pub mod mem;
pub mod mp;
pub mod per_cpu;
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

    println!("");
    println!("backtrace:");
    backtrace::backtrace();

    loop {
        arch::halt();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    // SAFETY: This must come before any locking
    // or other code that uses CPU ID. It loads
    // a per-cpu structure for the BSP.
    mp::init();
    log::init();

    let boot_info = boot::init();
    mem::init(&boot_info);

    mp::kickoff(&boot_info);
}
