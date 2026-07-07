use core::cell::OnceCell;
use core::pin::Pin;
use core::ptr::NonNull;

use crate::gfx;
use crate::sync::SpinLock;

pub mod fb;

#[repr(C)]
pub struct Request<const M0: u64, const M1: u64, R> {
    id: [u64; 4],
    rev: u64,
    pub response: Option<NonNull<R>>,
}

impl<const M0: u64, const M1: u64, R> Request<M0, M1, R> {
    pub const fn new() -> Self {
        Self {
            id: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, M0, M1],
            rev: 0,
            response: None,
        }
    }
}

unsafe impl<const M0: u64, const M1: u64, R> Send for Request<M0, M1, R> {}

pub type FbRequest = Request<0x9d5827dcd881dd75, 0xa3148604f6fab11b, FbResponse>;

#[repr(C)]
pub struct FbResponse {
    rev: u64,
    pub count: u64,
    pub framebuffers: Option<NonNull<Option<NonNull<fb::Fb>>>>,
}

static FB_REQUEST: SpinLock<FbRequest> = SpinLock::new(FbRequest::new());
static FB_CONSOLE: SpinLock<OnceCell<gfx::fb::Console>> = SpinLock::new(OnceCell::new());

pub fn init() {
    let fb = FB_REQUEST.with(|req| {
        let response = unsafe { req.response.unwrap().as_ref() };
        let fb = unsafe { (*response.framebuffers.unwrap().as_ptr()).unwrap().as_ref() };
        gfx::fb::Fb::try_new(
            fb.address.cast(),
            fb.width.try_into().unwrap(),
            fb.height.try_into().unwrap(),
            fb.pitch.try_into().unwrap(),
            fb.bpp,
            gfx::fb::Mask::new(fb.red_mask.size, fb.red_mask.shift),
            gfx::fb::Mask::new(fb.green_mask.size, fb.green_mask.shift),
            gfx::fb::Mask::new(fb.blue_mask.size, fb.blue_mask.shift),
        )
        .unwrap()
    });
    FB_CONSOLE.with_mut(|console| {
        console
            .set(gfx::fb::Console::new(fb, gfx::font::terminus16_8::FONT))
            .unwrap();
        let console = unsafe { Pin::new_unchecked(console.get().unwrap_unchecked().base()) };
        crate::log::register(console);
    });
}
