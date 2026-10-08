//! Sockets of a PolyASM guest over the `rustpython_std::host::Host` of the embedder.
//!
//! The host opens a socket already connected or bound, so a [`Socket`] keeps its family, kind,
//! protocol and timeout until [`Socket::connect`] or [`Socket::bind`] brings its descriptor into
//! being. The constants are the Linux ABI that `Lib/socket.py` expects.

use core::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};
use std::{
    host::{PollFd, SockAddr, SocketKind, host},
    io,
    net::Shutdown,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
        polyasm::{
            errno::{EAFNOSUPPORT, EINVAL, ENOTCONN, ESOCKTNOSUPPORT},
            poll::{POLLIN, POLLOUT},
        },
    },
    time::Instant,
};

use crate::consts::{HOSTNAME, LOCALHOST, SOCKET_TIMEOUT_MESSAGE};

pub use crate::consts::MSG_DONTWAIT;
pub use crate::socket_wasm::*;

/// A socket of the guest.
#[derive(Debug)]
pub struct Socket {
    family: i32,
    kind: i32,
    protocol: i32,
    /// `None` blocks; zero answers `EAGAIN` at once; any other span bounds each wait.
    timeout: Option<Duration>,
    fd: Option<OwnedFd>,
}

/// The [`SocketKind`] of a `SOCK_*` number.
pub fn socket_kind(kind: i32) -> io::Result<SocketKind> {
    match kind {
        SOCK_STREAM => Ok(SocketKind::Stream),
        SOCK_DGRAM => Ok(SocketKind::Datagram),
        _ => Err(io::Error::from_raw_os_error(ESOCKTNOSUPPORT)),
    }
}

/// The `AF_*` family of `addr`.
#[must_use]
pub const fn address_family(addr: &SockAddr) -> i32 {
    match addr {
        SockAddr::Inet(SocketAddr::V4(_)) => AF_INET,
        SockAddr::Inet(SocketAddr::V6(_)) => AF_INET6,
        SockAddr::Unix(_) => AF_UNIX,
    }
}

fn timed_out() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, SOCKET_TIMEOUT_MESSAGE)
}

impl Socket {
    /// A socket of `family`, `kind` and `protocol`, its descriptor still to come.
    pub fn new(family: i32, kind: i32, protocol: i32) -> io::Result<Self> {
        if !matches!(family, AF_INET | AF_INET6 | AF_UNIX) {
            return Err(io::Error::from_raw_os_error(EAFNOSUPPORT));
        }
        socket_kind(kind)?;
        Ok(Self {
            family,
            kind,
            protocol,
            timeout: None,
            fd: None,
        })
    }

    /// A socket around the open descriptor `fd`.
    pub const fn from_owned_fd(fd: OwnedFd, family: i32, kind: i32, protocol: i32) -> Self {
        Self {
            family,
            kind,
            protocol,
            timeout: None,
            fd: Some(fd),
        }
    }

    #[must_use]
    pub const fn family(&self) -> i32 {
        self.family
    }

    #[must_use]
    pub const fn kind(&self) -> i32 {
        self.kind
    }

    #[must_use]
    pub const fn protocol(&self) -> i32 {
        self.protocol
    }

    /// The descriptor, or `-1` before `connect` or `bind`.
    #[must_use]
    pub fn fileno(&self) -> RawFd {
        self.fd.as_ref().map_or(-1, AsRawFd::as_raw_fd)
    }

    #[must_use]
    pub const fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Sets the timeout: `None` blocks, zero answers `EAGAIN` at once, any other span bounds
    /// each wait and answers [`io::ErrorKind::TimedOut`] when it passes.
    pub fn set_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.timeout = timeout;
        self.apply_blocking()
    }

    fn apply_blocking(&self) -> io::Result<()> {
        match &self.fd {
            Some(fd) => host().set_blocking(fd.as_raw_fd(), self.timeout.is_none()),
            None => Ok(()),
        }
    }

    fn raw_fd(&self) -> io::Result<RawFd> {
        self.fd
            .as_ref()
            .map(AsRawFd::as_raw_fd)
            .ok_or_else(|| io::Error::from_raw_os_error(ENOTCONN))
    }

    fn adopt(&mut self, fd: RawFd) -> io::Result<()> {
        // SAFETY: the host answered a new descriptor this socket owns.
        self.fd = Some(unsafe { OwnedFd::from_raw_fd(fd) });
        self.apply_blocking()
    }

    /// Waits until the descriptor is ready for `events` within the timeout.
    fn wait(&self, fd: RawFd, events: i16, deadline: Option<Instant>) -> io::Result<()> {
        let Some(deadline) = deadline else {
            return Ok(());
        };
        let remaining = deadline.saturating_duration_since(Instant::now());
        let mut fds = [PollFd::new(fd, events)];
        match host().poll(&mut fds, Some(remaining))? {
            0 => Err(timed_out()),
            _ => Ok(()),
        }
    }

    fn deadline(&self) -> Option<Instant> {
        self.timeout
            .filter(|timeout| !timeout.is_zero())
            .map(|timeout| Instant::now() + timeout)
    }

    /// Runs `op` once the descriptor is ready for `events`, within the timeout.
    fn when_ready<T>(
        &self,
        events: i16,
        mut op: impl FnMut(RawFd) -> io::Result<T>,
    ) -> io::Result<T> {
        let fd = self.raw_fd()?;
        let deadline = self.deadline();
        loop {
            self.wait(fd, events, deadline)?;
            match op(fd) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock && deadline.is_some() => {}
                result => return result,
            }
        }
    }

    /// Connects to `addr`, bringing the descriptor into being.
    pub fn connect(&mut self, addr: &SockAddr) -> io::Result<()> {
        if self.fd.is_some() {
            return Err(io::Error::from_raw_os_error(EINVAL));
        }
        let kind = socket_kind(self.kind)?;
        let fd = host().connect(kind, addr, self.timeout.filter(|t| !t.is_zero()))?;
        self.adopt(fd)
    }

    /// Binds to `addr`, bringing the descriptor into being.
    pub fn bind(&mut self, addr: &SockAddr) -> io::Result<()> {
        if self.fd.is_some() {
            return Err(io::Error::from_raw_os_error(EINVAL));
        }
        let fd = host().bind(socket_kind(self.kind)?, addr)?;
        self.adopt(fd)
    }

    pub fn listen(&self, backlog: i32) -> io::Result<()> {
        host().listen(self.raw_fd()?, backlog)
    }

    /// Takes the next connection, as a socket of the same family, kind and timeout.
    pub fn accept(&self) -> io::Result<(Self, SockAddr)> {
        let (fd, addr) = self.when_ready(POLLIN, |fd| host().accept(fd))?;
        let mut socket = Self::new(self.family, self.kind, self.protocol)?;
        socket.timeout = self.timeout;
        socket.adopt(fd)?;
        Ok((socket, addr))
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        self.when_ready(POLLOUT, |fd| host().send(fd, buf, flags))
    }

    /// Sends every byte of `buf`.
    pub fn send_all(&self, mut buf: &[u8], flags: i32) -> io::Result<()> {
        while !buf.is_empty() {
            let sent = self.send(buf, flags)?;
            buf = &buf[sent..];
        }
        Ok(())
    }

    pub fn recv(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        self.when_ready(POLLIN, |fd| host().recv(fd, buf, flags))
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        host().shutdown(self.raw_fd()?, how)
    }

    pub fn local_addr(&self) -> io::Result<SockAddr> {
        host().local_addr(self.raw_fd()?)
    }

    pub fn peer_addr(&self) -> io::Result<SockAddr> {
        host().peer_addr(self.raw_fd()?)
    }

    /// Closes the descriptor through the host, answering its error.
    pub fn close(self) -> io::Result<()> {
        match self.fd {
            Some(fd) => host().close(fd.into_raw_fd()),
            None => Ok(()),
        }
    }

    /// Hands the descriptor over to the caller, or `-1` before `connect` or `bind`.
    #[must_use]
    pub fn detach(self) -> RawFd {
        self.fd.map_or(-1, IntoRawFd::into_raw_fd)
    }
}

/// `SHUT_*` as a [`Shutdown`].
pub fn shutdown_how(how: i32) -> io::Result<Shutdown> {
    match how {
        SHUT_RD => Ok(Shutdown::Read),
        SHUT_WR => Ok(Shutdown::Write),
        SHUT_RDWR => Ok(Shutdown::Both),
        _ => Err(io::Error::from_raw_os_error(EINVAL)),
    }
}

/// Two connected stream sockets.
pub fn socketpair() -> io::Result<(Socket, Socket)> {
    let (a, b) = host().socketpair()?;
    // SAFETY: the host answered two new descriptors this call owns.
    let (a, b) = unsafe { (OwnedFd::from_raw_fd(a), OwnedFd::from_raw_fd(b)) };
    Ok((
        Socket::from_owned_fd(a, AF_UNIX, SOCK_STREAM, 0),
        Socket::from_owned_fd(b, AF_UNIX, SOCK_STREAM, 0),
    ))
}

fn family_admits(family: i32, ip: IpAddr) -> bool {
    match family {
        AF_INET => ip.is_ipv4(),
        AF_INET6 => ip.is_ipv6(),
        _ => true,
    }
}

/// The addresses of `node` and `port` for `family` (`AF_UNSPEC` admits both).
///
/// A numeric address and `localhost` resolve in the guest; any other name asks the host. An
/// `None` for `node` answers the wildcard address with `passive` and the loopback address
/// otherwise.
pub fn getaddrinfo(
    node: Option<&str>,
    port: u16,
    family: i32,
    passive: bool,
) -> io::Result<Vec<SocketAddr>> {
    let ips: Vec<IpAddr> = match node {
        None if passive => vec![
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        ],
        Some(name) if !name.eq_ignore_ascii_case(LOCALHOST) => match name.parse::<IpAddr>() {
            Ok(ip) => vec![ip],
            Err(_) => host().resolve(name.as_bytes())?,
        },
        _ => vec![
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        ],
    };
    let addrs: Vec<SocketAddr> = ips
        .into_iter()
        .filter(|ip| family_admits(family, *ip))
        .map(|ip| SocketAddr::new(ip, port))
        .collect();
    if addrs.is_empty() {
        return Err(io::Error::from_raw_os_error(EAFNOSUPPORT));
    }
    Ok(addrs)
}

/// The host name of the guest.
#[must_use]
pub fn gethostname() -> String {
    HOSTNAME.to_owned()
}
