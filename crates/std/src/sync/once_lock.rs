use core::{cell::OnceCell, fmt};

/// A cell written once, for the one guest thread.
pub struct OnceLock<T> {
    cell: OnceCell<T>,
}

// SAFETY: a PolyASM guest runs one thread, so the cell is reached from that thread alone.
unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}
// SAFETY: as above.
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T> OnceLock<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cell: OnceCell::new(),
        }
    }

    pub fn get(&self) -> Option<&T> {
        self.cell.get()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.cell.get_mut()
    }

    /// Writes `value` into an empty cell; a full cell hands `value` back.
    pub fn set(&self, value: T) -> Result<(), T> {
        self.cell.set(value)
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        self.cell.get_or_init(f)
    }

    pub fn into_inner(self) -> Option<T> {
        self.cell.into_inner()
    }

    pub fn take(&mut self) -> Option<T> {
        self.cell.take()
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for OnceLock<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T> From<T> for OnceLock<T> {
    fn from(value: T) -> Self {
        Self {
            cell: OnceCell::from(value),
        }
    }
}

impl<T: PartialEq> PartialEq for OnceLock<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq> Eq for OnceLock<T> {}

impl<T: fmt::Debug> fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("OnceLock");
        match self.get() {
            Some(value) => d.field(value),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}
