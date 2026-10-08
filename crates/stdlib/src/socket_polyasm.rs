//! `_socket` on PolyASM, over the sockets of `rustpython_host_env::socket`.
//!
//! The embedder's host opens each socket already connected or bound, so a socket gets its
//! descriptor at `connect` or `bind`; `fileno` answers `-1` before that. Name resolution answers
//! numeric addresses and `localhost` in the guest and asks the host for other names.

pub(crate) use _socket::module_def;

#[pymodule]
mod _socket {
    use core::{
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
        time::Duration,
    };
    use rustpython_host_env::socket::{self as host_socket, Socket};
    use rustpython_vm::{
        Py, PyObjectRef, PyPayload, PyResult, VirtualMachine,
        builtins::{PyBaseExceptionRef, PyTuple, PyTypeRef, PyUtf8StrRef},
        common::lock::PyMutex,
        convert::{IntoPyException, ToPyObject},
        function::{ArgBytesLike, ArgIntoFloat, ArgMemoryBuffer, Either, OptionalArg},
        types::{Constructor, DefaultConstructor, Initializer},
    };
    use std::{
        host::SockAddr,
        io,
        os::fd::{FromRawFd, OwnedFd},
        sync::atomic::{AtomicI32, AtomicU64, Ordering},
    };

    #[pyattr]
    use rustpython_host_env::socket::{
        AF_INET, AF_INET6, AF_UNIX, AF_UNSPEC, AI_ADDRCONFIG, AI_CANONNAME, AI_NUMERICHOST,
        AI_NUMERICSERV, AI_PASSIVE, INADDR_ANY, INADDR_BROADCAST, INADDR_LOOPBACK, INADDR_NONE,
        IPPORT_RESERVED, IPPORT_USERRESERVED, IPPROTO_IP, IPPROTO_IPV6, IPPROTO_TCP, IPPROTO_UDP,
        MSG_DONTROUTE, MSG_DONTWAIT, MSG_OOB, MSG_PEEK, NI_DGRAM, NI_NAMEREQD, NI_NOFQDN,
        NI_NUMERICHOST, NI_NUMERICSERV, SHUT_RD, SHUT_RDWR, SHUT_WR, SO_BROADCAST, SO_ERROR,
        SO_KEEPALIVE, SO_RCVBUF, SO_REUSEADDR, SO_SNDBUF, SO_TYPE, SOCK_DGRAM, SOCK_RAW,
        SOCK_STREAM, SOL_SOCKET, SOL_TCP, TCP_NODELAY,
    };
    #[pyattr(name = "has_ipv6")]
    const HAS_IPV6: bool = true;

    static DEFAULT_TIMEOUT: AtomicU64 = AtomicU64::new(f64::to_bits(-1.0));

    /// The Python exception of a socket failure: `TimeoutError` for a passed timeout, the
    /// `OSError` subclass of its errno otherwise.
    fn os_error(err: io::Error, vm: &VirtualMachine) -> PyBaseExceptionRef {
        if err.kind() == io::ErrorKind::TimedOut && err.raw_os_error().is_none() {
            return vm.new_exception_msg(
                vm.ctx.exceptions.timeout_error.to_owned(),
                err.to_string().into(),
            );
        }
        err.into_pyexception(vm)
    }

    /// The `gaierror` of a name the resolver leaves unanswered.
    fn unknown_name(vm: &VirtualMachine) -> PyBaseExceptionRef {
        vm.new_exception_msg(gaierror(vm), "Name or service not known".into())
    }

    #[pyattr]
    fn error(vm: &VirtualMachine) -> PyTypeRef {
        vm.ctx.exceptions.os_error.to_owned()
    }

    #[pyattr]
    fn timeout(vm: &VirtualMachine) -> PyTypeRef {
        vm.ctx.exceptions.timeout_error.to_owned()
    }

    #[pyattr(once)]
    fn herror(vm: &VirtualMachine) -> PyTypeRef {
        vm.ctx.new_exception_type(
            "socket",
            "herror",
            Some(vec![vm.ctx.exceptions.os_error.to_owned()]),
        )
    }

    #[pyattr(once)]
    fn gaierror(vm: &VirtualMachine) -> PyTypeRef {
        vm.ctx.new_exception_type(
            "socket",
            "gaierror",
            Some(vec![vm.ctx.exceptions.os_error.to_owned()]),
        )
    }

    #[pyfunction]
    const fn htonl(x: u32) -> u32 {
        u32::to_be(x)
    }

    #[pyfunction]
    const fn htons(x: u16) -> u16 {
        u16::to_be(x)
    }

    #[pyfunction]
    const fn ntohl(x: u32) -> u32 {
        u32::from_be(x)
    }

    #[pyfunction]
    const fn ntohs(x: u16) -> u16 {
        u16::from_be(x)
    }

    #[pyfunction]
    fn inet_aton(ip: PyUtf8StrRef, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        ip.as_str()
            .parse::<Ipv4Addr>()
            .map(|addr| addr.octets().to_vec())
            .map_err(|_| vm.new_os_error("illegal IP address string passed to inet_aton"))
    }

    #[pyfunction]
    fn inet_ntoa(packed: ArgBytesLike, vm: &VirtualMachine) -> PyResult<String> {
        let buf = packed.borrow_buf();
        let octets: [u8; 4] = (&*buf)
            .try_into()
            .map_err(|_| vm.new_os_error("packed IP wrong length for inet_ntoa"))?;
        Ok(Ipv4Addr::from(octets).to_string())
    }

    #[pyfunction]
    fn inet_pton(af: i32, ip: PyUtf8StrRef, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        let text = ip.as_str();
        let packed = match af {
            AF_INET => text.parse::<Ipv4Addr>().map(|addr| addr.octets().to_vec()),
            AF_INET6 => text.parse::<Ipv6Addr>().map(|addr| addr.octets().to_vec()),
            _ => return Err(vm.new_os_error("Address family not supported")),
        };
        packed.map_err(|_| vm.new_os_error("illegal IP address string passed to inet_pton"))
    }

    #[pyfunction]
    fn inet_ntop(af: i32, packed: ArgBytesLike, vm: &VirtualMachine) -> PyResult<String> {
        let buf = packed.borrow_buf();
        let ip = match af {
            AF_INET => <[u8; 4]>::try_from(&*buf).map(IpAddr::from),
            AF_INET6 => <[u8; 16]>::try_from(&*buf).map(IpAddr::from),
            _ => return Err(vm.new_value_error("unknown address family")),
        };
        ip.map(|ip| ip.to_string())
            .map_err(|_| vm.new_value_error("invalid length of packed IP address string"))
    }

    /// A timeout in seconds, checked the way `settimeout` checks it.
    fn timeout_seconds(
        timeout: Option<ArgIntoFloat>,
        vm: &VirtualMachine,
    ) -> PyResult<Option<f64>> {
        timeout
            .map(|value| {
                let value = value.into_float();
                if value.is_nan() {
                    return Err(vm.new_value_error("Invalid value NaN (not a number)"));
                }
                if value < 0.0 || !value.is_finite() {
                    return Err(vm.new_value_error("Timeout value out of range"));
                }
                Ok(value)
            })
            .transpose()
    }

    #[pyfunction]
    fn getdefaulttimeout() -> Option<f64> {
        let timeout = f64::from_bits(DEFAULT_TIMEOUT.load(Ordering::Relaxed));
        (timeout >= 0.0).then_some(timeout)
    }

    #[pyfunction]
    fn setdefaulttimeout(timeout: Option<ArgIntoFloat>, vm: &VirtualMachine) -> PyResult<()> {
        let timeout = timeout_seconds(timeout, vm)?.unwrap_or(-1.0);
        DEFAULT_TIMEOUT.store(timeout.to_bits(), Ordering::Relaxed);
        Ok(())
    }

    #[pyfunction]
    fn gethostname() -> String {
        host_socket::gethostname()
    }

    /// The text of a host argument: `str`, ASCII `bytes`, or `None`.
    fn host_text(host: Option<PyObjectRef>, vm: &VirtualMachine) -> PyResult<Option<String>> {
        let Some(host) = host.filter(|host| !vm.is_none(host)) else {
            return Ok(None);
        };
        match host.try_into_value::<Either<PyUtf8StrRef, ArgBytesLike>>(vm)? {
            Either::A(text) => Ok(Some(text.as_str().to_owned())),
            Either::B(bytes) => String::from_utf8(bytes.borrow_buf().to_vec())
                .map(Some)
                .map_err(|_| vm.new_value_error("a host name takes UTF-8 text")),
        }
    }

    /// The port of a port argument: an integer, a numeric string, or `None`.
    fn port_number(port: Option<PyObjectRef>, vm: &VirtualMachine) -> PyResult<u16> {
        let Some(port) = port.filter(|port| !vm.is_none(port)) else {
            return Ok(0);
        };
        match port.try_into_value::<Either<i32, PyUtf8StrRef>>(vm)? {
            Either::A(number) => {
                u16::try_from(number).map_err(|_| vm.new_overflow_error("port must be 0-65535."))
            }
            Either::B(text) => text.as_str().parse().map_err(|_| {
                vm.new_exception_msg(
                    gaierror(vm),
                    "Servname not supported for ai_socktype".into(),
                )
            }),
        }
    }

    /// The Python form of a socket address.
    fn address(addr: &SockAddr, vm: &VirtualMachine) -> PyObjectRef {
        match addr {
            SockAddr::Inet(SocketAddr::V4(v4)) => (v4.ip().to_string(), v4.port()).to_pyobject(vm),
            SockAddr::Inet(SocketAddr::V6(v6)) => {
                (v6.ip().to_string(), v6.port(), v6.flowinfo(), v6.scope_id()).to_pyobject(vm)
            }
            SockAddr::Unix(path) => String::from_utf8_lossy(path).into_owned().to_pyobject(vm),
        }
    }

    /// The socket address of a Python address for `family`; an empty host of a `bind` names
    /// the wildcard address.
    fn sock_addr(
        family: i32,
        object: PyObjectRef,
        passive: bool,
        vm: &VirtualMachine,
    ) -> PyResult<SockAddr> {
        if family == AF_UNIX {
            let path = host_text(Some(object), vm)?.unwrap_or_default();
            return Ok(SockAddr::Unix(path.into_bytes()));
        }
        let Some([host, port, ..]) = object
            .downcast_ref::<PyTuple>()
            .map(|tuple| tuple.as_slice())
        else {
            return Err(vm.new_type_error("a socket address is a (host, port) tuple"));
        };
        let host = host_text(Some(host.clone()), vm)?.filter(|host| !host.is_empty());
        let port = port_number(Some(port.clone()), vm)?;
        let addrs = host_socket::getaddrinfo(host.as_deref(), port, family, passive)
            .map_err(|_| unknown_name(vm))?;
        Ok(SockAddr::Inet(addrs[0]))
    }

    #[derive(FromArgs)]
    struct GaiArgs {
        #[pyarg(any)]
        host: Option<PyObjectRef>,
        #[pyarg(any)]
        port: Option<PyObjectRef>,
        #[pyarg(any, default = AF_UNSPEC)]
        family: i32,
        #[pyarg(any, name = "type", default)]
        kind: i32,
        #[pyarg(any, default)]
        proto: i32,
        #[pyarg(any, default)]
        flags: i32,
    }

    #[pyfunction]
    fn getaddrinfo(args: GaiArgs, vm: &VirtualMachine) -> PyResult<Vec<PyObjectRef>> {
        let host = host_text(args.host, vm)?;
        let port = port_number(args.port, vm)?;
        let passive = args.flags & AI_PASSIVE != 0;
        let addrs = host_socket::getaddrinfo(host.as_deref(), port, args.family, passive)
            .map_err(|_| unknown_name(vm))?;
        let kinds: &[(i32, i32)] = match args.kind {
            SOCK_STREAM => &[(SOCK_STREAM, IPPROTO_TCP)],
            SOCK_DGRAM => &[(SOCK_DGRAM, IPPROTO_UDP)],
            _ => &[(SOCK_STREAM, IPPROTO_TCP), (SOCK_DGRAM, IPPROTO_UDP)],
        };
        Ok(addrs
            .iter()
            .flat_map(|addr| {
                let addr = SockAddr::Inet(*addr);
                let family = host_socket::address_family(&addr);
                kinds.iter().map(move |(kind, proto)| {
                    let proto = if args.proto == 0 { *proto } else { args.proto };
                    (family, *kind, proto, "", address(&addr, vm)).to_pyobject(vm)
                })
            })
            .collect())
    }

    #[pyfunction]
    fn gethostbyname(name: PyUtf8StrRef, vm: &VirtualMachine) -> PyResult<String> {
        host_socket::getaddrinfo(Some(name.as_str()), 0, AF_INET, false)
            .map(|addrs| addrs[0].ip().to_string())
            .map_err(|_| unknown_name(vm))
    }

    #[pyfunction]
    fn socketpair(
        family: OptionalArg<i32>,
        kind: OptionalArg<i32>,
        proto: OptionalArg<i32>,
        vm: &VirtualMachine,
    ) -> PyResult<(PySocket, PySocket)> {
        let _ = (family, kind, proto);
        let (a, b) = host_socket::socketpair().map_err(|err| os_error(err, vm))?;
        Ok((PySocket::with_socket(a), PySocket::with_socket(b)))
    }

    #[derive(FromArgs)]
    struct SocketInitArgs {
        #[pyarg(any, optional)]
        family: OptionalArg<i32>,
        #[pyarg(any, optional)]
        r#type: OptionalArg<i32>,
        #[pyarg(any, optional)]
        proto: OptionalArg<i32>,
        #[pyarg(any, optional)]
        fileno: Option<PyObjectRef>,
    }

    #[pyattr(name = "socket")]
    #[pyattr(name = "SocketType")]
    #[pyclass(name = "socket")]
    #[derive(Debug, PyPayload)]
    struct PySocket {
        #[pymember]
        family: AtomicI32,
        #[pymember(name = "type")]
        kind: AtomicI32,
        #[pymember]
        proto: AtomicI32,
        timeout: PyMutex<Option<f64>>,
        socket: PyMutex<Option<Socket>>,
    }

    impl Default for PySocket {
        fn default() -> Self {
            Self {
                family: AtomicI32::new(AF_INET),
                kind: AtomicI32::new(SOCK_STREAM),
                proto: AtomicI32::new(0),
                timeout: PyMutex::new(None),
                socket: PyMutex::new(None),
            }
        }
    }

    impl PySocket {
        fn with_socket(socket: Socket) -> Self {
            Self {
                family: AtomicI32::new(socket.family()),
                kind: AtomicI32::new(socket.kind()),
                proto: AtomicI32::new(socket.protocol()),
                timeout: PyMutex::new(None),
                socket: PyMutex::new(Some(socket)),
            }
        }

        /// Runs `op` on the open socket.
        fn with<T>(
            &self,
            vm: &VirtualMachine,
            op: impl FnOnce(&mut Socket) -> io::Result<T>,
        ) -> PyResult<T> {
            let mut socket = self.socket.lock();
            let socket = socket
                .as_mut()
                .ok_or_else(|| vm.new_os_error("Bad file descriptor"))?;
            op(socket).map_err(|err| os_error(err, vm))
        }

        fn family(&self) -> i32 {
            self.family.load(Ordering::Relaxed)
        }

        fn set_timeout(&self, timeout: Option<f64>, vm: &VirtualMachine) -> PyResult<()> {
            *self.timeout.lock() = timeout;
            self.with(vm, |socket| {
                socket.set_timeout(timeout.map(Duration::from_secs_f64))
            })
        }
    }

    #[pyclass(with(Constructor, Initializer), flags(BASETYPE))]
    impl PySocket {
        #[pymethod]
        fn fileno(zelf: &Py<Self>) -> i32 {
            zelf.socket.lock().as_ref().map_or(-1, Socket::fileno)
        }

        #[pymethod]
        fn close(zelf: &Py<Self>, vm: &VirtualMachine) -> PyResult<()> {
            match zelf.socket.lock().take() {
                Some(socket) => socket.close().map_err(|err| os_error(err, vm)),
                None => Ok(()),
            }
        }

        #[pymethod]
        fn detach(zelf: &Py<Self>) -> i32 {
            zelf.socket.lock().take().map_or(-1, Socket::detach)
        }

        #[pymethod]
        fn gettimeout(zelf: &Py<Self>) -> Option<f64> {
            *zelf.timeout.lock()
        }

        #[pymethod]
        fn settimeout(
            zelf: &Py<Self>,
            timeout: Option<ArgIntoFloat>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            let timeout = timeout_seconds(timeout, vm)?;
            zelf.set_timeout(timeout, vm)
        }

        #[pymethod]
        fn setblocking(zelf: &Py<Self>, blocking: bool, vm: &VirtualMachine) -> PyResult<()> {
            zelf.set_timeout(if blocking { None } else { Some(0.0) }, vm)
        }

        #[pymethod]
        fn getblocking(zelf: &Py<Self>) -> bool {
            !matches!(*zelf.timeout.lock(), Some(t) if t == 0.0)
        }

        #[pymethod]
        fn getsockopt(
            zelf: &Py<Self>,
            level: i32,
            optname: i32,
            _buflen: OptionalArg<i32>,
            vm: &VirtualMachine,
        ) -> PyResult<i32> {
            zelf.with(vm, |_| Ok(()))?;
            Ok(match (level, optname) {
                (SOL_SOCKET, SO_TYPE) => zelf.kind.load(Ordering::Relaxed),
                _ => 0,
            })
        }

        #[pymethod]
        fn setsockopt(
            _zelf: &Py<Self>,
            _level: i32,
            _optname: i32,
            _value: OptionalArg<PyObjectRef>,
            _optlen: OptionalArg<i32>,
        ) {
        }

        #[pymethod]
        fn bind(zelf: &Py<Self>, address: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
            let addr = sock_addr(zelf.family(), address, true, vm)?;
            zelf.with(vm, |socket| socket.bind(&addr))
        }

        #[pymethod]
        fn connect(zelf: &Py<Self>, address: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
            let addr = sock_addr(zelf.family(), address, false, vm)?;
            zelf.with(vm, |socket| socket.connect(&addr))
        }

        #[pymethod]
        fn connect_ex(zelf: &Py<Self>, address: PyObjectRef, vm: &VirtualMachine) -> PyResult<i32> {
            let addr = sock_addr(zelf.family(), address, false, vm)?;
            zelf.with(vm, |socket| {
                Ok(socket
                    .connect(&addr)
                    .map_or_else(|err| err.raw_os_error().unwrap_or(-1), |()| 0))
            })
        }

        #[pymethod]
        fn listen(zelf: &Py<Self>, backlog: OptionalArg<i32>, vm: &VirtualMachine) -> PyResult<()> {
            zelf.with(vm, |socket| socket.listen(backlog.unwrap_or(128)))
        }

        #[pymethod]
        fn _accept(zelf: &Py<Self>, vm: &VirtualMachine) -> PyResult<(i32, PyObjectRef)> {
            let (accepted, addr) = zelf.with(vm, |socket| socket.accept())?;
            Ok((accepted.detach(), address(&addr, vm)))
        }

        #[pymethod]
        fn send(
            zelf: &Py<Self>,
            data: ArgBytesLike,
            flags: OptionalArg<i32>,
            vm: &VirtualMachine,
        ) -> PyResult<usize> {
            let buf = data.borrow_buf();
            zelf.with(vm, |socket| socket.send(&buf, flags.unwrap_or(0)))
        }

        #[pymethod]
        fn sendall(
            zelf: &Py<Self>,
            data: ArgBytesLike,
            flags: OptionalArg<i32>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            let buf = data.borrow_buf();
            zelf.with(vm, |socket| socket.send_all(&buf, flags.unwrap_or(0)))
        }

        #[pymethod]
        fn recv(
            zelf: &Py<Self>,
            bufsize: usize,
            flags: OptionalArg<i32>,
            vm: &VirtualMachine,
        ) -> PyResult<Vec<u8>> {
            let mut buf = vec![0; bufsize];
            let count = zelf.with(vm, |socket| socket.recv(&mut buf, flags.unwrap_or(0)))?;
            buf.truncate(count);
            Ok(buf)
        }

        #[pymethod]
        fn recv_into(
            zelf: &Py<Self>,
            buffer: ArgMemoryBuffer,
            nbytes: OptionalArg<usize>,
            flags: OptionalArg<i32>,
            vm: &VirtualMachine,
        ) -> PyResult<usize> {
            let mut buf = buffer.borrow_buf_mut();
            let len = match nbytes {
                OptionalArg::Present(nbytes) if nbytes > 0 => nbytes.min(buf.len()),
                _ => buf.len(),
            };
            zelf.with(vm, |socket| {
                socket.recv(&mut buf[..len], flags.unwrap_or(0))
            })
        }

        #[pymethod]
        fn shutdown(zelf: &Py<Self>, how: i32, vm: &VirtualMachine) -> PyResult<()> {
            zelf.with(vm, |socket| {
                socket.shutdown(host_socket::shutdown_how(how)?)
            })
        }

        #[pymethod]
        fn getsockname(zelf: &Py<Self>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let addr = zelf.with(vm, |socket| socket.local_addr())?;
            Ok(address(&addr, vm))
        }

        #[pymethod]
        fn getpeername(zelf: &Py<Self>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let addr = zelf.with(vm, |socket| socket.peer_addr())?;
            Ok(address(&addr, vm))
        }
    }

    impl DefaultConstructor for PySocket {}

    impl Initializer for PySocket {
        type Args = SocketInitArgs;

        fn init(zelf: &Py<Self>, args: Self::Args, vm: &VirtualMachine) -> PyResult<()> {
            let family = args
                .family
                .into_option()
                .filter(|f| *f != -1)
                .unwrap_or(AF_INET);
            let kind = args
                .r#type
                .into_option()
                .filter(|k| *k != -1)
                .unwrap_or(SOCK_STREAM);
            let proto = args.proto.into_option().filter(|p| *p != -1).unwrap_or(0);
            let fileno = args
                .fileno
                .filter(|fileno| !vm.is_none(fileno))
                .map(|fileno| fileno.try_into_value::<i32>(vm))
                .transpose()?;
            let socket = match fileno {
                // SAFETY: the caller hands over a descriptor it owns, as `socket(fileno=…)`
                // promises.
                Some(fd) => {
                    Socket::from_owned_fd(unsafe { OwnedFd::from_raw_fd(fd) }, family, kind, proto)
                }
                None => Socket::new(family, kind, proto).map_err(|err| os_error(err, vm))?,
            };
            zelf.family.store(family, Ordering::Relaxed);
            zelf.kind.store(kind, Ordering::Relaxed);
            zelf.proto.store(proto, Ordering::Relaxed);
            *zelf.socket.lock() = Some(socket);
            zelf.set_timeout(getdefaulttimeout(), vm)
        }
    }
}
