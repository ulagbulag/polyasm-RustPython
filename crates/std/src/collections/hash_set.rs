//! The std `HashSet` over `hashbrown`, with the std signatures.

use core::{
    fmt,
    hash::{BuildHasher, Hash},
    ops::{BitAnd, BitOr, BitXor, Deref, DerefMut, Sub},
};

use super::hash_map::RandomState;

pub use hashbrown::hash_set::{
    Difference, Drain, Intersection, IntoIter, Iter, SymmetricDifference, Union,
};

/// A keyed set. The methods of `hashbrown::HashSet` serve through `Deref`.
pub struct HashSet<T, S = RandomState> {
    base: hashbrown::HashSet<T, S>,
}

impl<T> HashSet<T, RandomState> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<T, S> HashSet<T, S> {
    pub const fn with_hasher(state: S) -> Self {
        Self {
            base: hashbrown::HashSet::with_hasher(state),
        }
    }

    pub fn with_capacity_and_hasher(capacity: usize, state: S) -> Self {
        Self {
            base: hashbrown::HashSet::with_capacity_and_hasher(capacity, state),
        }
    }
}

impl<T, S> Deref for HashSet<T, S> {
    type Target = hashbrown::HashSet<T, S>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T, S> DerefMut for HashSet<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<T: Clone, S: Clone> Clone for HashSet<T, S> {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl<T: fmt::Debug, S> fmt::Debug for HashSet<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.base, f)
    }
}

impl<T, S: Default> Default for HashSet<T, S> {
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<T: Eq + Hash, S: BuildHasher> PartialEq for HashSet<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}

impl<T: Eq + Hash, S: BuildHasher> Eq for HashSet<T, S> {}

impl<T: Eq + Hash, S: BuildHasher + Default> FromIterator<T> for HashSet<T, S> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            base: hashbrown::HashSet::from_iter(iter),
        }
    }
}

impl<T: Eq + Hash, S: BuildHasher> Extend<T> for HashSet<T, S> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.base.extend(iter);
    }
}

impl<'a, T: 'a + Eq + Hash + Copy, S: BuildHasher> Extend<&'a T> for HashSet<T, S> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.base.extend(iter);
    }
}

impl<T: Eq + Hash, const N: usize> From<[T; N]> for HashSet<T, RandomState> {
    fn from(items: [T; N]) -> Self {
        items.into_iter().collect()
    }
}

impl<T, S> IntoIterator for HashSet<T, S> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        self.base.into_iter()
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.base.iter()
    }
}

macro_rules! set_op {
    ($op:ident, $method:ident, $combine:ident) => {
        impl<T, S> $op<&HashSet<T, S>> for &HashSet<T, S>
        where
            T: Eq + Hash + Clone,
            S: BuildHasher + Default,
        {
            type Output = HashSet<T, S>;

            fn $method(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
                self.base.$combine(&rhs.base).cloned().collect()
            }
        }
    };
}

set_op!(BitOr, bitor, union);
set_op!(BitAnd, bitand, intersection);
set_op!(BitXor, bitxor, symmetric_difference);
set_op!(Sub, sub, difference);
