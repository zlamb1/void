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
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    log::init();
    log::println!("booting...");
    limine::init();
    unsafe {
        let x = 1 as *mut u8;
        x.read();
    }
    loop {}
}
