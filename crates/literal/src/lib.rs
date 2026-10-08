#![no_std]
#![cfg_attr(
    target_abi = "polyasm",
    feature(prelude_import),
    allow(internal_features)
)]

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

pub mod complex;
pub mod escape;
pub mod float;
pub mod format;
pub mod hexf;
