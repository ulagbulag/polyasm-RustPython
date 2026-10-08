//! Constants of the PolyASM backend: clock ids, descriptor limits and the guest identity.

pub const CLOCK_REALTIME: i32 = 0;
pub const CLOCK_MONOTONIC: i32 = 1;
pub const CLOCK_PROCESS_CPUTIME_ID: i32 = 2;
pub const CLOCK_THREAD_CPUTIME_ID: i32 = 3;

/// Largest descriptor count a `select` set holds.
pub const FD_SETSIZE: usize = 1024;
/// Largest write a pipe takes in one piece.
pub const PIPE_BUF: usize = 4096;
pub(crate) const STDERR_FILENO: i32 = 2;

/// Page size of PolyASM linear memory.
pub(crate) const PAGE_SIZE: usize = 64 * 1024;

/// Parent process id the guest reports for itself.
pub(crate) const PARENT_PROCESS_ID: i32 = 0;
/// User id and group id the guest reports for itself.
pub(crate) const GUEST_ID: u32 = 0;
/// `umask(2)` value of a fresh guest.
pub(crate) const INITIAL_UMASK: u32 = 0o022;
/// `uname(2)` fields of the guest.
pub(crate) const UNAME_SYSNAME: &str = "PolyASM";
pub(crate) const UNAME_RELEASE: &str = "1.0";
pub(crate) const UNAME_VERSION: &str = "wasm-direct";
pub(crate) const UNAME_MACHINE: &str = "polyasm";
/// Host name of the guest: [`crate::socket::gethostname`] answers it, and `uname(2)` reports
/// it as the node name.
pub(crate) const HOSTNAME: &str = "localhost";
/// Time zone name the guest reports: the host clock is UTC.
pub(crate) const TIME_ZONE_NAME: &str = "UTC";
/// Name that resolves to the loopback addresses inside the guest.
pub(crate) const LOCALHOST: &str = "localhost";
/// Linux signal numbers: the `signal` module names them, and the guest raises none of them.
pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGTRAP: i32 = 5;
pub const SIGABRT: i32 = 6;
pub const SIGBUS: i32 = 7;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGSEGV: i32 = 11;
pub const SIGUSR2: i32 = 12;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;
pub const SIGCHLD: i32 = 17;
/// `recv(2)`/`send(2)` flag: answer `EAGAIN` at once in place of a wait.
pub const MSG_DONTWAIT: i32 = 0x40;
/// Message of the error a socket operation answers when its timeout passes.
pub(crate) const SOCKET_TIMEOUT_MESSAGE: &str = "timed out";
