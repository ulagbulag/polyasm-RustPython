//! Linux errno numbers, the `errno` value of the guest, and errno conversions.

use core::cell::Cell;

use crate::{io, sys::GuestCell};

pub use crate::consts::{
    E2BIG, EACCES, EADDRINUSE, EADDRNOTAVAIL, EADV, EAFNOSUPPORT, EAGAIN, EALREADY, EBADE, EBADF,
    EBADFD, EBADMSG, EBADR, EBADRQC, EBADSLT, EBFONT, EBUSY, ECANCELED, ECHILD, ECHRNG, ECOMM,
    ECONNABORTED, ECONNREFUSED, ECONNRESET, EDEADLK, EDEADLOCK, EDESTADDRREQ, EDOM, EDOTDOT,
    EDQUOT, EEXIST, EFAULT, EFBIG, EHOSTDOWN, EHOSTUNREACH, EHWPOISON, EIDRM, EILSEQ, EINPROGRESS,
    EINTR, EINVAL, EIO, EISCONN, EISDIR, EISNAM, EKEYEXPIRED, EKEYREJECTED, EKEYREVOKED, EL2HLT,
    EL2NSYNC, EL3HLT, EL3RST, ELIBACC, ELIBBAD, ELIBEXEC, ELIBMAX, ELIBSCN, ELNRNG, ELOOP,
    EMEDIUMTYPE, EMFILE, EMLINK, EMSGSIZE, EMULTIHOP, ENAMETOOLONG, ENAVAIL, ENETDOWN, ENETRESET,
    ENETUNREACH, ENFILE, ENOANO, ENOBUFS, ENOCSI, ENODATA, ENODEV, ENOENT, ENOEXEC, ENOKEY, ENOLCK,
    ENOLINK, ENOMEDIUM, ENOMEM, ENOMSG, ENONET, ENOPKG, ENOPROTOOPT, ENOSPC, ENOSR, ENOSTR, ENOSYS,
    ENOTBLK, ENOTCONN, ENOTDIR, ENOTEMPTY, ENOTNAM, ENOTRECOVERABLE, ENOTSOCK, ENOTSUP, ENOTTY,
    ENOTUNIQ, ENXIO, EOPNOTSUPP, EOVERFLOW, EOWNERDEAD, EPERM, EPFNOSUPPORT, EPIPE, EPROTO,
    EPROTONOSUPPORT, EPROTOTYPE, ERANGE, EREMCHG, EREMOTE, EREMOTEIO, ERESTART, ERFKILL, EROFS,
    ESHUTDOWN, ESOCKTNOSUPPORT, ESPIPE, ESRCH, ESRMNT, ESTALE, ESTRPIPE, ETIME, ETIMEDOUT,
    ETOOMANYREFS, ETXTBSY, EUCLEAN, EUNATCH, EUSERS, EWOULDBLOCK, EXDEV, EXFULL,
};

static ERRNO: GuestCell<Cell<i32>> = GuestCell::new(Cell::new(0));

/// The `errno` value of the guest, which [`io::Error::last_os_error`] reads.
#[must_use]
pub fn get() -> i32 {
    ERRNO.get()
}

/// Sets the `errno` value of the guest.
pub fn set(code: i32) {
    ERRNO.set(code);
}

/// The `strerror(3)` text of `code`.
#[must_use]
pub fn strerror(code: i32) -> Option<&'static str> {
    io::strerror(code)
}

/// The errno number of `error`: its own number, or the canonical number of its kind.
#[must_use]
pub fn code_of(error: &io::Error) -> i32 {
    error
        .raw_os_error()
        .unwrap_or_else(|| io::kind_errno(error.kind()))
}

/// The [`io::ErrorKind`] `code` decodes to.
#[must_use]
pub fn kind_of(code: i32) -> io::ErrorKind {
    io::decode_error_kind(code)
}
