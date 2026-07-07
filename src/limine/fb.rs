use core::ffi::c_void;

#[repr(C)]
pub struct Mask {
    pub size: u8,
    pub shift: u8,
}

#[repr(C)]
pub struct Fb {
    pub addr: *mut c_void,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    /// Bits per pixel.
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask: Mask,
    pub green_mask: Mask,
    pub blue_mask: Mask,
    unused: [u8; 7],
    pub edid_size: u64,
    pub edid: *mut c_void,
}
