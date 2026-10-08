use super::{BufRead, Read, Result, Seek, SeekFrom, Write};
use crate::consts::DEFAULT_BUF_SIZE;

/// Copies every byte of `reader` into `writer`, answering the count.
pub fn copy<R: ?Sized + Read, W: ?Sized + Write>(reader: &mut R, writer: &mut W) -> Result<u64> {
    let mut buf = [0; DEFAULT_BUF_SIZE];
    let mut copied = 0;
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => return Ok(copied),
            Ok(n) => n,
            Err(e) if e.is_interrupted() => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..n])?;
        copied += n as u64;
    }
}

/// A reader at its end and a writer that takes everything.
#[derive(Clone, Copy, Debug, Default)]
pub struct Empty;

/// The reader at its end and writer that takes everything.
#[must_use]
pub const fn empty() -> Empty {
    Empty
}

impl Read for Empty {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}

impl BufRead for Empty {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(&[])
    }

    fn consume(&mut self, _amount: usize) {}
}

impl Seek for Empty {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        Ok(0)
    }
}

impl Write for Empty {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A reader that yields one byte forever.
#[derive(Clone, Copy, Debug)]
pub struct Repeat {
    byte: u8,
}

/// The reader that yields `byte` forever.
#[must_use]
pub const fn repeat(byte: u8) -> Repeat {
    Repeat { byte }
}

impl Read for Repeat {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        buf.fill(self.byte);
        Ok(buf.len())
    }
}

/// A writer that takes and drops everything.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sink;

/// The writer that takes and drops everything.
#[must_use]
pub const fn sink() -> Sink {
    Sink
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for &Sink {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
