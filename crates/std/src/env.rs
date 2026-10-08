//! Arguments, environment variables and the working directory of the guest.

use alloc_crate::{string::String, vec, vec::Vec};
use core::{cell::RefCell, error, fmt, iter::FusedIterator};

use crate::{
    consts::{PATH_LIST_SEPARATOR, TEMP_DIR},
    ffi::{OsStr, OsString},
    host::host,
    io,
    path::{self, Path, PathBuf},
    sys::GuestCell,
};

/// Constants that describe the PolyASM target.
pub mod consts {
    pub use crate::consts::{
        ARCH, DLL_EXTENSION, DLL_PREFIX, DLL_SUFFIX, EXE_EXTENSION, EXE_SUFFIX, FAMILY, OS,
    };
}

type Environment = Vec<(OsString, OsString)>;

static ENVIRONMENT: GuestCell<RefCell<Option<Environment>>> = GuestCell::new(RefCell::new(None));

/// Runs `f` on the environment, reading it from the host on first use.
fn with_environment<R>(f: impl FnOnce(&mut Environment) -> R) -> R {
    let mut environment = ENVIRONMENT.borrow_mut();
    let environment = environment.get_or_insert_with(|| {
        host()
            .environ()
            .into_iter()
            .map(|(name, value)| (OsString::from_vec(name), OsString::from_vec(value)))
            .collect()
    });
    f(environment)
}

/// The error of [`var`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VarError {
    /// The variable lies outside the environment.
    NotPresent,
    /// The value holds bytes other than UTF-8.
    NotUnicode(OsString),
}

impl fmt::Display for VarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPresent => f.write_str("environment variable not found"),
            Self::NotUnicode(s) => write!(f, "environment variable was not valid unicode: {s:?}"),
        }
    }
}

impl error::Error for VarError {}

/// The value of the environment variable `key`.
pub fn var<K: AsRef<OsStr>>(key: K) -> Result<String, VarError> {
    match var_os(key) {
        Some(value) => value.into_string().map_err(VarError::NotUnicode),
        None => Err(VarError::NotPresent),
    }
}

/// The value of the environment variable `key`, as bytes.
pub fn var_os<K: AsRef<OsStr>>(key: K) -> Option<OsString> {
    let key = key.as_ref();
    with_environment(|env| {
        env.iter()
            .find(|(name, _)| name.as_os_str() == key)
            .map(|(_, value)| value.clone())
    })
}

/// Sets the environment variable `key` to `value`.
///
/// # Safety
///
/// The guest runs one thread, so this call alone touches the environment; the function keeps
/// the signature of std.
pub unsafe fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    let key = key.as_ref();
    let value = value.as_ref().to_os_string();
    with_environment(
        |env| match env.iter_mut().find(|(name, _)| name.as_os_str() == key) {
            Some((_, slot)) => *slot = value,
            None => env.push((key.to_os_string(), value)),
        },
    );
}

/// Removes the environment variable `key`.
///
/// # Safety
///
/// The guest runs one thread, so this call alone touches the environment; the function keeps
/// the signature of std.
pub unsafe fn remove_var<K: AsRef<OsStr>>(key: K) {
    let key = key.as_ref();
    with_environment(|env| env.retain(|(name, _)| name.as_os_str() != key));
}

/// Iterator over the environment as UTF-8 pairs, made by [`vars`].
pub struct Vars {
    inner: VarsOs,
}

/// Iterator over the environment as byte pairs, made by [`vars_os`].
pub struct VarsOs {
    inner: vec::IntoIter<(OsString, OsString)>,
}

/// The environment as UTF-8 pairs. A pair of other bytes panics, as in std.
#[must_use]
pub fn vars() -> Vars {
    Vars { inner: vars_os() }
}

/// The environment as byte pairs.
#[must_use]
pub fn vars_os() -> VarsOs {
    VarsOs {
        inner: with_environment(|env| env.clone()).into_iter(),
    }
}

impl Iterator for Vars {
    type Item = (String, String);

    fn next(&mut self) -> Option<(String, String)> {
        self.inner.next().map(|(name, value)| {
            (
                name.into_string().expect("environment names are UTF-8"),
                value.into_string().expect("environment values are UTF-8"),
            )
        })
    }
}

impl Iterator for VarsOs {
    type Item = (OsString, OsString);

    fn next(&mut self) -> Option<(OsString, OsString)> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl fmt::Debug for Vars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vars").finish_non_exhaustive()
    }
}

impl fmt::Debug for VarsOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VarsOs").finish_non_exhaustive()
    }
}

/// Iterator over the program arguments as UTF-8 text, made by [`args`].
pub struct Args {
    inner: ArgsOs,
}

/// Iterator over the program arguments as bytes, made by [`args_os`].
pub struct ArgsOs {
    inner: vec::IntoIter<OsString>,
}

/// The program arguments as UTF-8 text, `argv[0]` first. An argument of other bytes panics,
/// as in std.
#[must_use]
pub fn args() -> Args {
    Args { inner: args_os() }
}

/// The program arguments as bytes, `argv[0]` first.
#[must_use]
pub fn args_os() -> ArgsOs {
    ArgsOs {
        inner: host()
            .argv()
            .into_iter()
            .map(OsString::from_vec)
            .collect::<Vec<_>>()
            .into_iter(),
    }
}

impl Iterator for Args {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.inner
            .next()
            .map(|arg| arg.into_string().expect("program arguments are UTF-8"))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Args {}

impl DoubleEndedIterator for Args {
    fn next_back(&mut self) -> Option<String> {
        self.inner
            .next_back()
            .map(|arg| arg.into_string().expect("program arguments are UTF-8"))
    }
}

impl FusedIterator for Args {}

impl Iterator for ArgsOs {
    type Item = OsString;

    fn next(&mut self) -> Option<OsString> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for ArgsOs {}

impl DoubleEndedIterator for ArgsOs {
    fn next_back(&mut self) -> Option<OsString> {
        self.inner.next_back()
    }
}

impl FusedIterator for ArgsOs {}

impl fmt::Debug for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.inner.as_slice()).finish()
    }
}

impl fmt::Debug for ArgsOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.as_slice()).finish()
    }
}

/// The working directory of the guest.
pub fn current_dir() -> io::Result<PathBuf> {
    host().getcwd().map(path::from_host_bytes)
}

/// Makes `path` the working directory of the guest.
pub fn set_current_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    host().chdir(path::host_bytes(path.as_ref()))
}

/// The directory for temporary files: `TMPDIR`, or `/tmp`.
#[must_use]
pub fn temp_dir() -> PathBuf {
    var_os("TMPDIR").map_or_else(|| path::from_host_bytes(TEMP_DIR.to_vec()), PathBuf::from)
}

/// The home directory of the guest user: `HOME`.
#[must_use]
pub fn home_dir() -> Option<PathBuf> {
    var_os("HOME").map(PathBuf::from)
}

/// The path of the running program: `argv[0]`.
pub fn current_exe() -> io::Result<PathBuf> {
    host()
        .argv()
        .into_iter()
        .next()
        .map(path::from_host_bytes)
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

/// Iterator over the paths of a `PATH`-style list, made by [`split_paths`].
pub struct SplitPaths<'a> {
    rest: Option<&'a [u8]>,
}

/// The paths of a `:`-separated list.
pub fn split_paths<T: AsRef<OsStr> + ?Sized>(paths: &T) -> SplitPaths<'_> {
    SplitPaths {
        rest: Some(paths.as_ref().as_encoded_bytes()),
    }
}

impl Iterator for SplitPaths<'_> {
    type Item = PathBuf;

    fn next(&mut self) -> Option<PathBuf> {
        let rest = self.rest?;
        let path = match rest.iter().position(|b| *b == PATH_LIST_SEPARATOR) {
            Some(i) => {
                self.rest = Some(&rest[i + 1..]);
                &rest[..i]
            }
            None => {
                self.rest = None;
                rest
            }
        };
        Some(path::from_host_bytes(path.to_vec()))
    }
}

impl fmt::Debug for SplitPaths<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitPaths").finish_non_exhaustive()
    }
}

/// The error of [`join_paths`]: a path holds the `:` separator.
#[derive(Debug)]
pub struct JoinPathsError;

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("path segment contains separator `:`")
    }
}

impl error::Error for JoinPathsError {}

/// A `:`-separated list of `paths`.
pub fn join_paths<I, T>(paths: I) -> Result<OsString, JoinPathsError>
where
    I: IntoIterator<Item = T>,
    T: AsRef<OsStr>,
{
    let mut joined = Vec::new();
    for (i, path) in paths.into_iter().enumerate() {
        let bytes = path.as_ref().as_encoded_bytes();
        if bytes.contains(&PATH_LIST_SEPARATOR) {
            return Err(JoinPathsError);
        }
        if i > 0 {
            joined.push(PATH_LIST_SEPARATOR);
        }
        joined.extend_from_slice(bytes);
    }
    Ok(OsString::from_vec(joined))
}
