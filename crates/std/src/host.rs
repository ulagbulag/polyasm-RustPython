//! The embedder seam: every operating-system service a PolyASM guest reaches.
//!
//! A PolyASM guest carries `core` and `alloc` alone. The embedder implements [`Host`]
//! over its own services (on wasm-direct: the eDMA rings of wasm-direct-core and a
//! wasm-direct-fs store) and calls [`install`] before RustPython runs. Files, standard streams,
//! clocks, entropy, sockets and the environment of this crate all call the installed host.
//!
//! # Rules every method follows
//!
//! - Every method is synchronous: it returns once its operation completes.
//! - A failure answers `io::Error::from_raw_os_error(errno)` with a Linux errno number from
//!   [`crate::os::polyasm::errno`]. An error of another shape reaches Python as the canonical
//!   errno of its [`io::ErrorKind`].
//! - Paths are bytes, absolute or relative to the directory [`Host::getcwd`] answers, with `/`
//!   between components.
//! - Descriptors are small integers from 0 up. Descriptors 0, 1 and 2 are standard input,
//!   standard output and standard error from the start.
//! - `open` flags, `whence`, mode bits and poll events follow the Linux generic ABI, exported
//!   under [`crate::os::polyasm`].
//! - The guest runs one thread, so the host serves one call at a time.

use alloc_crate::vec::Vec;
use core::{cell::Cell, net::SocketAddr, time::Duration};

use crate::{
    consts::{EINVAL, ENOSYS},
    io,
    net::{IpAddr, Shutdown},
    os::fd::RawFd,
    sys::GuestCell,
};

/// The status of a file, as `stat(2)` reports it.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Stat {
    /// File type (`S_IF*`) and permission bits.
    pub mode: u32,
    /// Length in bytes.
    pub size: u64,
    /// Last access, in nanoseconds since the Unix epoch.
    pub atime_ns: i64,
    /// Last modification, in nanoseconds since the Unix epoch.
    pub mtime_ns: i64,
    /// Last status change, in nanoseconds since the Unix epoch.
    pub ctime_ns: i64,
    /// Inode number, unique per file within [`Stat::dev`].
    pub ino: u64,
    /// Device number of the file system holding the file.
    pub dev: u64,
    /// Number of hard links.
    pub nlink: u64,
    /// Owner user id.
    pub uid: u32,
    /// Owner group id.
    pub gid: u32,
}

/// One descriptor of a [`Host::poll`] call, laid out as `struct pollfd`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct PollFd {
    /// The descriptor to watch; a negative one stays out of the wait and keeps `revents` at 0.
    pub fd: RawFd,
    /// Requested `POLL*` events.
    pub events: i16,
    /// Events the host reports: requested ones that are ready, plus `POLLHUP`, `POLLERR` and
    /// `POLLNVAL` whenever they hold.
    pub revents: i16,
}

impl PollFd {
    #[must_use]
    pub const fn new(fd: RawFd, events: i16) -> Self {
        Self {
            fd,
            events,
            revents: 0,
        }
    }
}

/// The address of a socket.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SockAddr {
    /// An IPv4 or IPv6 address with its port.
    Inet(SocketAddr),
    /// A Unix domain socket path; empty for an unnamed socket.
    Unix(Vec<u8>),
}

/// The kind of a socket.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SocketKind {
    /// A connected byte stream: TCP, or a Unix stream socket.
    Stream,
    /// Datagrams: UDP, or a Unix datagram socket.
    Datagram,
}

const fn enosys<T>() -> io::Result<T> {
    Err(io::Error::from_raw_os_error(ENOSYS))
}

/// The operating-system services of a PolyASM guest, implemented by the embedder.
///
/// See the [module documentation](self) for the rules every method follows.
pub trait Host: Sync {
    /// The program arguments, `argv[0]` first.
    fn argv(&self) -> Vec<Vec<u8>>;

    /// The initial environment as `(name, value)` pairs. The guest reads it once and keeps its
    /// own changes to itself.
    fn environ(&self) -> Vec<(Vec<u8>, Vec<u8>)>;

    /// Ends the guest with exit status `code`.
    fn exit(&self, code: i32) -> !;

    /// Opens `path`, answering a new descriptor.
    ///
    /// `flags` holds the access mode (`O_RDONLY`, `O_WRONLY`, `O_RDWR`) and any of `O_CREAT`,
    /// `O_EXCL`, `O_TRUNC`, `O_APPEND`, `O_DIRECTORY`, `O_NONBLOCK` and `O_CLOEXEC`. A file
    /// `O_CREAT` brings into being takes the permission bits of `mode`. `O_DIRECTORY` opens a
    /// directory, which serves [`Host::fstat`] and [`Host::close`].
    fn open(&self, path: &[u8], flags: i32, mode: u32) -> io::Result<RawFd>;

    /// Closes `fd`. Bytes written through it reach storage first.
    fn close(&self, fd: RawFd) -> io::Result<()>;

    /// Reads up to `buf.len()` bytes at the position of `fd`, answering the count.
    ///
    /// 0 marks the end of the file or a closed peer. A blocking descriptor waits for data; one
    /// set to `O_NONBLOCK` answers `EAGAIN` while the data is pending.
    fn read(&self, fd: RawFd, buf: &mut [u8]) -> io::Result<usize>;

    /// Writes bytes of `buf` at the position of `fd`, or at its end under `O_APPEND`,
    /// answering the count.
    fn write(&self, fd: RawFd, buf: &[u8]) -> io::Result<usize>;

    /// Moves the position of `fd` by `offset` from `whence` (`SEEK_SET`, `SEEK_CUR`,
    /// `SEEK_END`), answering the new position. Pipes and sockets answer `ESPIPE`.
    fn seek(&self, fd: RawFd, offset: i64, whence: i32) -> io::Result<u64>;

    /// The status of the object behind `fd`. A pipe reports `S_IFIFO`, a socket `S_IFSOCK`.
    fn fstat(&self, fd: RawFd) -> io::Result<Stat>;

    /// Sets the length of the file behind `fd`, zero-filling any growth.
    fn ftruncate(&self, fd: RawFd, len: u64) -> io::Result<()>;

    /// Delivers the bytes written through `fd` to storage.
    fn fsync(&self, fd: RawFd) -> io::Result<()>;

    /// Whether `fd` is a terminal. The default answers `false`.
    fn isatty(&self, fd: RawFd) -> bool {
        let _ = fd;
        false
    }

    /// Sets whether reads and writes on `fd` wait (`blocking`) or answer `EAGAIN` at once.
    fn set_blocking(&self, fd: RawFd, blocking: bool) -> io::Result<()>;

    /// Waits until a descriptor of `fds` is ready for its requested events, or until `timeout`
    /// passes: `None` waits for readiness alone and zero answers at once.
    ///
    /// Fills every `revents` and answers the count of entries whose `revents` carry events.
    fn poll(&self, fds: &mut [PollFd], timeout: Option<Duration>) -> io::Result<usize>;

    /// The status of `path`, following symbolic links.
    fn stat(&self, path: &[u8]) -> io::Result<Stat>;

    /// The status of `path` itself. The default answers [`Host::stat`], for hosts whose files
    /// are regular files and directories alone.
    fn lstat(&self, path: &[u8]) -> io::Result<Stat> {
        self.stat(path)
    }

    /// The entry names of the directory `path`, `.` and `..` left out.
    fn listdir(&self, path: &[u8]) -> io::Result<Vec<Vec<u8>>>;

    /// Creates the directory `path` with permission bits `mode`. Its parent exists already.
    fn mkdir(&self, path: &[u8], mode: u32) -> io::Result<()>;

    /// Removes the empty directory `path`.
    fn rmdir(&self, path: &[u8]) -> io::Result<()>;

    /// Removes the file `path`.
    fn unlink(&self, path: &[u8]) -> io::Result<()>;

    /// Moves `from` to `to`, replacing a file at `to`.
    fn rename(&self, from: &[u8], to: &[u8]) -> io::Result<()>;

    /// The target of the symbolic link `path`. The default answers `EINVAL`, the answer for a
    /// path that holds a regular file or a directory.
    fn readlink(&self, path: &[u8]) -> io::Result<Vec<u8>> {
        let _ = path;
        Err(io::Error::from_raw_os_error(EINVAL))
    }

    /// The absolute path of the working directory.
    fn getcwd(&self) -> io::Result<Vec<u8>>;

    /// Makes the directory `path` the working directory.
    fn chdir(&self, path: &[u8]) -> io::Result<()>;

    /// The wall-clock time since the Unix epoch.
    fn wall_clock(&self) -> io::Result<Duration>;

    /// A clock that only advances, from an arbitrary origin.
    fn monotonic(&self) -> io::Result<Duration>;

    /// Suspends the guest for `duration`.
    fn sleep(&self, duration: Duration) -> io::Result<()>;

    /// Fills `buf` with cryptographically secure random bytes.
    fn fill_random(&self, buf: &mut [u8]) -> io::Result<()>;

    /// Opens a socket of `kind` connected to `addr`, answering its descriptor.
    ///
    /// `timeout` bounds the wait for the connection: `None` waits as long as the connection
    /// takes. The default answers `ENOSYS`, for hosts that keep to local services.
    fn connect(
        &self,
        kind: SocketKind,
        addr: &SockAddr,
        timeout: Option<Duration>,
    ) -> io::Result<RawFd> {
        let _ = (kind, addr, timeout);
        enosys()
    }

    /// Opens a socket of `kind` bound to `addr`, answering its descriptor. Port 0 of an
    /// [`SockAddr::Inet`] picks a free port, which [`Host::local_addr`] reports. The default
    /// answers `ENOSYS`.
    fn bind(&self, kind: SocketKind, addr: &SockAddr) -> io::Result<RawFd> {
        let _ = (kind, addr);
        enosys()
    }

    /// Starts accepting connections on the bound stream socket `fd`. The default answers
    /// `ENOSYS`.
    fn listen(&self, fd: RawFd, backlog: i32) -> io::Result<()> {
        let _ = (fd, backlog);
        enosys()
    }

    /// Takes the next connection of the listening socket `fd`, answering its descriptor and
    /// peer address. The default answers `ENOSYS`.
    fn accept(&self, fd: RawFd) -> io::Result<(RawFd, SockAddr)> {
        let _ = fd;
        enosys()
    }

    /// Sends bytes of `buf` on the connected socket `fd`, answering the count. `flags` holds
    /// Linux `MSG_*` bits. The default answers `ENOSYS`.
    fn send(&self, fd: RawFd, buf: &[u8], flags: i32) -> io::Result<usize> {
        let _ = (fd, buf, flags);
        enosys()
    }

    /// Receives up to `buf.len()` bytes from the connected socket `fd`, answering the count;
    /// 0 marks a closed peer. `flags` holds Linux `MSG_*` bits: `MSG_PEEK` (`0x2`) leaves the
    /// bytes queued and `MSG_DONTWAIT` (`0x40`) answers `EAGAIN` while the bytes are pending.
    /// The default answers `ENOSYS`.
    fn recv(&self, fd: RawFd, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        let _ = (fd, buf, flags);
        enosys()
    }

    /// Shuts down the reading half, the writing half or both halves of the socket `fd`. The
    /// default answers `ENOSYS`.
    fn shutdown(&self, fd: RawFd, how: Shutdown) -> io::Result<()> {
        let _ = (fd, how);
        enosys()
    }

    /// Opens two connected stream sockets. The default answers `ENOSYS`.
    fn socketpair(&self) -> io::Result<(RawFd, RawFd)> {
        enosys()
    }

    /// The local address of the socket `fd`. The default answers `ENOSYS`.
    fn local_addr(&self, fd: RawFd) -> io::Result<SockAddr> {
        let _ = fd;
        enosys()
    }

    /// The peer address of the connected socket `fd`. The default answers `ENOSYS`.
    fn peer_addr(&self, fd: RawFd) -> io::Result<SockAddr> {
        let _ = fd;
        enosys()
    }

    /// The addresses of the host name `host`. Numeric addresses and `localhost` resolve in the
    /// guest before this call. The default answers `ENOSYS`.
    fn resolve(&self, host: &[u8]) -> io::Result<Vec<IpAddr>> {
        let _ = host;
        enosys()
    }
}

static HOST: GuestCell<Cell<Option<&'static dyn Host>>> = GuestCell::new(Cell::new(None));

/// Installs the host every operating-system call of the guest reaches. A later call replaces
/// the host an earlier call installed.
pub fn install(host: &'static dyn Host) {
    HOST.set(Some(host));
}

/// The installed host, when the embedder has installed one.
#[must_use]
pub fn installed() -> Option<&'static dyn Host> {
    HOST.get()
}

/// The installed host. The guest traps with a message when this runs before [`install`].
#[must_use]
pub fn host() -> &'static dyn Host {
    installed().unwrap_or_else(|| {
        panic!("rustpython_std::host::install runs before the first operating-system call")
    })
}
