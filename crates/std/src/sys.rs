//! Crate-wide state of the one guest thread.

use core::ops::Deref;

/// A value shared across the guest.
///
/// A PolyASM guest runs exactly one thread, so every access happens on that thread; this is
/// the ground for the `Sync` claim that lets a `static` hold a `Cell` or a `RefCell`.
pub(crate) struct GuestCell<T>(T);

// SAFETY: a PolyASM guest runs one thread, so the value is reached from that thread alone.
unsafe impl<T> Sync for GuestCell<T> {}

impl<T> GuestCell<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for GuestCell<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}
