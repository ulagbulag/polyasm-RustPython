//! Descriptors of the embedder host: raw, owned and borrowed.

use alloc_crate::{boxed::Box, rc::Rc, sync::Arc};
use core::{fmt, marker::PhantomData, mem::ManuallyDrop};

use crate::{consts::ENOSYS, host::installed, io};

/// A raw descriptor number.
pub type RawFd = core::ffi::c_int;

/// Access to the raw descriptor of an object.
pub trait AsRawFd {
    fn as_raw_fd(&self) -> RawFd;
}

/// Construction of an object from a raw descriptor it takes over.
pub trait FromRawFd {
    /// # Safety
    ///
    /// `fd` is an open descriptor that the caller owns and hands over.
    unsafe fn from_raw_fd(fd: RawFd) -> Self;
}

/// Hand-over of the raw descriptor of an object.
pub trait IntoRawFd {
    fn into_raw_fd(self) -> RawFd;
}

/// A descriptor this value owns; dropping it closes the descriptor through the host.
#[repr(transparent)]
pub struct OwnedFd {
    fd: RawFd,
}

/// A descriptor borrowed for `'fd`.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct BorrowedFd<'fd> {
    fd: RawFd,
    _marker: PhantomData<&'fd OwnedFd>,
}

/// Borrowing the descriptor of an object.
pub trait AsFd {
    fn as_fd(&self) -> BorrowedFd<'_>;
}

impl BorrowedFd<'_> {
    /// # Safety
    ///
    /// `fd` stays open for the lifetime of the borrow.
    #[must_use]
    pub const unsafe fn borrow_raw(fd: RawFd) -> Self {
        Self {
            fd,
            _marker: PhantomData,
        }
    }

    /// A duplicate owned descriptor; duplication stays outside the host interface, so this
    /// answers `ENOSYS`.
    pub fn try_clone_to_owned(&self) -> io::Result<OwnedFd> {
        Err(io::Error::from_raw_os_error(ENOSYS))
    }
}

impl OwnedFd {
    /// A duplicate owned descriptor; duplication stays outside the host interface, so this
    /// answers `ENOSYS`.
    pub fn try_clone(&self) -> io::Result<Self> {
        self.as_fd().try_clone_to_owned()
    }
}

impl Drop for OwnedFd {
    fn drop(&mut self) {
        if let Some(host) = installed() {
            let _ = host.close(self.fd);
        }
    }
}

impl AsRawFd for OwnedFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl AsRawFd for BorrowedFd<'_> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl AsRawFd for RawFd {
    fn as_raw_fd(&self) -> RawFd {
        *self
    }
}

impl FromRawFd for OwnedFd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self { fd }
    }
}

impl FromRawFd for RawFd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        fd
    }
}

impl IntoRawFd for OwnedFd {
    fn into_raw_fd(self) -> RawFd {
        ManuallyDrop::new(self).fd
    }
}

impl IntoRawFd for RawFd {
    fn into_raw_fd(self) -> RawFd {
        self
    }
}

impl AsFd for OwnedFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        // SAFETY: `self` owns the descriptor for at least the borrow.
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

impl AsFd for BorrowedFd<'_> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        *self
    }
}

impl<T: AsFd + ?Sized> AsFd for &T {
    fn as_fd(&self) -> BorrowedFd<'_> {
        T::as_fd(self)
    }
}

impl<T: AsFd + ?Sized> AsFd for &mut T {
    fn as_fd(&self) -> BorrowedFd<'_> {
        T::as_fd(self)
    }
}

impl<T: AsFd + ?Sized> AsFd for Box<T> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        (**self).as_fd()
    }
}

impl<T: AsFd + ?Sized> AsFd for Rc<T> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        (**self).as_fd()
    }
}

impl<T: AsFd + ?Sized> AsFd for Arc<T> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        (**self).as_fd()
    }
}

impl fmt::Debug for OwnedFd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedFd").field("fd", &self.fd).finish()
    }
}

impl fmt::Debug for BorrowedFd<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedFd").field("fd", &self.fd).finish()
    }
}
