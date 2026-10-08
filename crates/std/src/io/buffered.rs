use alloc_crate::{boxed::Box, vec, vec::Vec};
use core::{error, fmt, mem::ManuallyDrop, ptr};

use super::{BufRead, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use crate::consts::{DEFAULT_BUF_SIZE, LINE_WRITER_CAPACITY};

/// A reader that buffers the bytes of an inner reader.
pub struct BufReader<R: ?Sized> {
    buf: Box<[u8]>,
    pos: usize,
    filled: usize,
    inner: R,
}

impl<R: Read> BufReader<R> {
    pub fn new(inner: R) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            buf: vec![0; capacity].into_boxed_slice(),
            pos: 0,
            filled: 0,
            inner,
        }
    }
}

impl<R> BufReader<R> {
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: ?Sized> BufReader<R> {
    pub const fn get_ref(&self) -> &R {
        &self.inner
    }

    pub const fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// The bytes buffered and still unread.
    pub fn buffer(&self) -> &[u8] {
        &self.buf[self.pos..self.filled]
    }

    pub const fn capacity(&self) -> usize {
        self.buf.len()
    }

    const fn discard_buffer(&mut self) {
        self.pos = 0;
        self.filled = 0;
    }
}

impl<R: ?Sized + Read> Read for BufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.pos == self.filled && buf.len() >= self.capacity() {
            self.discard_buffer();
            return self.inner.read(buf);
        }
        let n = Read::read(&mut self.fill_buf()?, buf)?;
        self.consume(n);
        Ok(n)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let buffered = self.filled - self.pos;
        buf.extend_from_slice(self.buffer());
        self.discard_buffer();
        Ok(buffered + self.inner.read_to_end(buf)?)
    }
}

impl<R: ?Sized + Read> BufRead for BufReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.pos >= self.filled {
            self.filled = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.filled])
    }

    fn consume(&mut self, amount: usize) {
        self.pos = (self.pos + amount).min(self.filled);
    }
}

impl<R: ?Sized + Seek> Seek for BufReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let result = match pos {
            SeekFrom::Current(offset) => {
                let remainder = (self.filled - self.pos) as i64;
                self.inner.seek(SeekFrom::Current(offset - remainder))?
            }
            other => self.inner.seek(other)?,
        };
        self.discard_buffer();
        Ok(result)
    }
}

impl<R: ?Sized + fmt::Debug> fmt::Debug for BufReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufReader")
            .field("reader", &&self.inner)
            .field(
                "buffer",
                &format_args!("{}/{}", self.filled - self.pos, self.capacity()),
            )
            .finish()
    }
}

/// A writer that buffers bytes before they reach an inner writer.
pub struct BufWriter<W: ?Sized + Write> {
    buf: Vec<u8>,
    panicked: bool,
    inner: W,
}

impl<W: Write> BufWriter<W> {
    pub fn new(inner: W) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacity(capacity: usize, inner: W) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            panicked: false,
            inner,
        }
    }

    /// The inner writer after the buffer reaches it.
    pub fn into_inner(mut self) -> core::result::Result<W, IntoInnerError<Self>> {
        match self.flush_buf() {
            Err(e) => Err(IntoInnerError(self, e)),
            Ok(()) => Ok(self.into_parts().0),
        }
    }

    /// The inner writer and the bytes still buffered.
    pub fn into_parts(self) -> (W, Vec<u8>) {
        let this = ManuallyDrop::new(self);
        // SAFETY: `this` stays inside `ManuallyDrop`, so each field is read exactly once.
        let buf = unsafe { ptr::read(&raw const this.buf) };
        // SAFETY: as above.
        let inner = unsafe { ptr::read(&raw const this.inner) };
        (inner, buf)
    }
}

impl<W: ?Sized + Write> BufWriter<W> {
    /// Delivers the buffered bytes to the inner writer.
    pub(crate) fn flush_buf(&mut self) -> Result<()> {
        let mut written = 0;
        let mut result = Ok(());
        while written < self.buf.len() {
            self.panicked = true;
            let r = self.inner.write(&self.buf[written..]);
            self.panicked = false;
            match r {
                Ok(0) => {
                    result = Err(Error::const_message(
                        ErrorKind::WriteZero,
                        "failed to write the buffered data",
                    ));
                    break;
                }
                Ok(n) => written += n,
                Err(e) if e.is_interrupted() => {}
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        self.buf.drain(..written);
        result
    }

    pub const fn get_ref(&self) -> &W {
        &self.inner
    }

    pub const fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// The bytes buffered and still undelivered.
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }
}

impl<W: ?Sized + Write> Write for BufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            self.flush_buf()?;
        }
        if buf.len() >= self.buf.capacity() {
            self.panicked = true;
            let r = self.inner.write(buf);
            self.panicked = false;
            r
        } else {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> Result<()> {
        self.flush_buf()?;
        self.inner.flush()
    }
}

impl<W: ?Sized + Write + Seek> Seek for BufWriter<W> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.flush_buf()?;
        self.inner.seek(pos)
    }
}

impl<W: ?Sized + Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        if !self.panicked {
            let _ = self.flush_buf();
        }
    }
}

impl<W: ?Sized + Write + fmt::Debug> fmt::Debug for BufWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufWriter")
            .field("writer", &&self.inner)
            .field(
                "buffer",
                &format_args!("{}/{}", self.buf.len(), self.buf.capacity()),
            )
            .finish()
    }
}

/// The writer [`BufWriter::into_inner`] hands back with the error of its final flush.
#[derive(Debug)]
pub struct IntoInnerError<W>(W, Error);

impl<W> IntoInnerError<W> {
    pub const fn error(&self) -> &Error {
        &self.1
    }

    pub fn into_inner(self) -> W {
        self.0
    }

    pub fn into_error(self) -> Error {
        self.1
    }

    pub fn into_parts(self) -> (Error, W) {
        (self.1, self.0)
    }
}

impl<W> From<IntoInnerError<W>> for Error {
    fn from(error: IntoInnerError<W>) -> Self {
        error.1
    }
}

impl<W> fmt::Display for IntoInnerError<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.1, f)
    }
}

impl<W: fmt::Debug> error::Error for IntoInnerError<W> {}

/// A writer that delivers its buffer whenever a newline passes through.
pub struct LineWriter<W: ?Sized + Write> {
    inner: BufWriter<W>,
}

impl<W: Write> LineWriter<W> {
    pub fn new(inner: W) -> Self {
        Self::with_capacity(LINE_WRITER_CAPACITY, inner)
    }

    pub fn with_capacity(capacity: usize, inner: W) -> Self {
        Self {
            inner: BufWriter::with_capacity(capacity, inner),
        }
    }

    pub fn into_inner(self) -> core::result::Result<W, IntoInnerError<Self>> {
        self.inner
            .into_inner()
            .map_err(|IntoInnerError(inner, e)| IntoInnerError(Self { inner }, e))
    }
}

impl<W: ?Sized + Write> LineWriter<W> {
    pub const fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }

    pub const fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }
}

impl<W: ?Sized + Write> Write for LineWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write_all(buf)?;
        if buf.contains(&b'\n') {
            self.inner.flush_buf()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W: ?Sized + Write + fmt::Debug> fmt::Debug for LineWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineWriter")
            .field("writer", &self.get_ref())
            .finish_non_exhaustive()
    }
}
