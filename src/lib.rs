#![no_main]
#![no_std]
#![feature(unsafe_pinned)]

use core::panic::PanicInfo;

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
pub mod arch;
pub mod lending;
pub mod limine;
pub mod list;
pub mod log;
pub mod sync;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

#[macro_export]
macro_rules! container_of {
    ($field_ptr:expr, $Container:ty, $field:ident) => {{
        let ptr = $field_ptr as *const _ as *const u8;
        let ptr = ptr.wrapping_sub(core::mem::offset_of!($Container, $field));
        ptr.cast::<$Container>()
    }};
}

static FB_REQUEST: sync::SpinLock<limine::FbRequest> =
    sync::SpinLock::new(limine::FbRequest::new());

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    FB_REQUEST.with(|req| {
        let response = unsafe { req.response.unwrap().as_ref() };
        let fb = unsafe { (*response.framebuffers.unwrap().as_ptr()).unwrap().as_ref() };
        let fb: *mut u8 = fb.address.cast();
        unsafe {
            fb.write_volatile(255);
        }
    });
    log::init();
    log::print!("Hello, Kernel!");
    loop {}
}
