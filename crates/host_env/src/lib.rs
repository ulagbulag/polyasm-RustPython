#![cfg_attr(
    target_abi = "polyasm",
    no_std,
    feature(prelude_import),
    allow(internal_features)
)]
#![allow(clippy::must_use_candidate)]

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

#[macro_use]
mod macros;
pub use macros::*;

#[cfg(target_abi = "polyasm")]
mod consts;
#[cfg(all(feature = "ctypes", not(target_abi = "polyasm")))]
pub mod ctypes;
#[cfg(any(unix, windows, target_os = "wasi", target_abi = "polyasm"))]
pub mod errno;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[path = "errno_wasm.rs"]
pub mod errno;
#[cfg(any(unix, windows, target_os = "wasi", target_abi = "polyasm"))]
pub mod io;
#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
#[path = "io_unsupported.rs"]
pub mod io;
/// The libc surface of PolyASM: errno numbers, flags and the calls `rustpython_std::host::Host` serves.
#[cfg(target_abi = "polyasm")]
pub mod libc_polyasm;
#[cfg(target_abi = "polyasm")]
pub use libc_polyasm as libc;
pub mod os;
#[cfg(any(unix, windows))]
pub mod thread;

#[cfg(any(unix, windows, target_os = "wasi", target_abi = "polyasm"))]
pub mod crt_fd;
#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
#[path = "crt_fd_unsupported.rs"]
pub mod crt_fd;

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub mod fileutils;
pub mod fs;
#[cfg(any(unix, windows))]
pub mod locale;
#[cfg(feature = "native-certs")]
pub mod native_certs;
pub mod readline;
#[cfg(feature = "ssl")]
pub mod ssl;

#[cfg(windows)]
pub mod windows;

#[cfg(any(unix, target_os = "wasi"))]
pub mod fcntl;
#[cfg(any(unix, windows, target_os = "wasi", target_abi = "polyasm"))]
pub mod select;
#[cfg(any(unix, windows))]
pub mod socket;
#[cfg(any(all(target_arch = "wasm32", target_os = "unknown"), target_os = "wasi"))]
#[path = "socket_wasm.rs"]
pub mod socket;
cfg_select! {
    target_abi = "polyasm" => {
        mod socket_wasm;
        pub mod socket_polyasm;
        pub use socket_polyasm as socket;
    }
    _ => {}
}
#[cfg(unix)]
pub mod syslog;
#[cfg(all(unix, not(target_os = "redox"), not(target_os = "ios")))]
pub mod termios;

#[cfg(unix)]
pub mod grp;
#[cfg(unix)]
pub mod posix;
#[cfg(target_os = "wasi")]
#[path = "posix_wasi.rs"]
pub mod posix;
cfg_select! {
    target_abi = "polyasm" => {
        pub mod posix_polyasm;
        pub use posix_polyasm as posix;
    }
    _ => {}
}
#[cfg(windows)]
#[path = "posix_windows.rs"]
pub mod posix;
#[cfg(any(unix, target_os = "wasi"))]
pub mod posix_unix_like;
#[cfg(unix)]
pub mod pwd;
#[cfg(unix)]
pub mod resource;
#[cfg(all(unix, not(target_os = "redox"), not(target_os = "android")))]
pub mod shm;
#[cfg(any(unix, windows))]
pub mod signal;
pub mod time;

#[cfg(windows)]
pub mod cert_store;
#[cfg(target_os = "macos")]
pub mod system_configuration {
    pub use ::system_configuration::*;
}
#[cfg(any(unix, windows))]
pub mod faulthandler;
#[cfg(any(unix, windows))]
pub mod mmap;
#[cfg(windows)]
pub mod msvcrt;
#[cfg(any(unix, windows))]
pub mod multiprocessing;
#[cfg(windows)]
pub mod nt;
#[cfg(windows)]
pub mod overlapped;
#[cfg(windows)]
pub mod testconsole;
#[cfg(windows)]
pub mod uuid;
#[cfg(windows)]
pub mod winapi;
#[cfg(windows)]
pub mod winreg;
#[cfg(windows)]
pub mod winsound;
#[cfg(windows)]
pub mod wmi;
