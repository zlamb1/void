#![no_main]
#![no_std]
#![feature(unsafe_pinned)]

use core::{panic::PanicInfo, pin::Pin};

use crate::gfx::fb::Mask;

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

static FB_REQUEST: sync::SpinLock<limine::FbRequest> =
    sync::SpinLock::new(limine::FbRequest::new());

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let fb = FB_REQUEST.with(|req| {
        let response = unsafe { req.response.unwrap().as_ref() };
        let fb = unsafe { (*response.framebuffers.unwrap().as_ptr()).unwrap().as_ref() };
        return gfx::fb::Fb::try_new(
            fb.address.cast(),
            fb.width.try_into().unwrap(),
            fb.height.try_into().unwrap(),
            fb.pitch.try_into().unwrap(),
            fb.bpp,
            Mask::new(fb.red_mask.size, fb.red_mask.shift),
            Mask::new(fb.green_mask.size, fb.green_mask.shift),
            Mask::new(fb.blue_mask.size, fb.blue_mask.shift),
        )
        .unwrap();
    });
    let console = gfx::fb::Console::new(fb, gfx::font::terminus16_8::FONT);
    let console = unsafe { Pin::new_unchecked(console.base()) };
    log::init();
    log::register(console);
    log::println!("Hello, Kernel!");
    log::println!("Hello, Kernel 2!");
    loop {}
}
