use core::{fmt, time::Duration};

use super::{LockResult, MutexGuard};
use crate::thread;

/// A condition variable for the one guest thread.
///
/// The guest thread is the only thread, so a wait answers at once, as a spurious wakeup, and a
/// timed wait sleeps for its timeout. A wait on a condition that still holds traps, since the
/// condition keeps its value for good.
#[derive(Default)]
pub struct Condvar {
    _private: (),
}

/// Whether a timed wait of a [`Condvar`] reached its timeout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitTimeoutResult(bool);

impl WaitTimeoutResult {
    #[must_use]
    pub const fn timed_out(&self) -> bool {
        self.0
    }
}

impl Condvar {
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> LockResult<MutexGuard<'a, T>> {
        Ok(guard)
    }

    pub fn wait_while<'a, T, F>(
        &self,
        mut guard: MutexGuard<'a, T>,
        mut condition: F,
    ) -> LockResult<MutexGuard<'a, T>>
    where
        F: FnMut(&mut T) -> bool,
    {
        assert!(
            !condition(&mut *guard),
            "the one guest thread waits on a condition only another thread changes"
        );
        Ok(guard)
    }

    pub fn wait_timeout<'a, T>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
    ) -> LockResult<(MutexGuard<'a, T>, WaitTimeoutResult)> {
        thread::sleep(dur);
        Ok((guard, WaitTimeoutResult(true)))
    }

    pub fn wait_timeout_while<'a, T, F>(
        &self,
        mut guard: MutexGuard<'a, T>,
        dur: Duration,
        mut condition: F,
    ) -> LockResult<(MutexGuard<'a, T>, WaitTimeoutResult)>
    where
        F: FnMut(&mut T) -> bool,
    {
        if condition(&mut *guard) {
            thread::sleep(dur);
            Ok((guard, WaitTimeoutResult(true)))
        } else {
            Ok((guard, WaitTimeoutResult(false)))
        }
    }

    pub const fn notify_one(&self) {}

    pub const fn notify_all(&self) {}
}

impl fmt::Debug for Condvar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Condvar").finish_non_exhaustive()
    }
}
