//! Synchronization for the one guest thread.
//!
//! A PolyASM guest runs one thread, so a lock tracks whether that thread holds it: a second
//! acquisition by the holder traps, where a multi-threaded std waits forever. Locks stay
//! unpoisoned, since a panic ends the guest.

mod condvar;
mod lazy_lock;
pub mod mpsc;
mod mutex;
mod once;
mod once_lock;
mod poison;
mod rwlock;

pub use alloc_crate::sync::{Arc, Weak};
pub use condvar::{Condvar, WaitTimeoutResult};
pub use core::sync::atomic;
pub use lazy_lock::LazyLock;
pub use mutex::{Mutex, MutexGuard};
pub use once::{Once, OnceState};
pub use once_lock::OnceLock;
pub use poison::{LockResult, PoisonError, TryLockError, TryLockResult};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
