//! Byte views of [`OsStr`] and [`OsString`].

use alloc_crate::vec::Vec;

use crate::ffi::{OsStr, OsString};

mod sealed {
    pub trait Sealed {}

    impl Sealed for crate::ffi::OsStr {}
    impl Sealed for crate::ffi::OsString {}
}

/// Byte access to [`OsStr`].
pub trait OsStrExt: sealed::Sealed {
    fn from_bytes(slice: &[u8]) -> &Self;
    fn as_bytes(&self) -> &[u8];
}

impl OsStrExt for OsStr {
    fn from_bytes(slice: &[u8]) -> &Self {
        Self::from_bytes(slice)
    }

    fn as_bytes(&self) -> &[u8] {
        self.bytes()
    }
}

/// Byte access to [`OsString`].
pub trait OsStringExt: sealed::Sealed {
    fn from_vec(vec: Vec<u8>) -> Self;
    fn into_vec(self) -> Vec<u8>;
}

impl OsStringExt for OsString {
    fn from_vec(vec: Vec<u8>) -> Self {
        Self::from_vec(vec)
    }

    fn into_vec(self) -> Vec<u8> {
        self.into_vec()
    }
}
