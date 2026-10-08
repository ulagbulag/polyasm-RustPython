use core::{error, fmt};

/// The error a poisoned lock answers. Guest locks stay unpoisoned; the type keeps the std
/// signatures.
pub struct PoisonError<T> {
    guard: T,
}

/// The error of a `try_lock` style call.
pub enum TryLockError<T> {
    Poisoned(PoisonError<T>),
    WouldBlock,
}

/// The answer of a lock acquisition.
pub type LockResult<T> = Result<T, PoisonError<T>>;

/// The answer of a `try_lock` style acquisition.
pub type TryLockResult<T> = Result<T, TryLockError<T>>;

impl<T> PoisonError<T> {
    pub const fn new(guard: T) -> Self {
        Self { guard }
    }

    pub fn into_inner(self) -> T {
        self.guard
    }

    pub const fn get_ref(&self) -> &T {
        &self.guard
    }

    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

impl<T> fmt::Debug for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PoisonError").finish_non_exhaustive()
    }
}

impl<T> fmt::Display for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("poisoned lock: another task failed inside")
    }
}

impl<T> error::Error for PoisonError<T> {}

impl<T> From<PoisonError<T>> for TryLockError<T> {
    fn from(err: PoisonError<T>) -> Self {
        Self::Poisoned(err)
    }
}

impl<T> fmt::Debug for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poisoned(..) => f.write_str("Poisoned(..)"),
            Self::WouldBlock => f.write_str("WouldBlock"),
        }
    }
}

impl<T> fmt::Display for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poisoned(..) => f.write_str("poisoned lock: another task failed inside"),
            Self::WouldBlock => f.write_str("try_lock failed because the operation would block"),
        }
    }
}

impl<T> error::Error for TryLockError<T> {}
