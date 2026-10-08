use alloc_crate::{boxed::Box, collections::VecDeque, string::String, vec::Vec};
use core::{fmt, mem};

use super::{BufRead, EOF_BEFORE_FILL, IoSlice, IoSliceMut, Read, Result, Seek, SeekFrom, Write};

impl<R: Read + ?Sized> Read for &mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        (**self).read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact(buf)
    }
}

impl<W: Write + ?Sized> Write for &mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        (**self).write_vectored(bufs)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        (**self).write_fmt(args)
    }
}

impl<S: Seek + ?Sized> Seek for &mut S {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }
}

impl<B: BufRead + ?Sized> BufRead for &mut B {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        (**self).consume(amount);
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_until(byte, buf)
    }

    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_line(buf)
    }
}

impl<R: Read + ?Sized> Read for Box<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact(buf)
    }
}

impl<W: Write + ?Sized> Write for Box<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        (**self).write_fmt(args)
    }
}

impl<S: Seek + ?Sized> Seek for Box<S> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }
}

impl<B: BufRead + ?Sized> BufRead for Box<B> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        (**self).consume(amount);
    }
}

impl Read for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let amount = buf.len().min(self.len());
        let (head, tail) = self.split_at(amount);
        buf[..amount].copy_from_slice(head);
        *self = tail;
        Ok(amount)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        if buf.len() > self.len() {
            *self = &self[self.len()..];
            return Err(EOF_BEFORE_FILL);
        }
        let (head, tail) = self.split_at(buf.len());
        buf.copy_from_slice(head);
        *self = tail;
        Ok(())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let len = self.len();
        buf.extend_from_slice(self);
        *self = &self[len..];
        Ok(len)
    }
}

impl BufRead for &[u8] {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(*self)
    }

    fn consume(&mut self, amount: usize) {
        *self = &self[amount..];
    }
}

impl Write for &mut [u8] {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let amount = data.len().min(self.len());
        let (head, tail) = mem::take(self).split_at_mut(amount);
        head.copy_from_slice(&data[..amount]);
        *self = tail;
        Ok(amount)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.extend_from_slice(buf);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Read for VecDeque<u8> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let (mut front, _) = self.as_slices();
        let n = Read::read(&mut front, buf)?;
        self.drain(..n);
        Ok(n)
    }
}

impl Write for VecDeque<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
