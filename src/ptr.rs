#[macro_export]
macro_rules! container_of {
    ($field_ptr:expr, $Container:ty, $field:ident) => {{
        let ptr = $field_ptr as *const _ as *const u8;
        let ptr = ptr.wrapping_sub(core::mem::offset_of!($Container, $field));
        ptr.cast::<$Container>()
    }};
}

#[macro_export]
macro_rules! container_of_mut {
    ($field_ptr:expr, $Container:ty, $field:ident) => {{
        let ptr = $field_ptr as *const _ as *const u8;
        let ptr = ptr.wrapping_sub(core::mem::offset_of!($Container, $field));
        ptr.cast_mut::<$Container>()
    }};
}
