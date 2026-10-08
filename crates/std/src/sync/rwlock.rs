use core::{
    cell::{Cell, UnsafeCell},
    fmt,
    ops::{Deref, DerefMut},
};

use super::{LockResult, TryLockError, TryLockResult};

/// The state of a [`RwLock`] held for writing.
const WRITER: isize = -1;

/// A reader-writer lock for the one guest thread.
pub struct RwLock<T: ?Sized> {
    /// 0 when free, the reader count when read-locked, [`WRITER`] when write-locked.
    state: Cell<isize>,
    data: UnsafeCell<T>,
}

// SAFETY: a PolyASM guest runs one thread, so the lock is reached from that thread alone.
unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
// SAFETY: as above.
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

/// A shared hold of a [`RwLock`].
#[must_use = "if unused the RwLock will immediately unlock"]
pub struct RwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

/// An exclusive hold of a [`RwLock`].
#[must_use = "if unused the RwLock will immediately unlock"]
pub struct RwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

impl<T> RwLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: Cell::new(0),
            data: UnsafeCell::new(value),
        }
    }

    pub fn into_inner(self) -> LockResult<T> {
        Ok(self.data.into_inner())
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Locks for reading. The one guest thread reading while it writes traps.
    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        match self.try_read() {
            Ok(guard) => Ok(guard),
            Err(_) => panic!("the one guest thread reads a RwLock it writes"),
        }
    }

    /// Locks for writing. The one guest thread writing while it holds the lock traps.
    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        match self.try_write() {
            Ok(guard) => Ok(guard),
            Err(_) => panic!("the one guest thread writes a RwLock it holds"),
        }
    }

    pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
        let state = self.state.get();
        if state == WRITER {
            Err(TryLockError::WouldBlock)
        } else {
            self.state.set(state + 1);
            Ok(RwLockReadGuard { lock: self })
        }
    }

    pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
        if self.state.get() == 0 {
            self.state.set(WRITER);
            Ok(RwLockWriteGuard { lock: self })
        } else {
            Err(TryLockError::WouldBlock)
        }
    }

    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        false
    }

    pub const fn clear_poison(&self) {}

    pub const fn get_mut(&mut self) -> LockResult<&mut T> {
        Ok(self.data.get_mut())
    }
}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: a read hold excludes writers, so the data stays unaliased by `&mut`.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.set(self.lock.state.get() - 1);
    }
}

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the write hold is exclusive.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the write hold is exclusive.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.set(0);
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> From<T> for RwLock<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("RwLock");
        match self.try_read() {
            Ok(guard) => d.field("data", &&*guard),
            Err(_) => d.field("data", &format_args!("<locked>")),
        };
        d.field("poisoned", &false).finish_non_exhaustive()
    }
}
