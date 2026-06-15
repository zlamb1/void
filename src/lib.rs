#![no_main]
#![no_std]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
pub mod arch;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    loop {}
}
