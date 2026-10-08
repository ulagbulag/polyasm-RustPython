//! The std surface RustPython runs on in PolyASM guests.
//!
//! A PolyASM sysroot carries `core`, `alloc` and `compiler_builtins`. On PolyASM each RustPython
//! crate root names this crate `std` and imports its prelude:
//!
//! ```ignore
//! #![cfg_attr(target_abi = "polyasm", no_std, feature(prelude_import), allow(internal_features))]
//! #[cfg(target_abi = "polyasm")]
//! extern crate rustpython_std as std;
//! #[cfg(target_abi = "polyasm")]
//! #[prelude_import]
//! #[allow(unused_imports, reason = "names reach the crate through prelude resolution")]
//! use std::prelude::rust_2024::*;
//! ```
//!
//! Every `std::` path, the prelude and the std macros then resolve here. `core` and `alloc`
//! items keep their std names; the operating-system parts (files, streams, clocks, entropy,
//! sockets, the environment) call the [`host::Host`] the embedder installs. The guest runs one
//! thread, the ground for the `Sync` claims of locks and thread-local values.

#![cfg(target_abi = "polyasm")]
#![no_std]
#![feature(abort_immediate, decl_macro, hashmap_internals)]
#![allow(internal_features)]

extern crate alloc as alloc_crate;

mod consts;
mod macros;
mod sys;

pub mod collections;
pub mod env;
pub mod ffi;
pub mod fs;
pub mod host;
pub mod io;
pub mod net;
pub mod os;
pub mod panic;
pub mod path;
pub mod prelude;
pub mod process;
pub mod sync;
pub mod thread;
pub mod time;

/// Memory allocation of `alloc` and `core`.
pub mod alloc {
    pub use alloc_crate::alloc::*;
}

pub use alloc_crate::{borrow, boxed, fmt, format, rc, slice, str, string, vec};
pub use core::{
    any, array, ascii, cell, char, clone, cmp, convert, default, error, f32, f64, future, hash,
    hint, iter, marker, mem, num, ops, option, pin, primitive, ptr, result, task,
};
#[expect(deprecated)]
pub use core::{
    assert, assert_eq, assert_ne, cfg, cfg_select, column, compile_error, concat, debug_assert,
    debug_assert_eq, debug_assert_ne, env, file, format_args, include, line, matches, module_path,
    option_env, stringify, r#try, unimplemented, unreachable, write, writeln,
};
pub use macros::{dbg, eprint, eprintln, print, println};
pub use thread::thread_local;
