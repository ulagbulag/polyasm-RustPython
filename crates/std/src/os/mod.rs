//! Platform-specific parts: descriptors, C types and the PolyASM extensions.

pub mod fd;
pub mod polyasm;

/// C types.
pub mod raw {
    pub use core::ffi::{
        c_char, c_double, c_float, c_int, c_long, c_longlong, c_schar, c_short, c_uchar, c_uint,
        c_ulong, c_ulonglong, c_ushort, c_void,
    };
}
