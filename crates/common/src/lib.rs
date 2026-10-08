//! A crate to hold types and functions common to all rustpython components.

#![cfg_attr(any(not(feature = "std"), target_abi = "polyasm"), no_std)]
#![cfg_attr(
    target_abi = "polyasm",
    feature(prelude_import),
    allow(internal_features)
)]
#![deny(clippy::disallowed_methods)]

#[cfg(target_abi = "polyasm")]
extern crate rustpython_std as std;
#[cfg(target_abi = "polyasm")]
#[prelude_import]
#[allow(
    unused_imports,
    reason = "names reach the crate through prelude resolution"
)]
use std::prelude::rust_2024::*;

extern crate alloc;

pub mod atomic;
#[cfg(feature = "binascii")]
pub mod binascii;
pub mod borrow;
pub mod boxvec;
pub mod cformat;
#[cfg(any(feature = "bz2", feature = "lzma", feature = "zlib"))]
pub mod compression;
pub mod encodings;
pub mod float_ops;
pub mod format;
pub mod hash;
#[cfg(feature = "hashlib")]
pub mod hashlib;
#[cfg(feature = "inet")]
pub mod inet;
pub mod int;
#[cfg(feature = "json")]
pub mod json;
pub mod linked_list;
pub mod lock;
pub mod rand;
pub mod rc;
pub mod refcount;
pub mod static_cell;
pub mod str;
pub mod wtf8_index;

pub use rustpython_wtf8 as wtf8;

pub mod vendored {
    pub use ascii;
}
