//! The POSIX surface of a PolyASM guest over the `rustpython_std::host::Host` of the embedder.
//!
//! Paths relative to a directory descriptor other than the working directory answer `ENOSYS`,
//! since the host resolves paths alone. The guest reports a fixed identity: one process of the
//! first user.

use core::{ffi::CStr, sync::atomic::AtomicU32, sync::atomic::Ordering, time::Duration};
use std::{
    host::host,
    io,
    os::polyasm::{
        errno::{EACCES, ENOSYS},
        stat::{S_IRUSR, S_IRWXG, S_IRWXO, S_IRWXU, S_IWUSR, S_IXUSR},
    },
    path::Path,
};

use crate::{
    consts::{
        GUEST_ID, HOSTNAME, INITIAL_UMASK, PARENT_PROCESS_ID, UNAME_MACHINE, UNAME_RELEASE,
        UNAME_SYSNAME, UNAME_VERSION,
    },
    crt_fd,
    fileutils::StatStruct,
    os::AccessFlag,
};

pub use crate::os::{AccessMode, F_OK, R_OK, W_OK, X_OK};

/// Permission bits of `mkdir(2)` and `open(2)`.
pub type RawMode = u32;

static UMASK: AtomicU32 = AtomicU32::new(INITIAL_UMASK);

/// `path` as the host resolves it: absolute, or relative to the working directory. A relative
/// path under a directory descriptor answers `ENOSYS`.
fn host_path<'a>(dir_fd: Option<crt_fd::Borrowed<'_>>, path: &'a Path) -> io::Result<&'a [u8]> {
    match dir_fd {
        Some(_) if !path.is_absolute() => Err(io::Error::from_raw_os_error(ENOSYS)),
        _ => Ok(path.as_os_str().as_encoded_bytes()),
    }
}

/// https://pubs.opengroup.org/onlinepubs/9799919799/functions/mkdir.html
pub fn make_dir(
    dir_fd: Option<crt_fd::Borrowed<'_>>,
    path: impl AsRef<Path>,
    mode: RawMode,
) -> io::Result<()> {
    let mode = mode & !UMASK.load(Ordering::Relaxed);
    host().mkdir(host_path(dir_fd, path.as_ref())?, mode)
}

/// https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html
pub fn rename(
    from: impl AsRef<Path>,
    from_fd: Option<crt_fd::Borrowed<'_>>,
    to: impl AsRef<Path>,
    to_fd: Option<crt_fd::Borrowed<'_>>,
) -> io::Result<()> {
    host().rename(
        host_path(from_fd, from.as_ref())?,
        host_path(to_fd, to.as_ref())?,
    )
}

/// https://docs.python.org/3/library/os.html#os.replace
///
/// The host rename replaces its target, so this forwards to [`rename`].
#[inline]
pub fn replace(
    from: impl AsRef<Path>,
    from_fd: Option<crt_fd::Borrowed<'_>>,
    to: impl AsRef<Path>,
    to_fd: Option<crt_fd::Borrowed<'_>>,
) -> io::Result<()> {
    rename(from, from_fd, to, to_fd)
}

pub fn remove_dir_at(
    dir_fd: Option<crt_fd::Borrowed<'_>>,
    path: impl AsRef<Path>,
) -> io::Result<()> {
    host().rmdir(host_path(dir_fd, path.as_ref())?)
}

pub fn unlinkat(dir_fd: Option<crt_fd::Borrowed<'_>>, path: impl AsRef<Path>) -> io::Result<()> {
    host().unlink(host_path(dir_fd, path.as_ref())?)
}

pub fn stat_path(
    path: impl AsRef<Path>,
    dir_fd: Option<crt_fd::Borrowed<'_>>,
    follow_symlinks: bool,
) -> io::Result<Option<StatStruct>> {
    let path = host_path(dir_fd, path.as_ref())?;
    let stat = if follow_symlinks {
        host().stat(path)?
    } else {
        host().lstat(path)?
    };
    Ok(Some(StatStruct::from(stat)))
}

pub fn stat_fd(fd: crt_fd::Borrowed<'_>) -> io::Result<StatStruct> {
    crate::fileutils::fstat(fd)
}

/// Sets access and modification times; the host keeps them itself, so this answers `ENOSYS`.
pub fn set_file_times_at(
    dir_fd: i32,
    path: &CStr,
    access: Duration,
    modified: Duration,
    follow_symlinks: bool,
) -> io::Result<()> {
    let _ = (dir_fd, path, access, modified, follow_symlinks);
    Err(io::Error::from_raw_os_error(ENOSYS))
}

/// Whether the guest holds `mode` access to `path`: the guest owns every file it reaches.
pub fn check_access(path: &Path, mode: u8) -> io::Result<bool> {
    let Some(mode) = AccessMode::from_bits(mode) else {
        return Err(io::Error::from_raw_os_error(EACCES));
    };
    let Ok(stat) = host().stat(path.as_os_str().as_encoded_bytes()) else {
        return Ok(false);
    };
    let granted = [
        (AccessFlag::R, S_IRUSR),
        (AccessFlag::W, S_IWUSR),
        (AccessFlag::X, S_IXUSR),
    ];
    Ok(granted
        .iter()
        .all(|(flag, bit)| !mode.contains(flag) || stat.mode & bit != 0))
}

#[must_use]
pub const fn getpid() -> i32 {
    std::process::id().cast_signed()
}

#[must_use]
pub const fn getppid() -> i32 {
    PARENT_PROCESS_ID
}

#[must_use]
pub const fn getuid() -> u32 {
    GUEST_ID
}

#[must_use]
pub const fn geteuid() -> u32 {
    GUEST_ID
}

#[must_use]
pub const fn getgid() -> u32 {
    GUEST_ID
}

#[must_use]
pub const fn getegid() -> u32 {
    GUEST_ID
}

pub fn getgroups() -> io::Result<Vec<u32>> {
    Ok(vec![GUEST_ID])
}

/// Sets the file-creation mask, answering the previous one.
pub fn umask(mask: RawMode) -> RawMode {
    UMASK.swap(mask & (S_IRWXU | S_IRWXG | S_IRWXO), Ordering::Relaxed)
}

/// The `uname(2)` fields of the guest.
#[derive(Clone, Debug)]
pub struct UnameInfo {
    pub sysname: String,
    pub nodename: String,
    pub release: String,
    pub version: String,
    pub machine: String,
}

#[must_use]
pub fn uname_info() -> UnameInfo {
    UnameInfo {
        sysname: UNAME_SYSNAME.to_owned(),
        nodename: HOSTNAME.to_owned(),
        release: UNAME_RELEASE.to_owned(),
        version: UNAME_VERSION.to_owned(),
        machine: UNAME_MACHINE.to_owned(),
    }
}
