//! The `parking_lot` surface of the one PolyASM guest thread.
//!
//! A module that names `parking_lot` on other targets imports this module under that name on
//! PolyASM. Each lock tracks whether the one guest thread holds it, as the cell locks do; a
//! guest has one thread, the ground for the `Sync` claims a `static` lock needs.

use core::{num::NonZero, time::Duration};

pub use lock_api;

use super::cell_lock::{RawCellMutex, RawCellRwLock};

/// A raw mutex of the one guest thread.
pub struct RawMutex(RawCellMutex);

// SAFETY: a PolyASM guest runs one thread, so the lock is reached from that thread alone.
unsafe impl Sync for RawMutex {}
// SAFETY: as above.
unsafe impl Send for RawMutex {}

// SAFETY: every method forwards to the cell mutex.
unsafe impl lock_api::RawMutex for RawMutex {
    #[allow(
        clippy::declare_interior_mutable_const,
        reason = "const lock initializer intentionally uses interior mutability"
    )]
    const INIT: Self = Self(<RawCellMutex as lock_api::RawMutex>::INIT);

    type GuardMarker = lock_api::GuardSend;

    fn lock(&self) {
        self.0.lock();
    }

    fn try_lock(&self) -> bool {
        self.0.try_lock()
    }

    unsafe fn unlock(&self) {
        // SAFETY: the caller holds the lock.
        unsafe { self.0.unlock() }
    }

    fn is_locked(&self) -> bool {
        self.0.is_locked()
    }
}

/// A raw reader-writer lock of the one guest thread.
pub struct RawRwLock(RawCellRwLock);

// SAFETY: a PolyASM guest runs one thread, so the lock is reached from that thread alone.
unsafe impl Sync for RawRwLock {}
// SAFETY: as above.
unsafe impl Send for RawRwLock {}

// SAFETY: every method forwards to the cell reader-writer lock.
unsafe impl lock_api::RawRwLock for RawRwLock {
    #[allow(
        clippy::declare_interior_mutable_const,
        reason = "const rwlock initializer intentionally uses interior mutability"
    )]
    const INIT: Self = Self(<RawCellRwLock as lock_api::RawRwLock>::INIT);

    type GuardMarker = lock_api::GuardSend;

    fn lock_shared(&self) {
        self.0.lock_shared();
    }

    fn try_lock_shared(&self) -> bool {
        self.0.try_lock_shared()
    }

    unsafe fn unlock_shared(&self) {
        // SAFETY: the caller holds a shared lock.
        unsafe { self.0.unlock_shared() }
    }

    fn lock_exclusive(&self) {
        self.0.lock_exclusive();
    }

    fn try_lock_exclusive(&self) -> bool {
        self.0.try_lock_exclusive()
    }

    unsafe fn unlock_exclusive(&self) {
        // SAFETY: the caller holds the exclusive lock.
        unsafe { self.0.unlock_exclusive() }
    }

    fn is_locked(&self) -> bool {
        self.0.is_locked()
    }
}

/// The id of the one guest thread.
pub struct RawThreadId;

// SAFETY: the one guest thread keeps one id for its whole life.
unsafe impl lock_api::GetThreadId for RawThreadId {
    const INIT: Self = Self;

    fn nonzero_thread_id(&self) -> NonZero<usize> {
        NonZero::<usize>::MIN
    }
}

pub type Mutex<T> = lock_api::Mutex<RawMutex, T>;
pub type MutexGuard<'a, T> = lock_api::MutexGuard<'a, RawMutex, T>;
pub type MappedMutexGuard<'a, T> = lock_api::MappedMutexGuard<'a, RawMutex, T>;
pub type RwLock<T> = lock_api::RwLock<RawRwLock, T>;
pub type RwLockReadGuard<'a, T> = lock_api::RwLockReadGuard<'a, RawRwLock, T>;
pub type RwLockWriteGuard<'a, T> = lock_api::RwLockWriteGuard<'a, RawRwLock, T>;
pub type ReentrantMutex<T> = lock_api::ReentrantMutex<RawMutex, RawThreadId, T>;
pub type ReentrantMutexGuard<'a, T> = lock_api::ReentrantMutexGuard<'a, RawMutex, RawThreadId, T>;

/// Creates a mutex in a `const` context.
pub const fn const_mutex<T>(value: T) -> Mutex<T> {
    Mutex::const_new(<RawMutex as lock_api::RawMutex>::INIT, value)
}

/// The answer of a timed wait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitTimeoutResult(bool);

impl WaitTimeoutResult {
    /// Whether the wait ended by its timeout.
    #[must_use]
    pub const fn timed_out(self) -> bool {
        self.0
    }
}

/// A condition variable of the one guest thread.
///
/// The thread that waits is the thread that would notify, so a wait with a timeout sleeps
/// through it and reports the timeout, and an unbounded wait traps at once.
#[derive(Debug, Default)]
pub struct Condvar(());

impl Condvar {
    #[must_use]
    pub const fn new() -> Self {
        Self(())
    }

    /// Notifies one waiter: the one guest thread holds zero waiters, so the answer is `false`.
    pub fn notify_one(&self) -> bool {
        false
    }

    /// Notifies every waiter: the one guest thread holds zero waiters.
    pub fn notify_all(&self) -> usize {
        0
    }

    /// Waits for a notification that the waiting thread alone sends: a deadlock, which
    /// traps.
    pub fn wait<T: ?Sized>(&self, _guard: &mut MutexGuard<'_, T>) {
        panic!("deadlock: the one guest thread waits on a Condvar with an unbounded wait")
    }

    /// Waits out `timeout` with the mutex unlocked, then reports the timeout.
    pub fn wait_for<T: ?Sized>(
        &self,
        guard: &mut MutexGuard<'_, T>,
        timeout: Duration,
    ) -> WaitTimeoutResult {
        MutexGuard::unlocked(guard, || std::thread::sleep(timeout));
        WaitTimeoutResult(true)
    }

    /// Waits until `deadline` with the mutex unlocked, then reports the timeout.
    pub fn wait_until<T: ?Sized>(
        &self,
        guard: &mut MutexGuard<'_, T>,
        deadline: std::time::Instant,
    ) -> WaitTimeoutResult {
        let timeout = deadline.saturating_duration_since(std::time::Instant::now());
        self.wait_for(guard, timeout)
    }
}
