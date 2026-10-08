//! The one guest thread: sleeping, identity, and thread-local storage.
//!
//! A PolyASM guest runs one thread. [`spawn`] and [`Builder::spawn`] answer
//! [`io::ErrorKind::Unsupported`]; `thread_local!` values live in statics.

mod local;

use alloc_crate::{boxed::Box, string::String};
use core::{any::Any, convert::Infallible, fmt, marker::PhantomData, num::NonZero, time::Duration};

use crate::{consts::MAIN_THREAD_NAME, host::host, io};

pub use local::{AccessError, LocalKey, thread_local};

/// The answer of [`JoinHandle::join`].
pub type Result<T> = core::result::Result<T, Box<dyn Any + Send + 'static>>;

/// Suspends the guest for `dur`.
pub fn sleep(dur: Duration) {
    host()
        .sleep(dur)
        .unwrap_or_else(|e| panic!("the sleep of the host answers: {e}"));
}

/// Answers at once: the guest thread is the only thread.
pub const fn yield_now() {}

/// Whether the thread unwinds from a panic; a panic traps the guest, so this answers `false`.
#[must_use]
pub const fn panicking() -> bool {
    false
}

/// Answers at once: the guest thread runs alone.
pub const fn park() {}

/// Sleeps for the whole `dur`: the guest thread runs alone.
pub fn park_timeout(dur: Duration) {
    sleep(dur);
}

/// The number of threads that run at once: one.
pub const fn available_parallelism() -> io::Result<NonZero<usize>> {
    Ok(NonZero::<usize>::MIN)
}

/// The identifier of a thread.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ThreadId(NonZero<u64>);

impl ThreadId {
    #[must_use]
    pub const fn as_u64(&self) -> NonZero<u64> {
        self.0
    }
}

/// A handle of a thread.
#[derive(Clone, Debug)]
pub struct Thread {
    _private: (),
}

impl Thread {
    #[must_use]
    pub const fn id(&self) -> ThreadId {
        ThreadId(NonZero::<u64>::MIN)
    }

    #[must_use]
    pub const fn name(&self) -> Option<&str> {
        Some(MAIN_THREAD_NAME)
    }

    pub const fn unpark(&self) {}
}

/// The handle of the guest thread.
#[must_use]
pub const fn current() -> Thread {
    Thread { _private: () }
}

/// Configuration of a new thread. [`Builder::spawn`] answers [`io::ErrorKind::Unsupported`].
#[derive(Debug, Default)]
pub struct Builder {
    name: Option<String>,
    stack_size: Option<usize>,
}

impl Builder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: None,
            stack_size: None,
        }
    }

    #[must_use]
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    #[must_use]
    pub const fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = Some(size);
        self
    }

    /// Answers [`io::ErrorKind::Unsupported`]: the guest runs one thread.
    pub fn spawn<F, T>(self, f: F) -> io::Result<JoinHandle<T>>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let _ = (self.name, self.stack_size, f);
        Err(io::Error::const_message(
            io::ErrorKind::Unsupported,
            "a PolyASM guest runs one thread",
        ))
    }
}

/// Spawns a thread; the guest runs one thread, so this panics as std does when spawning fails.
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Builder::new()
        .spawn(f)
        .unwrap_or_else(|e| panic!("failed to spawn thread: {e}"))
}

/// The handle of a spawned thread. The guest runs one thread, so [`spawn`] answers an error and
/// this type stays uninhabited.
pub struct JoinHandle<T> {
    never: Infallible,
    _marker: PhantomData<T>,
}

impl<T> JoinHandle<T> {
    pub fn join(self) -> Result<T> {
        match self.never {}
    }

    #[must_use]
    pub const fn thread(&self) -> &Thread {
        match self.never {}
    }

    #[must_use]
    pub const fn is_finished(&self) -> bool {
        match self.never {}
    }
}

impl<T> fmt::Debug for JoinHandle<T> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.never {}
    }
}
