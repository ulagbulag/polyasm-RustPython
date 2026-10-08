use alloc_crate::{string::String, vec::Vec};
use core::{
    cell::{LazyCell, RefCell, RefMut},
    fmt,
    marker::PhantomData,
};

use super::{BufRead, BufReader, LineWriter, Lines, Read, Result, Write};
use crate::{
    consts::{MAX_RW, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO},
    fs::File,
    host::host,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    sys::GuestCell,
};

/// Unbuffered reads of a host descriptor.
pub(crate) struct FdReader(pub(crate) RawFd);

impl Read for FdReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = buf.len().min(MAX_RW);
        host().read(self.0, &mut buf[..len])
    }
}

/// Unbuffered writes to a host descriptor.
pub(crate) struct FdWriter(pub(crate) RawFd);

impl Write for FdWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        host().write(self.0, &buf[..buf.len().min(MAX_RW)])
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

type StdinBuffer = RefCell<BufReader<FdReader>>;
type StdoutBuffer = RefCell<LineWriter<FdWriter>>;

static STDIN: GuestCell<LazyCell<StdinBuffer>> = GuestCell::new(LazyCell::new(|| {
    RefCell::new(BufReader::new(FdReader(STDIN_FILENO)))
}));
static STDOUT: GuestCell<LazyCell<StdoutBuffer>> = GuestCell::new(LazyCell::new(|| {
    RefCell::new(LineWriter::new(FdWriter(STDOUT_FILENO)))
}));

/// The standard input of the guest, buffered.
#[derive(Clone, Copy)]
pub struct Stdin {
    _private: (),
}

/// A locked [`Stdin`] serving [`BufRead`].
pub struct StdinLock<'a> {
    inner: RefMut<'a, BufReader<FdReader>>,
}

/// The standard input of the guest.
#[must_use]
pub const fn stdin() -> Stdin {
    Stdin { _private: () }
}

impl Stdin {
    #[must_use]
    pub fn lock(&self) -> StdinLock<'static> {
        StdinLock {
            inner: STDIN.borrow_mut(),
        }
    }

    /// Reads one line, newline included, into `buf`.
    pub fn read_line(&self, buf: &mut String) -> Result<usize> {
        self.lock().read_line(buf)
    }

    /// An iterator over the lines of standard input.
    #[must_use]
    pub fn lines(self) -> Lines<StdinLock<'static>> {
        self.lock().lines()
    }
}

impl Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.lock().read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.lock().read_to_end(buf)
    }
}

impl Read for &Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.lock().read(buf)
    }
}

impl Read for StdinLock<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.inner.read_to_end(buf)
    }
}

impl BufRead for StdinLock<'_> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.inner.consume(amount);
    }
}

impl fmt::Debug for Stdin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stdin").finish_non_exhaustive()
    }
}

impl fmt::Debug for StdinLock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StdinLock").finish_non_exhaustive()
    }
}

/// The standard output of the guest, line-buffered.
#[derive(Clone, Copy)]
pub struct Stdout {
    _private: (),
}

/// A locked [`Stdout`].
pub struct StdoutLock<'a> {
    _marker: PhantomData<&'a Stdout>,
}

/// The standard output of the guest.
#[must_use]
pub const fn stdout() -> Stdout {
    Stdout { _private: () }
}

impl Stdout {
    #[must_use]
    pub const fn lock(&self) -> StdoutLock<'static> {
        StdoutLock {
            _marker: PhantomData,
        }
    }
}

impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        STDOUT.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        STDOUT.borrow_mut().flush()
    }
}

impl Write for &Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        STDOUT.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        STDOUT.borrow_mut().flush()
    }
}

impl Write for StdoutLock<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        STDOUT.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        STDOUT.borrow_mut().flush()
    }
}

impl fmt::Debug for Stdout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stdout").finish_non_exhaustive()
    }
}

impl fmt::Debug for StdoutLock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StdoutLock").finish_non_exhaustive()
    }
}

/// The standard error of the guest, unbuffered.
#[derive(Clone, Copy)]
pub struct Stderr {
    _private: (),
}

/// A locked [`Stderr`].
pub struct StderrLock<'a> {
    _marker: PhantomData<&'a Stderr>,
}

/// The standard error of the guest.
#[must_use]
pub const fn stderr() -> Stderr {
    Stderr { _private: () }
}

impl Stderr {
    #[must_use]
    pub const fn lock(&self) -> StderrLock<'static> {
        StderrLock {
            _marker: PhantomData,
        }
    }
}

impl Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        FdWriter(STDERR_FILENO).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for &Stderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        FdWriter(STDERR_FILENO).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for StderrLock<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        FdWriter(STDERR_FILENO).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl fmt::Debug for Stderr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stderr").finish_non_exhaustive()
    }
}

impl fmt::Debug for StderrLock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StderrLock").finish_non_exhaustive()
    }
}

macro_rules! std_stream_fd {
    ($fd:expr => $($stream:ty),+ $(,)?) => {
        $(
            impl AsRawFd for $stream {
                fn as_raw_fd(&self) -> RawFd {
                    $fd
                }
            }

            impl AsFd for $stream {
                fn as_fd(&self) -> BorrowedFd<'_> {
                    // SAFETY: the standard descriptors stay open for the whole guest run.
                    unsafe { BorrowedFd::borrow_raw($fd) }
                }
            }
        )+
    };
}

std_stream_fd!(STDIN_FILENO => Stdin, StdinLock<'_>);
std_stream_fd!(STDOUT_FILENO => Stdout, StdoutLock<'_>);
std_stream_fd!(STDERR_FILENO => Stderr, StderrLock<'_>);

mod sealed {
    pub trait Sealed {}
}

/// Whether a descriptor refers to a terminal.
pub trait IsTerminal: sealed::Sealed {
    fn is_terminal(&self) -> bool;
}

macro_rules! is_terminal {
    ($($t:ty),+ $(,)?) => {
        $(
            impl sealed::Sealed for $t {}

            impl IsTerminal for $t {
                fn is_terminal(&self) -> bool {
                    host().isatty(self.as_fd().as_raw_fd())
                }
            }
        )+
    };
}

is_terminal!(
    BorrowedFd<'_>,
    File,
    OwnedFd,
    Stderr,
    StderrLock<'_>,
    Stdin,
    StdinLock<'_>,
    Stdout,
    StdoutLock<'_>,
);

#[doc(hidden)]
pub fn _print(args: fmt::Arguments<'_>) {
    if let Err(e) = stdout().write_fmt(args) {
        panic!("failed printing to stdout: {e}");
    }
}

#[doc(hidden)]
pub fn _print_line(args: fmt::Arguments<'_>) {
    _print(format_args!("{args}\n"));
}

#[doc(hidden)]
pub fn _eprint(args: fmt::Arguments<'_>) {
    if let Err(e) = stderr().write_fmt(args) {
        panic!("failed printing to stderr: {e}");
    }
}

#[doc(hidden)]
pub fn _eprint_line(args: fmt::Arguments<'_>) {
    _eprint(format_args!("{args}\n"));
}
