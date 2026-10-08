use core::{cell::LazyCell, fmt, ops::Deref};

/// A value initialized on first access, for the one guest thread.
pub struct LazyLock<T, F = fn() -> T> {
    cell: LazyCell<T, F>,
}

// SAFETY: a PolyASM guest runs one thread, so the cell is reached from that thread alone.
unsafe impl<T: Sync + Send, F: Send> Sync for LazyLock<T, F> {}

impl<T, F: FnOnce() -> T> LazyLock<T, F> {
    pub const fn new(f: F) -> Self {
        Self {
            cell: LazyCell::new(f),
        }
    }

    pub fn force(this: &Self) -> &T {
        LazyCell::force(&this.cell)
    }
}

impl<T, F: FnOnce() -> T> Deref for LazyLock<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        Self::force(self)
    }
}

impl<T: Default> Default for LazyLock<T> {
    fn default() -> Self {
        Self::new(T::default)
    }
}

impl<T: fmt::Debug, F: FnOnce() -> T> fmt::Debug for LazyLock<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LazyLock").field(&**self).finish()
    }
}
