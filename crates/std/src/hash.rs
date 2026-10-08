//! The std `RandomState`, keyed from the entropy of the host.

use core::{cell::Cell, fmt};

#[expect(deprecated)]
pub use core::hash::SipHasher;
pub use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

use core::hash::SipHasher13;

use crate::{consts::FALLBACK_KEYS, host::installed, sys::GuestCell};

static KEYS: GuestCell<Cell<Option<(u64, u64)>>> = GuestCell::new(Cell::new(None));

/// Hasher keys: seeded once from [`crate::host::Host::fill_random`], then advanced per call so
/// each [`RandomState`] differs.
fn next_keys() -> (u64, u64) {
    let keys = KEYS.get().or_else(|| {
        let mut seed = [0; 16];
        installed()?.fill_random(&mut seed).ok()?;
        let (k0, k1) = seed.split_at(8);
        Some((
            u64::from_ne_bytes(k0.try_into().ok()?),
            u64::from_ne_bytes(k1.try_into().ok()?),
        ))
    });
    match keys {
        Some((k0, k1)) => {
            KEYS.set(Some((k0.wrapping_add(1), k1)));
            (k0, k1)
        }
        None => FALLBACK_KEYS,
    }
}

/// The default [`BuildHasher`] of [`crate::collections::HashMap`], with random keys.
#[derive(Clone)]
pub struct RandomState {
    k0: u64,
    k1: u64,
}

impl RandomState {
    #[must_use]
    pub fn new() -> Self {
        let (k0, k1) = next_keys();
        Self { k0, k1 }
    }
}

impl Default for RandomState {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildHasher for RandomState {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> DefaultHasher {
        DefaultHasher(SipHasher13::new_with_keys(self.k0, self.k1))
    }
}

impl fmt::Debug for RandomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RandomState").finish_non_exhaustive()
    }
}

/// The default [`Hasher`]: SipHash-1-3.
#[derive(Clone, Debug)]
pub struct DefaultHasher(SipHasher13);

impl DefaultHasher {
    /// A state with zero keys.
    #[must_use]
    pub fn new() -> Self {
        Self(SipHasher13::new_with_keys(0, 0))
    }
}

impl Default for DefaultHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for DefaultHasher {
    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
    }

    fn finish(&self) -> u64 {
        self.0.finish()
    }
}
