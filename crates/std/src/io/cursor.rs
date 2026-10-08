use alloc_crate::{boxed::Box, vec::Vec};

use super::{BufRead, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};

const NEGATIVE_POSITION: Error = Error::const_message(
    ErrorKind::InvalidInput,
    "invalid seek to a negative or overflowing position",
);

/// An in-memory buffer with a position, readable, writable and seekable.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Cursor<T> {
    inner: T,
    pos: u64,
}

impl<T> Cursor<T> {
    pub const fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
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

    pub const fn position(&self) -> u64 {
        self.pos
    }

    pub const fn set_position(&mut self, pos: u64) {
        self.pos = pos;
    }
}

impl<T: AsRef<[u8]>> Cursor<T> {
    fn remaining(&self) -> &[u8] {
        let inner = self.inner.as_ref();
        let start = usize::try_from(self.pos).map_or(inner.len(), |pos| pos.min(inner.len()));
        &inner[start..]
    }
}

impl<T: AsRef<[u8]>> Seek for Cursor<T> {
    fn seek(&mut self, style: SeekFrom) -> Result<u64> {
        let (base, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.inner.as_ref().len() as u64, n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        let pos = base.checked_add_signed(offset).ok_or(NEGATIVE_POSITION)?;
        self.pos = pos;
        Ok(pos)
    }
}

impl<T: AsRef<[u8]>> Read for Cursor<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = Read::read(&mut self.remaining(), buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl<T: AsRef<[u8]>> BufRead for Cursor<T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(self.remaining())
    }

    fn consume(&mut self, amount: usize) {
        self.pos += amount as u64;
    }
}

/// Writes `buf` at `pos` of a fixed slice, answering how many bytes fit.
fn slice_write(pos: &mut u64, slice: &mut [u8], buf: &[u8]) -> Result<usize> {
    let start = usize::try_from(*pos).map_or(slice.len(), |p| p.min(slice.len()));
    let amount = (&mut slice[start..]).write(buf)?;
    *pos += amount as u64;
    Ok(amount)
}

/// Writes `buf` at `pos` of a growable vector, zero-filling any gap.
fn vec_write(pos: &mut u64, vec: &mut Vec<u8>, buf: &[u8]) -> Result<usize> {
    let start = usize::try_from(*pos).map_err(|_| NEGATIVE_POSITION)?;
    let end = start.checked_add(buf.len()).ok_or(NEGATIVE_POSITION)?;
    if vec.len() < start {
        vec.resize(start, 0);
    }
    let overlap = vec.len().min(end) - start;
    vec[start..start + overlap].copy_from_slice(&buf[..overlap]);
    vec.extend_from_slice(&buf[overlap..]);
    *pos = end as u64;
    Ok(buf.len())
}

impl Write for Cursor<&mut [u8]> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        slice_write(&mut self.pos, self.inner, buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for Cursor<&mut Vec<u8>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        vec_write(&mut self.pos, self.inner, buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for Cursor<Vec<u8>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        vec_write(&mut self.pos, &mut self.inner, buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for Cursor<Box<[u8]>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        slice_write(&mut self.pos, &mut self.inner, buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<const N: usize> Write for Cursor<[u8; N]> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        slice_write(&mut self.pos, &mut self.inner, buf)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
