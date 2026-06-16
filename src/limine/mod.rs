use core::ptr::NonNull;

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
