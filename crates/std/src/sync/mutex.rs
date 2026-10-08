use core::{
    cell::{Cell, UnsafeCell},
    fmt,
    ops::{Deref, DerefMut},
};

use super::{LockResult, TryLockError, TryLockResult};

/// A mutual exclusion lock for the one guest thread.
pub struct Mutex<T: ?Sized> {
    locked: Cell<bool>,
    data: UnsafeCell<T>,
}

// SAFETY: a PolyASM guest runs one thread, so the lock is reached from that thread alone.
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
// SAFETY: as above.
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

/// The holder of a locked [`Mutex`]; dropping it unlocks.
#[must_use = "if unused the Mutex will immediately unlock"]
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a Mutex<T>,
}

// SAFETY: a PolyASM guest runs one thread.
unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: Cell::new(false),
            data: UnsafeCell::new(value),
        }
    }

    pub fn into_inner(self) -> LockResult<T> {
        Ok(self.data.into_inner())
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Locks the mutex. The one guest thread locking a mutex it holds traps.
    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        assert!(
            !self.locked.replace(true),
            "the one guest thread locks a Mutex it already holds"
        );
        Ok(MutexGuard { lock: self })
    }

    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        if self.locked.replace(true) {
            Err(TryLockError::WouldBlock)
        } else {
            Ok(MutexGuard { lock: self })
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

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the guard holds the lock, so this reference is the only one to the data.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the guard holds the lock, so this reference is the only one to the data.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.set(false);
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Mutex");
        match self.try_lock() {
            Ok(guard) => d.field("data", &&*guard),
            Err(_) => d.field("data", &format_args!("<locked>")),
        };
        d.field("poisoned", &false).finish_non_exhaustive()
    }
}
