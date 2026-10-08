//! The libc surface host_env shares with the libc platforms, served on PolyASM by the
//! `rustpython_std::host::Host` the embedder installs.
//!
//! A shared module names this module `libc` on PolyASM, so its `libc::` paths resolve here.
//! Each function keeps the libc signature and convention: a failure answers `-1` and leaves its
//! errno number for `std::io::Error::last_os_error`.

#![allow(non_camel_case_types)]

use core::{ffi::CStr, time::Duration};
use std::{
    host::{PollFd, host},
    io,
    os::polyasm::errno,
};

pub use crate::consts::*;
pub use core::ffi::{c_char, c_int, c_long, c_void};
pub use std::os::polyasm::{errno::*, fcntl::*, poll::*, stat::*};

pub type clockid_t = c_int;
pub type nfds_t = core::ffi::c_ulong;
pub type off_t = i64;
pub type pollfd = PollFd;
pub type suseconds_t = i64;
pub type time_t = i64;

/// `struct timespec`.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: time_t,
    pub tv_nsec: c_long,
}

/// `struct timeval`, public through `select::timeval`.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct timeval {
    pub tv_sec: time_t,
    pub tv_usec: suseconds_t,
}

/// Leaves the errno number of `error` and answers the libc failure value `-1`.
fn fail<T: From<i8>>(error: &io::Error) -> T {
    errno::set(errno::code_of(error));
    T::from(-1)
}

/// The libc answer of `result`: the value, or `-1` with its errno number left behind.
fn answer<T: From<i8>, V: TryInto<T>>(result: io::Result<V>) -> T {
    match result {
        Ok(value) => value
            .try_into()
            .unwrap_or_else(|_| fail(&io::Error::from_raw_os_error(EOVERFLOW))),
        Err(error) => fail(&error),
    }
}

/// # Safety
///
/// `path` points to a NUL-terminated string.
pub unsafe fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    // SAFETY: the caller hands a NUL-terminated string.
    let path = unsafe { CStr::from_ptr(path) };
    answer(host().open(path.to_bytes(), flags, mode.cast_unsigned()))
}

/// `openat(2)`: descriptors other than [`AT_FDCWD`] serve absolute paths alone.
///
/// # Safety
///
/// `path` points to a NUL-terminated string.
pub unsafe fn openat(dirfd: c_int, path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    // SAFETY: the caller hands a NUL-terminated string.
    let bytes = unsafe { CStr::from_ptr(path) }.to_bytes();
    if dirfd != AT_FDCWD && bytes.first() != Some(&b'/') {
        return fail(&io::Error::from_raw_os_error(ENOSYS));
    }
    // SAFETY: as above.
    unsafe { open(path, flags, mode) }
}

/// # Safety
///
/// `fd` is a descriptor the caller owns.
pub unsafe fn close(fd: c_int) -> c_int {
    answer(host().close(fd).map(|()| 0))
}

/// # Safety
///
/// `buf` points to `count` writable bytes.
pub unsafe fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize {
    // SAFETY: the caller hands `count` writable bytes.
    let buf = unsafe { core::slice::from_raw_parts_mut(buf.cast::<u8>(), count) };
    answer(host().read(fd, buf))
}

/// # Safety
///
/// `buf` points to `count` readable bytes.
pub unsafe fn write(fd: c_int, buf: *const c_void, count: usize) -> isize {
    // SAFETY: the caller hands `count` readable bytes.
    let buf = unsafe { core::slice::from_raw_parts(buf.cast::<u8>(), count) };
    answer(host().write(fd, buf))
}

/// # Safety
///
/// Every call is sound; the signature keeps the libc shape.
pub unsafe fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    answer(host().seek(fd, offset, whence))
}

/// # Safety
///
/// Every call is sound; the signature keeps the libc shape.
pub unsafe fn fsync(fd: c_int) -> c_int {
    answer(host().fsync(fd).map(|()| 0))
}

/// # Safety
///
/// Every call is sound; the signature keeps the libc shape.
pub unsafe fn ftruncate(fd: c_int, length: off_t) -> c_int {
    match u64::try_from(length) {
        Ok(length) => answer(host().ftruncate(fd, length).map(|()| 0)),
        Err(_) => fail(&io::Error::from_raw_os_error(EINVAL)),
    }
}

/// # Safety
///
/// Every call is sound; the signature keeps the libc shape.
pub unsafe fn isatty(fd: c_int) -> c_int {
    c_int::from(host().isatty(fd))
}

/// # Safety
///
/// `fds` points to `nfds` entries.
pub unsafe fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
    let Ok(len) = usize::try_from(nfds) else {
        return fail(&io::Error::from_raw_os_error(EINVAL));
    };
    // SAFETY: the caller hands `nfds` entries.
    let fds = unsafe { core::slice::from_raw_parts_mut(fds, len) };
    let timeout = u64::try_from(timeout).ok().map(Duration::from_millis);
    answer(host().poll(fds, timeout))
}

/// The reading of clock `clock`: the wall clock for [`CLOCK_REALTIME`], the monotonic clock
/// otherwise.
fn clock_now(clock: clockid_t) -> io::Result<Duration> {
    if clock == CLOCK_REALTIME {
        host().wall_clock()
    } else {
        host().monotonic()
    }
}

fn store_timespec(tp: *mut timespec, duration: Duration) -> c_int {
    let Ok(tv_sec) = time_t::try_from(duration.as_secs()) else {
        return fail(&io::Error::from_raw_os_error(EOVERFLOW));
    };
    let Ok(tv_nsec) = c_long::try_from(duration.subsec_nanos()) else {
        return fail(&io::Error::from_raw_os_error(EOVERFLOW));
    };
    // SAFETY: the callers hand a writable `timespec`.
    unsafe { tp.write(timespec { tv_sec, tv_nsec }) };
    0
}

/// # Safety
///
/// `tp` points to a writable `timespec`.
pub unsafe fn clock_gettime(clock: clockid_t, tp: *mut timespec) -> c_int {
    match clock_now(clock) {
        Ok(now) => store_timespec(tp, now),
        Err(error) => fail(&error),
    }
}

/// # Safety
///
/// `tp` points to a writable `timespec`.
pub unsafe fn clock_getres(clock: clockid_t, tp: *mut timespec) -> c_int {
    let _ = clock;
    store_timespec(tp, Duration::from_nanos(1))
}
