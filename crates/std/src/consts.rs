//! Every constant of the crate: Linux errno numbering, `open(2)` flags, `stat(2)` mode bits,
//! `poll(2)` events, buffer capacities and the guest identity.
//!
//! The numbers follow the Linux generic ABI, the numbering the embedder [`Host`] speaks.
//!
//! [`Host`]: crate::host::Host

use crate::io::ErrorKind;

pub const OS: &str = "polyasm";
pub const ARCH: &str = "polyasm";
pub const FAMILY: &str = "wasm";
pub const DLL_PREFIX: &str = "";
pub const DLL_SUFFIX: &str = ".poly";
pub const DLL_EXTENSION: &str = "poly";
pub const EXE_SUFFIX: &str = ".poly";
pub const EXE_EXTENSION: &str = "poly";

/// Process id the guest reports for itself.
pub(crate) const PROCESS_ID: u32 = 1;
/// Name of the one guest thread.
pub(crate) const MAIN_THREAD_NAME: &str = "main";
/// Default directory of [`crate::env::temp_dir`]; `TMPDIR` overrides it.
pub(crate) const TEMP_DIR: &[u8] = b"/tmp";

/// Capacity of [`crate::io::BufReader`] and [`crate::io::BufWriter`].
pub(crate) const DEFAULT_BUF_SIZE: usize = 8 * 1024;
/// Capacity of [`crate::io::LineWriter`].
pub(crate) const LINE_WRITER_CAPACITY: usize = 1024;
/// Largest byte count one `read(2)` or `write(2)` carries.
pub(crate) const MAX_RW: usize = isize::MAX as usize;
/// The keys `RandomState` uses before the embedder installs a host.
pub(crate) const FALLBACK_KEYS: (u64, u64) = (0x736f_6d65_7073_6575, 0x646f_7261_6e64_6f6d);

pub(crate) const STDIN_FILENO: i32 = 0;
pub(crate) const STDOUT_FILENO: i32 = 1;
pub(crate) const STDERR_FILENO: i32 = 2;

pub const MAIN_SEPARATOR: char = '/';
pub const MAIN_SEPARATOR_STR: &str = "/";
pub(crate) const PATH_LIST_SEPARATOR: u8 = b':';

pub const EPERM: i32 = 1;
pub const ENOENT: i32 = 2;
pub const ESRCH: i32 = 3;
pub const EINTR: i32 = 4;
pub const EIO: i32 = 5;
pub const ENXIO: i32 = 6;
pub const E2BIG: i32 = 7;
pub const ENOEXEC: i32 = 8;
pub const EBADF: i32 = 9;
pub const ECHILD: i32 = 10;
pub const EAGAIN: i32 = 11;
pub const ENOMEM: i32 = 12;
pub const EACCES: i32 = 13;
pub const EFAULT: i32 = 14;
pub const ENOTBLK: i32 = 15;
pub const EBUSY: i32 = 16;
pub const EEXIST: i32 = 17;
pub const EXDEV: i32 = 18;
pub const ENODEV: i32 = 19;
pub const ENOTDIR: i32 = 20;
pub const EISDIR: i32 = 21;
pub const EINVAL: i32 = 22;
pub const ENFILE: i32 = 23;
pub const EMFILE: i32 = 24;
pub const ENOTTY: i32 = 25;
pub const ETXTBSY: i32 = 26;
pub const EFBIG: i32 = 27;
pub const ENOSPC: i32 = 28;
pub const ESPIPE: i32 = 29;
pub const EROFS: i32 = 30;
pub const EMLINK: i32 = 31;
pub const EPIPE: i32 = 32;
pub const EDOM: i32 = 33;
pub const ERANGE: i32 = 34;
pub const EDEADLK: i32 = 35;
pub const ENAMETOOLONG: i32 = 36;
pub const ENOLCK: i32 = 37;
pub const ENOSYS: i32 = 38;
pub const ENOTEMPTY: i32 = 39;
pub const ELOOP: i32 = 40;
pub const EWOULDBLOCK: i32 = EAGAIN;
pub const ENOMSG: i32 = 42;
pub const EIDRM: i32 = 43;
pub const ECHRNG: i32 = 44;
pub const EL2NSYNC: i32 = 45;
pub const EL3HLT: i32 = 46;
pub const EL3RST: i32 = 47;
pub const ELNRNG: i32 = 48;
pub const EUNATCH: i32 = 49;
pub const ENOCSI: i32 = 50;
pub const EL2HLT: i32 = 51;
pub const EBADE: i32 = 52;
pub const EBADR: i32 = 53;
pub const EXFULL: i32 = 54;
pub const ENOANO: i32 = 55;
pub const EBADRQC: i32 = 56;
pub const EBADSLT: i32 = 57;
pub const EDEADLOCK: i32 = EDEADLK;
pub const EBFONT: i32 = 59;
pub const ENOSTR: i32 = 60;
pub const ENODATA: i32 = 61;
pub const ETIME: i32 = 62;
pub const ENOSR: i32 = 63;
pub const ENONET: i32 = 64;
pub const ENOPKG: i32 = 65;
pub const EREMOTE: i32 = 66;
pub const ENOLINK: i32 = 67;
pub const EADV: i32 = 68;
pub const ESRMNT: i32 = 69;
pub const ECOMM: i32 = 70;
pub const EPROTO: i32 = 71;
pub const EMULTIHOP: i32 = 72;
pub const EDOTDOT: i32 = 73;
pub const EBADMSG: i32 = 74;
pub const EOVERFLOW: i32 = 75;
pub const ENOTUNIQ: i32 = 76;
pub const EBADFD: i32 = 77;
pub const EREMCHG: i32 = 78;
pub const ELIBACC: i32 = 79;
pub const ELIBBAD: i32 = 80;
pub const ELIBSCN: i32 = 81;
pub const ELIBMAX: i32 = 82;
pub const ELIBEXEC: i32 = 83;
pub const EILSEQ: i32 = 84;
pub const ERESTART: i32 = 85;
pub const ESTRPIPE: i32 = 86;
pub const EUSERS: i32 = 87;
pub const ENOTSOCK: i32 = 88;
pub const EDESTADDRREQ: i32 = 89;
pub const EMSGSIZE: i32 = 90;
pub const EPROTOTYPE: i32 = 91;
pub const ENOPROTOOPT: i32 = 92;
pub const EPROTONOSUPPORT: i32 = 93;
pub const ESOCKTNOSUPPORT: i32 = 94;
pub const EOPNOTSUPP: i32 = 95;
pub const ENOTSUP: i32 = EOPNOTSUPP;
pub const EPFNOSUPPORT: i32 = 96;
pub const EAFNOSUPPORT: i32 = 97;
pub const EADDRINUSE: i32 = 98;
pub const EADDRNOTAVAIL: i32 = 99;
pub const ENETDOWN: i32 = 100;
pub const ENETUNREACH: i32 = 101;
pub const ENETRESET: i32 = 102;
pub const ECONNABORTED: i32 = 103;
pub const ECONNRESET: i32 = 104;
pub const ENOBUFS: i32 = 105;
pub const EISCONN: i32 = 106;
pub const ENOTCONN: i32 = 107;
pub const ESHUTDOWN: i32 = 108;
pub const ETOOMANYREFS: i32 = 109;
pub const ETIMEDOUT: i32 = 110;
pub const ECONNREFUSED: i32 = 111;
pub const EHOSTDOWN: i32 = 112;
pub const EHOSTUNREACH: i32 = 113;
pub const EALREADY: i32 = 114;
pub const EINPROGRESS: i32 = 115;
pub const ESTALE: i32 = 116;
pub const EUCLEAN: i32 = 117;
pub const ENOTNAM: i32 = 118;
pub const ENAVAIL: i32 = 119;
pub const EISNAM: i32 = 120;
pub const EREMOTEIO: i32 = 121;
pub const EDQUOT: i32 = 122;
pub const ENOMEDIUM: i32 = 123;
pub const EMEDIUMTYPE: i32 = 124;
pub const ECANCELED: i32 = 125;
pub const ENOKEY: i32 = 126;
pub const EKEYEXPIRED: i32 = 127;
pub const EKEYREVOKED: i32 = 128;
pub const EKEYREJECTED: i32 = 129;
pub const EOWNERDEAD: i32 = 130;
pub const ENOTRECOVERABLE: i32 = 131;
pub const ERFKILL: i32 = 132;
pub const EHWPOISON: i32 = 133;

/// One Linux errno number with its `strerror(3)` text and the [`ErrorKind`] it decodes to.
pub(crate) struct Errno {
    pub(crate) code: i32,
    pub(crate) message: &'static str,
    pub(crate) kind: ErrorKind,
}

const fn errno(code: i32, kind: ErrorKind, message: &'static str) -> Errno {
    Errno {
        code,
        message,
        kind,
    }
}

/// The errno table: number, decoded [`ErrorKind`] and `strerror(3)` text, in errno order.
///
/// A lookup by kind reads [`KIND_ERRNOS`] first and then the first row of that kind here.
#[rustfmt::skip]
pub(crate) const ERRNO_TABLE: &[Errno] = &[
    errno(EPERM, ErrorKind::PermissionDenied, "Operation not permitted"),
    errno(ENOENT, ErrorKind::NotFound, "No such file or directory"),
    errno(ESRCH, ErrorKind::Uncategorized, "No such process"),
    errno(EINTR, ErrorKind::Interrupted, "Interrupted system call"),
    errno(EIO, ErrorKind::Uncategorized, "Input/output error"),
    errno(ENXIO, ErrorKind::Uncategorized, "No such device or address"),
    errno(E2BIG, ErrorKind::ArgumentListTooLong, "Argument list too long"),
    errno(ENOEXEC, ErrorKind::Uncategorized, "Exec format error"),
    errno(EBADF, ErrorKind::Uncategorized, "Bad file descriptor"),
    errno(ECHILD, ErrorKind::Uncategorized, "No child processes"),
    errno(EAGAIN, ErrorKind::WouldBlock, "Resource temporarily unavailable"),
    errno(ENOMEM, ErrorKind::OutOfMemory, "Cannot allocate memory"),
    errno(EACCES, ErrorKind::PermissionDenied, "Permission denied"),
    errno(EFAULT, ErrorKind::Uncategorized, "Bad address"),
    errno(ENOTBLK, ErrorKind::Uncategorized, "Block device required"),
    errno(EBUSY, ErrorKind::ResourceBusy, "Device or resource busy"),
    errno(EEXIST, ErrorKind::AlreadyExists, "File exists"),
    errno(EXDEV, ErrorKind::CrossesDevices, "Invalid cross-device link"),
    errno(ENODEV, ErrorKind::Uncategorized, "No such device"),
    errno(ENOTDIR, ErrorKind::NotADirectory, "Not a directory"),
    errno(EISDIR, ErrorKind::IsADirectory, "Is a directory"),
    errno(EINVAL, ErrorKind::InvalidInput, "Invalid argument"),
    errno(ENFILE, ErrorKind::Uncategorized, "Too many open files in system"),
    errno(EMFILE, ErrorKind::Uncategorized, "Too many open files"),
    errno(ENOTTY, ErrorKind::Uncategorized, "Inappropriate ioctl for device"),
    errno(ETXTBSY, ErrorKind::ExecutableFileBusy, "Text file busy"),
    errno(EFBIG, ErrorKind::FileTooLarge, "File too large"),
    errno(ENOSPC, ErrorKind::StorageFull, "No space left on device"),
    errno(ESPIPE, ErrorKind::NotSeekable, "Illegal seek"),
    errno(EROFS, ErrorKind::ReadOnlyFilesystem, "Read-only file system"),
    errno(EMLINK, ErrorKind::TooManyLinks, "Too many links"),
    errno(EPIPE, ErrorKind::BrokenPipe, "Broken pipe"),
    errno(EDOM, ErrorKind::Uncategorized, "Numerical argument out of domain"),
    errno(ERANGE, ErrorKind::Uncategorized, "Numerical result out of range"),
    errno(EDEADLK, ErrorKind::Deadlock, "Resource deadlock avoided"),
    errno(ENAMETOOLONG, ErrorKind::InvalidFilename, "File name too long"),
    errno(ENOLCK, ErrorKind::Uncategorized, "No locks available"),
    errno(ENOSYS, ErrorKind::Unsupported, "Function not implemented"),
    errno(ENOTEMPTY, ErrorKind::DirectoryNotEmpty, "Directory not empty"),
    errno(ELOOP, ErrorKind::FilesystemLoop, "Too many levels of symbolic links"),
    errno(ENOMSG, ErrorKind::Uncategorized, "No message of desired type"),
    errno(EIDRM, ErrorKind::Uncategorized, "Identifier removed"),
    errno(ECHRNG, ErrorKind::Uncategorized, "Channel number out of range"),
    errno(EL2NSYNC, ErrorKind::Uncategorized, "Level 2 not synchronized"),
    errno(EL3HLT, ErrorKind::Uncategorized, "Level 3 halted"),
    errno(EL3RST, ErrorKind::Uncategorized, "Level 3 reset"),
    errno(ELNRNG, ErrorKind::Uncategorized, "Link number out of range"),
    errno(EUNATCH, ErrorKind::Uncategorized, "Protocol driver not attached"),
    errno(ENOCSI, ErrorKind::Uncategorized, "No CSI structure available"),
    errno(EL2HLT, ErrorKind::Uncategorized, "Level 2 halted"),
    errno(EBADE, ErrorKind::Uncategorized, "Invalid exchange"),
    errno(EBADR, ErrorKind::Uncategorized, "Invalid request descriptor"),
    errno(EXFULL, ErrorKind::Uncategorized, "Exchange full"),
    errno(ENOANO, ErrorKind::Uncategorized, "No anode"),
    errno(EBADRQC, ErrorKind::Uncategorized, "Invalid request code"),
    errno(EBADSLT, ErrorKind::Uncategorized, "Invalid slot"),
    errno(EBFONT, ErrorKind::Uncategorized, "Bad font file format"),
    errno(ENOSTR, ErrorKind::Uncategorized, "Device not a stream"),
    errno(ENODATA, ErrorKind::Uncategorized, "No data available"),
    errno(ETIME, ErrorKind::Uncategorized, "Timer expired"),
    errno(ENOSR, ErrorKind::Uncategorized, "Out of streams resources"),
    errno(ENONET, ErrorKind::Uncategorized, "Machine is not on the network"),
    errno(ENOPKG, ErrorKind::Uncategorized, "Package not installed"),
    errno(EREMOTE, ErrorKind::Uncategorized, "Object is remote"),
    errno(ENOLINK, ErrorKind::Uncategorized, "Link has been severed"),
    errno(EADV, ErrorKind::Uncategorized, "Advertise error"),
    errno(ESRMNT, ErrorKind::Uncategorized, "Srmount error"),
    errno(ECOMM, ErrorKind::Uncategorized, "Communication error on send"),
    errno(EPROTO, ErrorKind::Uncategorized, "Protocol error"),
    errno(EMULTIHOP, ErrorKind::Uncategorized, "Multihop attempted"),
    errno(EDOTDOT, ErrorKind::Uncategorized, "RFS specific error"),
    errno(EBADMSG, ErrorKind::Uncategorized, "Bad message"),
    errno(EOVERFLOW, ErrorKind::Uncategorized, "Value too large for defined data type"),
    errno(ENOTUNIQ, ErrorKind::Uncategorized, "Name not unique on network"),
    errno(EBADFD, ErrorKind::Uncategorized, "File descriptor in bad state"),
    errno(EREMCHG, ErrorKind::Uncategorized, "Remote address changed"),
    errno(ELIBACC, ErrorKind::Uncategorized, "Can not access a needed shared library"),
    errno(ELIBBAD, ErrorKind::Uncategorized, "Accessing a corrupted shared library"),
    errno(ELIBSCN, ErrorKind::Uncategorized, ".lib section in a.out corrupted"),
    errno(ELIBMAX, ErrorKind::Uncategorized, "Attempting to link in too many shared libraries"),
    errno(ELIBEXEC, ErrorKind::Uncategorized, "Cannot exec a shared library directly"),
    errno(EILSEQ, ErrorKind::Uncategorized, "Invalid or incomplete multibyte or wide character"),
    errno(ERESTART, ErrorKind::Uncategorized, "Interrupted system call should be restarted"),
    errno(ESTRPIPE, ErrorKind::Uncategorized, "Streams pipe error"),
    errno(EUSERS, ErrorKind::Uncategorized, "Too many users"),
    errno(ENOTSOCK, ErrorKind::Uncategorized, "Socket operation on non-socket"),
    errno(EDESTADDRREQ, ErrorKind::Uncategorized, "Destination address required"),
    errno(EMSGSIZE, ErrorKind::Uncategorized, "Message too long"),
    errno(EPROTOTYPE, ErrorKind::Uncategorized, "Protocol wrong type for socket"),
    errno(ENOPROTOOPT, ErrorKind::Uncategorized, "Protocol not available"),
    errno(EPROTONOSUPPORT, ErrorKind::Uncategorized, "Protocol not supported"),
    errno(ESOCKTNOSUPPORT, ErrorKind::Uncategorized, "Socket type not supported"),
    errno(EOPNOTSUPP, ErrorKind::Uncategorized, "Operation not supported"),
    errno(EPFNOSUPPORT, ErrorKind::Uncategorized, "Protocol family not supported"),
    errno(EAFNOSUPPORT, ErrorKind::Uncategorized, "Address family not supported by protocol"),
    errno(EADDRINUSE, ErrorKind::AddrInUse, "Address already in use"),
    errno(EADDRNOTAVAIL, ErrorKind::AddrNotAvailable, "Cannot assign requested address"),
    errno(ENETDOWN, ErrorKind::NetworkDown, "Network is down"),
    errno(ENETUNREACH, ErrorKind::NetworkUnreachable, "Network is unreachable"),
    errno(ENETRESET, ErrorKind::Uncategorized, "Network dropped connection on reset"),
    errno(ECONNABORTED, ErrorKind::ConnectionAborted, "Software caused connection abort"),
    errno(ECONNRESET, ErrorKind::ConnectionReset, "Connection reset by peer"),
    errno(ENOBUFS, ErrorKind::Uncategorized, "No buffer space available"),
    errno(EISCONN, ErrorKind::Uncategorized, "Transport endpoint is already connected"),
    errno(ENOTCONN, ErrorKind::NotConnected, "Transport endpoint is not connected"),
    errno(ESHUTDOWN, ErrorKind::Uncategorized, "Cannot send after transport endpoint shutdown"),
    errno(ETOOMANYREFS, ErrorKind::Uncategorized, "Too many references: cannot splice"),
    errno(ETIMEDOUT, ErrorKind::TimedOut, "Connection timed out"),
    errno(ECONNREFUSED, ErrorKind::ConnectionRefused, "Connection refused"),
    errno(EHOSTDOWN, ErrorKind::Uncategorized, "Host is down"),
    errno(EHOSTUNREACH, ErrorKind::HostUnreachable, "No route to host"),
    errno(EALREADY, ErrorKind::Uncategorized, "Operation already in progress"),
    errno(EINPROGRESS, ErrorKind::InProgress, "Operation now in progress"),
    errno(ESTALE, ErrorKind::StaleNetworkFileHandle, "Stale file handle"),
    errno(EUCLEAN, ErrorKind::Uncategorized, "Structure needs cleaning"),
    errno(ENOTNAM, ErrorKind::Uncategorized, "Not a XENIX named type file"),
    errno(ENAVAIL, ErrorKind::Uncategorized, "No XENIX semaphores available"),
    errno(EISNAM, ErrorKind::Uncategorized, "Is a named type file"),
    errno(EREMOTEIO, ErrorKind::Uncategorized, "Remote I/O error"),
    errno(EDQUOT, ErrorKind::QuotaExceeded, "Disk quota exceeded"),
    errno(ENOMEDIUM, ErrorKind::Uncategorized, "No medium found"),
    errno(EMEDIUMTYPE, ErrorKind::Uncategorized, "Wrong medium type"),
    errno(ECANCELED, ErrorKind::Uncategorized, "Operation canceled"),
    errno(ENOKEY, ErrorKind::Uncategorized, "Required key not available"),
    errno(EKEYEXPIRED, ErrorKind::Uncategorized, "Key has expired"),
    errno(EKEYREVOKED, ErrorKind::Uncategorized, "Key has been revoked"),
    errno(EKEYREJECTED, ErrorKind::Uncategorized, "Key was rejected by service"),
    errno(EOWNERDEAD, ErrorKind::Uncategorized, "Owner died"),
    errno(ENOTRECOVERABLE, ErrorKind::Uncategorized, "State not recoverable"),
    errno(ERFKILL, ErrorKind::Uncategorized, "Operation not possible due to RF-kill"),
    errno(EHWPOISON, ErrorKind::Uncategorized, "Memory page has hardware error"),
];

/// Canonical errno numbers of the kinds whose errno differs from their first [`ERRNO_TABLE`] row.
#[rustfmt::skip]
pub(crate) const KIND_ERRNOS: &[(ErrorKind, i32)] = &[
    (ErrorKind::InvalidData, EINVAL),
    (ErrorKind::PermissionDenied, EACCES),
    (ErrorKind::Other, EIO),
    (ErrorKind::UnexpectedEof, EIO),
    (ErrorKind::Uncategorized, EIO),
    (ErrorKind::WriteZero, EIO),
];

pub const O_RDONLY: i32 = 0;
pub const O_WRONLY: i32 = 0o1;
pub const O_RDWR: i32 = 0o2;
pub const O_ACCMODE: i32 = 0o3;
pub const O_CREAT: i32 = 0o100;
pub const O_EXCL: i32 = 0o200;
pub const O_NOCTTY: i32 = 0o400;
pub const O_TRUNC: i32 = 0o1000;
pub const O_APPEND: i32 = 0o2000;
pub const O_NONBLOCK: i32 = 0o4000;
pub const O_NDELAY: i32 = O_NONBLOCK;
pub const O_DSYNC: i32 = 0o10000;
pub const O_ASYNC: i32 = 0o20000;
pub const O_DIRECTORY: i32 = 0o200000;
pub const O_NOFOLLOW: i32 = 0o400000;
pub const O_CLOEXEC: i32 = 0o2000000;
pub const O_SYNC: i32 = 0o4010000;

pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;
pub const AT_FDCWD: i32 = -100;

pub const S_IFMT: u32 = 0o170000;
pub const S_IFSOCK: u32 = 0o140000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFBLK: u32 = 0o060000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFIFO: u32 = 0o010000;
pub const S_ISUID: u32 = 0o4000;
pub const S_ISGID: u32 = 0o2000;
pub const S_ISVTX: u32 = 0o1000;
pub const S_IRWXU: u32 = 0o700;
pub const S_IRUSR: u32 = 0o400;
pub const S_IWUSR: u32 = 0o200;
pub const S_IXUSR: u32 = 0o100;
pub const S_IRWXG: u32 = 0o070;
pub const S_IRGRP: u32 = 0o040;
pub const S_IWGRP: u32 = 0o020;
pub const S_IXGRP: u32 = 0o010;
pub const S_IRWXO: u32 = 0o007;
pub const S_IROTH: u32 = 0o004;
pub const S_IWOTH: u32 = 0o002;
pub const S_IXOTH: u32 = 0o001;
/// Write bits of every class, the bits [`crate::fs::Permissions::readonly`] reads.
pub(crate) const S_IWALL: u32 = S_IWUSR | S_IWGRP | S_IWOTH;
/// Mode [`crate::fs::OpenOptions`] creates files with.
pub(crate) const DEFAULT_FILE_MODE: u32 = 0o666;
/// Mode [`crate::fs::create_dir`] creates directories with.
pub(crate) const DEFAULT_DIR_MODE: u32 = 0o777;
/// Block size [`crate::os::polyasm::fs::MetadataExt::blksize`] reports.
pub(crate) const BLOCK_SIZE: u64 = 4096;
/// Unit of [`crate::os::polyasm::fs::MetadataExt::blocks`].
pub(crate) const STAT_BLOCK_UNIT: u64 = 512;

pub const POLLIN: i16 = 0x001;
pub const POLLPRI: i16 = 0x002;
pub const POLLOUT: i16 = 0x004;
pub const POLLERR: i16 = 0x008;
pub const POLLHUP: i16 = 0x010;
pub const POLLNVAL: i16 = 0x020;
pub const POLLRDNORM: i16 = 0x040;
pub const POLLRDBAND: i16 = 0x080;
pub const POLLWRNORM: i16 = 0x100;
pub const POLLWRBAND: i16 = 0x200;

pub(crate) const NANOS_PER_SEC: i64 = 1_000_000_000;
