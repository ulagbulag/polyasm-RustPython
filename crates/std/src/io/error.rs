use alloc_crate::{boxed::Box, collections::TryReserveError, ffi::NulError};
use core::{error, fmt, result};

use crate::consts::{ERRNO_TABLE, KIND_ERRNOS};

/// A specialized [`Result`](result::Result) type for I/O operations.
pub type Result<T> = result::Result<T, Error>;

/// The error type of I/O operations: a Linux errno number, a kind, or a payload of any error.
pub struct Error {
    repr: Repr,
}

enum Repr {
    Os(i32),
    Simple(ErrorKind),
    SimpleMessage(ErrorKind, &'static str),
    Custom(Box<Custom>),
}

#[derive(Debug)]
struct Custom {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

/// The general categories of I/O failures, matching [`std::io::ErrorKind`].
///
/// [`std::io::ErrorKind`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    FilesystemLoop,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    QuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    InvalidFilename,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    InProgress,
    Other,
    #[doc(hidden)]
    Uncategorized,
}

impl ErrorKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "entity not found",
            Self::PermissionDenied => "permission denied",
            Self::ConnectionRefused => "connection refused",
            Self::ConnectionReset => "connection reset",
            Self::HostUnreachable => "host unreachable",
            Self::NetworkUnreachable => "network unreachable",
            Self::ConnectionAborted => "connection aborted",
            Self::NotConnected => "not connected",
            Self::AddrInUse => "address in use",
            Self::AddrNotAvailable => "address not available",
            Self::NetworkDown => "network down",
            Self::BrokenPipe => "broken pipe",
            Self::AlreadyExists => "entity already exists",
            Self::WouldBlock => "operation would block",
            Self::NotADirectory => "not a directory",
            Self::IsADirectory => "is a directory",
            Self::DirectoryNotEmpty => "directory not empty",
            Self::ReadOnlyFilesystem => "read-only filesystem or storage medium",
            Self::FilesystemLoop => "filesystem loop or indirection limit (e.g. symlink loop)",
            Self::StaleNetworkFileHandle => "stale network file handle",
            Self::InvalidInput => "invalid input parameter",
            Self::InvalidData => "invalid data",
            Self::TimedOut => "timed out",
            Self::WriteZero => "write zero",
            Self::StorageFull => "no storage space",
            Self::NotSeekable => "seek on unseekable file",
            Self::QuotaExceeded => "quota exceeded",
            Self::FileTooLarge => "file too large",
            Self::ResourceBusy => "resource busy",
            Self::ExecutableFileBusy => "executable file busy",
            Self::Deadlock => "deadlock",
            Self::CrossesDevices => "cross-device link or rename",
            Self::TooManyLinks => "too many links",
            Self::InvalidFilename => "invalid filename",
            Self::ArgumentListTooLong => "argument list too long",
            Self::Interrupted => "operation interrupted",
            Self::Unsupported => "unsupported",
            Self::UnexpectedEof => "unexpected end of file",
            Self::OutOfMemory => "out of memory",
            Self::InProgress => "in progress",
            Self::Other => "other error",
            Self::Uncategorized => "uncategorized error",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The [`ErrorKind`] a Linux errno number decodes to.
pub(crate) fn decode_error_kind(code: i32) -> ErrorKind {
    ERRNO_TABLE
        .iter()
        .find(|row| row.code == code)
        .map_or(ErrorKind::Uncategorized, |row| row.kind)
}

/// The canonical Linux errno number of an [`ErrorKind`].
pub(crate) fn kind_errno(kind: ErrorKind) -> i32 {
    KIND_ERRNOS
        .iter()
        .find(|(row, _)| *row == kind)
        .map(|(_, code)| *code)
        .or_else(|| {
            ERRNO_TABLE
                .iter()
                .find(|row| row.kind == kind)
                .map(|row| row.code)
        })
        .unwrap_or(crate::consts::EIO)
}

/// The `strerror(3)` text of a Linux errno number.
pub(crate) fn strerror(code: i32) -> Option<&'static str> {
    ERRNO_TABLE
        .iter()
        .find(|row| row.code == code)
        .map(|row| row.message)
}

impl Error {
    /// Creates an error of `kind` carrying `error` as its payload.
    pub fn new<E>(kind: ErrorKind, error: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self {
            repr: Repr::Custom(Box::new(Custom {
                kind,
                error: error.into(),
            })),
        }
    }

    /// Creates an [`ErrorKind::Other`] error carrying `error` as its payload.
    pub fn other<E>(error: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::new(ErrorKind::Other, error)
    }

    /// Creates an error of `kind` with a static message.
    pub(crate) const fn const_message(kind: ErrorKind, message: &'static str) -> Self {
        Self {
            repr: Repr::SimpleMessage(kind, message),
        }
    }

    /// The error of the errno number the last failed operating-system call left behind.
    #[must_use]
    pub fn last_os_error() -> Self {
        Self::from_raw_os_error(crate::os::polyasm::errno::get())
    }

    /// Creates an error from a Linux errno number.
    #[must_use]
    pub const fn from_raw_os_error(code: i32) -> Self {
        Self {
            repr: Repr::Os(code),
        }
    }

    /// The Linux errno number of an operating-system error.
    #[must_use]
    pub const fn raw_os_error(&self) -> Option<i32> {
        match self.repr {
            Repr::Os(code) => Some(code),
            _ => None,
        }
    }

    /// The payload of an error made by [`Error::new`] or [`Error::other`].
    #[must_use]
    pub fn get_ref(&self) -> Option<&(dyn error::Error + Send + Sync + 'static)> {
        match &self.repr {
            Repr::Custom(custom) => Some(&*custom.error),
            _ => None,
        }
    }

    /// The mutable payload of an error made by [`Error::new`] or [`Error::other`].
    pub fn get_mut(&mut self) -> Option<&mut (dyn error::Error + Send + Sync + 'static)> {
        match &mut self.repr {
            Repr::Custom(custom) => Some(&mut *custom.error),
            _ => None,
        }
    }

    /// The payload of an error made by [`Error::new`] or [`Error::other`], by value.
    #[must_use]
    pub fn into_inner(self) -> Option<Box<dyn error::Error + Send + Sync>> {
        match self.repr {
            Repr::Custom(custom) => Some(custom.error),
            _ => None,
        }
    }

    /// The payload as `E` when it holds an `E`, and the error itself otherwise.
    pub fn downcast<E>(self) -> result::Result<E, Self>
    where
        E: error::Error + Send + Sync + 'static,
    {
        match self.repr {
            Repr::Custom(custom) if custom.error.is::<E>() => {
                let Custom { error, .. } = *custom;
                Ok(*error.downcast::<E>().unwrap_or_else(|_| unreachable!()))
            }
            repr => Err(Self { repr }),
        }
    }

    /// The category of the error.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match &self.repr {
            Repr::Os(code) => decode_error_kind(*code),
            Repr::Simple(kind) | Repr::SimpleMessage(kind, _) => *kind,
            Repr::Custom(custom) => custom.kind,
        }
    }

    pub(crate) fn is_interrupted(&self) -> bool {
        self.kind() == ErrorKind::Interrupted
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self {
            repr: Repr::Simple(kind),
        }
    }
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Self::const_message(ErrorKind::InvalidInput, "data provided contains a nul byte")
    }
}

impl From<TryReserveError> for Error {
    fn from(_: TryReserveError) -> Self {
        Self::from(ErrorKind::OutOfMemory)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Os(code) => f
                .debug_struct("Os")
                .field("code", code)
                .field("kind", &decode_error_kind(*code))
                .field("message", &strerror(*code).unwrap_or("Unknown error"))
                .finish(),
            Repr::Simple(kind) => f.debug_tuple("Kind").field(kind).finish(),
            Repr::SimpleMessage(kind, message) => f
                .debug_struct("Error")
                .field("kind", kind)
                .field("message", message)
                .finish(),
            Repr::Custom(custom) => fmt::Debug::fmt(custom, f),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Os(code) => match strerror(*code) {
                Some(message) => write!(f, "{message} (os error {code})"),
                None => write!(f, "Unknown error {code} (os error {code})"),
            },
            Repr::Simple(kind) => f.write_str(kind.as_str()),
            Repr::SimpleMessage(_, message) => f.write_str(message),
            Repr::Custom(custom) => fmt::Display::fmt(&custom.error, f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.repr {
            Repr::Custom(custom) => custom.error.source(),
            _ => None,
        }
    }
}
