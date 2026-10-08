//! Traits, helpers and types for I/O, with the standard streams on the embedder host.

mod buffered;
mod cursor;
mod error;
mod impls;
mod stdio;
mod util;

use alloc_crate::{string::String, vec::Vec};
use core::{fmt, ops, slice};

use crate::consts::DEFAULT_BUF_SIZE;

pub use buffered::{BufReader, BufWriter, IntoInnerError, LineWriter};
pub use cursor::Cursor;
pub use error::{Error, ErrorKind, Result};
pub(crate) use error::{decode_error_kind, kind_errno, strerror};
#[doc(hidden)]
pub use stdio::{_eprint, _eprint_line, _print, _print_line};
pub use stdio::{
    IsTerminal, Stderr, StderrLock, Stdin, StdinLock, Stdout, StdoutLock, stderr, stdin, stdout,
};
pub use util::{Empty, Repeat, Sink, copy, empty, repeat, sink};

/// The I/O prelude: the traits most I/O code imports.
pub mod prelude {
    pub use super::{BufRead, Read, Seek, Write};
}

const EOF_BEFORE_FILL: Error =
    Error::const_message(ErrorKind::UnexpectedEof, "failed to fill whole buffer");
const WRITE_ZERO: Error =
    Error::const_message(ErrorKind::WriteZero, "failed to write whole buffer");
const INVALID_UTF8: Error =
    Error::const_message(ErrorKind::InvalidData, "stream did not contain valid UTF-8");

/// Reads every byte of `reader` into `buf`, growing it in chunks.
pub(crate) fn default_read_to_end<R: Read + ?Sized>(
    reader: &mut R,
    buf: &mut Vec<u8>,
) -> Result<usize> {
    let start = buf.len();
    let mut chunk = DEFAULT_BUF_SIZE;
    loop {
        let filled = buf.len();
        buf.resize(filled + chunk, 0);
        match reader.read(&mut buf[filled..]) {
            Ok(0) => {
                buf.truncate(filled);
                return Ok(filled - start);
            }
            Ok(n) => {
                buf.truncate(filled + n);
                if n == chunk {
                    chunk = chunk.saturating_mul(2);
                }
            }
            Err(e) if e.is_interrupted() => buf.truncate(filled),
            Err(e) => {
                buf.truncate(filled);
                return Err(e);
            }
        }
    }
}

/// Appends the UTF-8 text `read` produces to `buf`, leaving `buf` intact on invalid UTF-8.
fn append_to_string<F>(buf: &mut String, read: F) -> Result<usize>
where
    F: FnOnce(&mut Vec<u8>) -> Result<usize>,
{
    let mut bytes = Vec::new();
    let result = read(&mut bytes);
    match String::from_utf8(bytes) {
        Ok(text) => {
            buf.push_str(&text);
            result
        }
        Err(_) => result.and(Err(INVALID_UTF8)),
    }
}

/// Reads every byte of `reader` into a new [`String`].
pub fn read_to_string<R: Read>(mut reader: R) -> Result<String> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    Ok(buf)
}

/// A source of bytes.
pub trait Read {
    /// Pulls some bytes into `buf`, answering how many arrived; 0 marks the end of the stream.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Reads into the first buffer of `bufs` that holds room.
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        let buf = bufs
            .iter_mut()
            .find(|b| !b.is_empty())
            .map_or(&mut [][..], |b| &mut **b);
        self.read(buf)
    }

    /// Reads every remaining byte into `buf`.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        default_read_to_end(self, buf)
    }

    /// Reads every remaining byte into `buf` as UTF-8 text.
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        append_to_string(buf, |bytes| self.read_to_end(bytes))
    }

    /// Fills `buf` completely.
    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => return Err(EOF_BEFORE_FILL),
                Ok(n) => buf = &mut buf[n..],
                Err(e) if e.is_interrupted() => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Borrows the reader as a [`Read`] of its own.
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    /// An iterator over the bytes of the reader.
    fn bytes(self) -> Bytes<Self>
    where
        Self: Sized,
    {
        Bytes { inner: self }
    }

    /// A reader of this reader followed by `next`.
    fn chain<R: Read>(self, next: R) -> Chain<Self, R>
    where
        Self: Sized,
    {
        Chain {
            first: self,
            second: next,
            done_first: false,
        }
    }

    /// A reader of at most `limit` bytes of this reader.
    fn take(self, limit: u64) -> Take<Self>
    where
        Self: Sized,
    {
        Take { inner: self, limit }
    }
}

/// A sink of bytes.
pub trait Write {
    /// Writes some bytes of `buf`, answering how many were taken.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Writes the first buffer of `bufs` that holds bytes.
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        let buf = bufs
            .iter()
            .find(|b| !b.is_empty())
            .map_or(&[][..], |b| &**b);
        self.write(buf)
    }

    /// Delivers every buffered byte to its destination.
    fn flush(&mut self) -> Result<()>;

    /// Writes every byte of `buf`.
    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => return Err(WRITE_ZERO),
                Ok(n) => buf = &buf[n..],
                Err(e) if e.is_interrupted() => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Writes formatted text.
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        struct Adapter<'a, T: ?Sized> {
            inner: &'a mut T,
            error: Result<()>,
        }

        impl<T: Write + ?Sized> fmt::Write for Adapter<'_, T> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.inner.write_all(s.as_bytes()).map_err(|e| {
                    self.error = Err(e);
                    fmt::Error
                })
            }
        }

        let mut output = Adapter {
            inner: self,
            error: Ok(()),
        };
        match fmt::write(&mut output, args) {
            Ok(()) => Ok(()),
            Err(_) => output.error.and(Err(Error::const_message(
                ErrorKind::Uncategorized,
                "formatter error",
            ))),
        }
    }

    /// Borrows the writer as a [`Write`] of its own.
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

/// A position to seek to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

/// A cursor that moves within a stream of bytes.
pub trait Seek {
    /// Moves the cursor, answering the new position from the start.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;

    /// Moves the cursor to the start.
    fn rewind(&mut self) -> Result<()> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    /// The position of the cursor from the start.
    fn stream_position(&mut self) -> Result<u64> {
        self.seek(SeekFrom::Current(0))
    }
}

/// A reader with an internal buffer.
pub trait BufRead: Read {
    /// The buffered bytes, refilled from the source when empty.
    fn fill_buf(&mut self) -> Result<&[u8]>;

    /// Marks `amount` buffered bytes as read.
    fn consume(&mut self, amount: usize);

    /// Whether more bytes follow.
    fn has_data_left(&mut self) -> Result<bool> {
        self.fill_buf().map(|b| !b.is_empty())
    }

    /// Reads into `buf` through the first `byte`, inclusive.
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        let mut read = 0;
        loop {
            let (done, used) = {
                let available = match self.fill_buf() {
                    Ok(available) => available,
                    Err(e) if e.is_interrupted() => continue,
                    Err(e) => return Err(e),
                };
                match available.iter().position(|b| *b == byte) {
                    Some(i) => {
                        buf.extend_from_slice(&available[..=i]);
                        (true, i + 1)
                    }
                    None => {
                        buf.extend_from_slice(available);
                        (available.is_empty(), available.len())
                    }
                }
            };
            self.consume(used);
            read += used;
            if done {
                return Ok(read);
            }
        }
    }

    /// Skips through the first `byte`, inclusive.
    fn skip_until(&mut self, byte: u8) -> Result<usize> {
        let mut skipped = Vec::new();
        self.read_until(byte, &mut skipped)
    }

    /// Reads one line, newline included, into `buf`.
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        append_to_string(buf, |bytes| self.read_until(b'\n', bytes))
    }

    /// An iterator over the chunks of this reader between `byte` separators.
    fn split(self, byte: u8) -> Split<Self>
    where
        Self: Sized,
    {
        Split {
            buf: self,
            delim: byte,
        }
    }

    /// An iterator over the lines of this reader, newline removed.
    fn lines(self) -> Lines<Self>
    where
        Self: Sized,
    {
        Lines { buf: self }
    }
}

/// Iterator over the bytes of a reader, made by [`Read::bytes`].
#[derive(Debug)]
pub struct Bytes<R> {
    inner: R,
}

impl<R: Read> Iterator for Bytes<R> {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Result<u8>> {
        let mut byte = 0;
        loop {
            return match self.inner.read(slice::from_mut(&mut byte)) {
                Ok(0) => None,
                Ok(_) => Some(Ok(byte)),
                Err(e) if e.is_interrupted() => continue,
                Err(e) => Some(Err(e)),
            };
        }
    }
}

/// Reader of two readers in sequence, made by [`Read::chain`].
#[derive(Debug)]
pub struct Chain<T, U> {
    first: T,
    second: U,
    done_first: bool,
}

impl<T, U> Chain<T, U> {
    pub fn into_inner(self) -> (T, U) {
        (self.first, self.second)
    }

    pub const fn get_ref(&self) -> (&T, &U) {
        (&self.first, &self.second)
    }

    pub const fn get_mut(&mut self) -> (&mut T, &mut U) {
        (&mut self.first, &mut self.second)
    }
}

impl<T: Read, U: Read> Read for Chain<T, U> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.done_first {
            match self.first.read(buf)? {
                0 if !buf.is_empty() => self.done_first = true,
                n => return Ok(n),
            }
        }
        self.second.read(buf)
    }
}

impl<T: BufRead, U: BufRead> BufRead for Chain<T, U> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if !self.done_first {
            match self.first.fill_buf()? {
                [] => self.done_first = true,
                buf => return Ok(buf),
            }
        }
        self.second.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        if self.done_first {
            self.second.consume(amount);
        } else {
            self.first.consume(amount);
        }
    }
}

/// Reader of a bounded prefix of a reader, made by [`Read::take`].
#[derive(Debug)]
pub struct Take<T> {
    inner: T,
    limit: u64,
}

impl<T> Take<T> {
    pub const fn limit(&self) -> u64 {
        self.limit
    }

    pub const fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub const fn get_ref(&self) -> &T {
        &self.inner
    }

    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Read> Read for Take<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.limit == 0 {
            return Ok(0);
        }
        let max = usize::try_from(self.limit).map_or(buf.len(), |limit| buf.len().min(limit));
        let n = self.inner.read(&mut buf[..max])?;
        self.limit -= n as u64;
        Ok(n)
    }
}

impl<T: BufRead> BufRead for Take<T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.limit == 0 {
            return Ok(&[]);
        }
        let limit = self.limit;
        let buf = self.inner.fill_buf()?;
        let max = usize::try_from(limit).map_or(buf.len(), |limit| buf.len().min(limit));
        Ok(&buf[..max])
    }

    fn consume(&mut self, amount: usize) {
        let amount = usize::try_from(self.limit).map_or(amount, |limit| amount.min(limit));
        self.limit -= amount as u64;
        self.inner.consume(amount);
    }
}

/// Iterator over the chunks of a reader, made by [`BufRead::split`].
#[derive(Debug)]
pub struct Split<B> {
    buf: B,
    delim: u8,
}

impl<B: BufRead> Iterator for Split<B> {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match self.buf.read_until(self.delim, &mut buf) {
            Ok(0) => None,
            Ok(_) => {
                if buf.last() == Some(&self.delim) {
                    buf.pop();
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// Iterator over the lines of a reader, made by [`BufRead::lines`].
#[derive(Debug)]
pub struct Lines<B> {
    buf: B,
}

impl<B: BufRead> Iterator for Lines<B> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Result<String>> {
        let mut line = String::new();
        match self.buf.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => {
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                Some(Ok(line))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// A buffer to write from in [`Write::write_vectored`].
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct IoSlice<'a>(&'a [u8]);

impl<'a> IoSlice<'a> {
    #[must_use]
    pub const fn new(buf: &'a [u8]) -> Self {
        Self(buf)
    }
}

impl ops::Deref for IoSlice<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.0
    }
}

/// A buffer to read into in [`Read::read_vectored`].
#[derive(Debug)]
#[repr(transparent)]
pub struct IoSliceMut<'a>(&'a mut [u8]);

impl<'a> IoSliceMut<'a> {
    pub const fn new(buf: &'a mut [u8]) -> Self {
        Self(buf)
    }
}

impl ops::Deref for IoSliceMut<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.0
    }
}

impl ops::DerefMut for IoSliceMut<'_> {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.0
    }
}
