//! The std `HashMap` over `hashbrown`, with the std signatures, and its `RandomState`, keyed from
//! the entropy of the host.

use core::{
    borrow::Borrow,
    cell::Cell,
    fmt,
    hash::{BuildHasher, Hash, Hasher, SipHasher13},
    ops::{Deref, DerefMut, Index},
};

pub use hashbrown::hash_map::{
    Drain, IntoIter, IntoKeys, IntoValues, Iter, IterMut, Keys, Values, ValuesMut,
};

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

/// The default [`BuildHasher`] of [`HashMap`], with random keys.
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

/// A view into one entry of a [`HashMap`], made by [`HashMap::entry`].
pub type Entry<'a, K, V> = hashbrown::hash_map::RustcEntry<'a, K, V>;
/// An occupied [`Entry`].
pub type OccupiedEntry<'a, K, V> = hashbrown::hash_map::RustcOccupiedEntry<'a, K, V>;
/// A vacant [`Entry`].
pub type VacantEntry<'a, K, V> = hashbrown::hash_map::RustcVacantEntry<'a, K, V>;

/// A keyed map. The methods of `hashbrown::HashMap` serve through `Deref`.
pub struct HashMap<K, V, S = RandomState> {
    base: hashbrown::HashMap<K, V, S>,
}

impl<K, V> HashMap<K, V, RandomState> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K, V, S> HashMap<K, V, S> {
    pub const fn with_hasher(hash_builder: S) -> Self {
        Self {
            base: hashbrown::HashMap::with_hasher(hash_builder),
        }
    }

    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            base: hashbrown::HashMap::with_capacity_and_hasher(capacity, hash_builder),
        }
    }

    pub fn into_keys(self) -> IntoKeys<K, V> {
        self.base.into_keys()
    }

    pub fn into_values(self) -> IntoValues<K, V> {
        self.base.into_values()
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> HashMap<K, V, S> {
    /// The entry of `key`, for in-place insertion or update.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        self.base.rustc_entry(key)
    }
}

impl<K, V, S> Deref for HashMap<K, V, S> {
    type Target = hashbrown::HashMap<K, V, S>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<K, V, S> DerefMut for HashMap<K, V, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for HashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for HashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.base, f)
    }
}

impl<K, V, S: Default> Default for HashMap<K, V, S> {
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for HashMap<K, V, S> {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for HashMap<K, V, S> {}

impl<K, Q, V, S> Index<&Q> for HashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
    S: BuildHasher,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.base.get(key).expect("key present in the HashMap")
    }
}

impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)> for HashMap<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self {
            base: hashbrown::HashMap::from_iter(iter),
        }
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> Extend<(K, V)> for HashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.base.extend(iter);
    }
}

impl<'a, K: Eq + Hash + Copy, V: Copy, S: BuildHasher> Extend<(&'a K, &'a V)> for HashMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.base.extend(iter);
    }
}

impl<K: Eq + Hash, V, const N: usize> From<[(K, V); N]> for HashMap<K, V, RandomState> {
    fn from(entries: [(K, V); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> IntoIter<K, V> {
        self.base.into_iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.base.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> IterMut<'a, K, V> {
        self.base.iter_mut()
    }
}
