#![no_main]
#![no_std]
#![feature(unsafe_pinned)]

use core::panic::PanicInfo;

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
pub mod arch;
pub mod gfx;
pub mod lending;
pub mod limine;
pub mod list;
pub mod log;
pub mod ptr;
pub mod sync;

#[panic_handler]
fn panic(pi: &PanicInfo) -> ! {
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

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    log::init();
    log::println!("booting...");
    limine::init();
    loop {}
}
