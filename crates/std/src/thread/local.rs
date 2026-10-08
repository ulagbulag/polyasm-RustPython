use core::{
    cell::{Cell, OnceCell, RefCell},
    error, fmt,
};

/// A key to a value of the guest thread, declared with [`thread_local!`].
///
/// The value lives in a `static`; the `Sync` claim rests on the one guest thread.
pub struct LocalKey<T: 'static> {
    cell: OnceCell<T>,
    init: fn() -> T,
}

// SAFETY: a PolyASM guest runs one thread, so the value is reached from that thread alone.
unsafe impl<T> Sync for LocalKey<T> {}

/// The error of [`LocalKey::try_with`]; the value of the guest thread stays reachable for the
/// whole run, so every call answers `Ok`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessError;

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("already destroyed")
    }
}

impl error::Error for AccessError {}

impl<T: 'static> LocalKey<T> {
    #[doc(hidden)]
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            cell: OnceCell::new(),
            init,
        }
    }

    /// Runs `f` on the value, initializing it on first access.
    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        f(self.cell.get_or_init(self.init))
    }

    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        Ok(self.with(f))
    }

    /// The value when initialized; otherwise `value` becomes the value and `None` answers.
    fn initialized_or_set(&'static self, value: T) -> Option<(&'static T, T)> {
        match self.cell.get() {
            Some(current) => Some((current, value)),
            None => {
                let _ = self.cell.set(value);
                None
            }
        }
    }
}

impl<T: 'static> LocalKey<Cell<T>> {
    pub fn set(&'static self, value: T) {
        if let Some((cell, value)) = self.initialized_or_set(Cell::new(value)) {
            cell.set(value.into_inner());
        }
    }

    pub fn get(&'static self) -> T
    where
        T: Copy,
    {
        self.with(Cell::get)
    }

    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(Cell::take)
    }

    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }
}

impl<T: 'static> LocalKey<RefCell<T>> {
    pub fn with_borrow<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.with(|cell| f(&cell.borrow()))
    }

    pub fn with_borrow_mut<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.with(|cell| f(&mut cell.borrow_mut()))
    }

    pub fn set(&'static self, value: T) {
        if let Some((cell, value)) = self.initialized_or_set(RefCell::new(value)) {
            *cell.borrow_mut() = value.into_inner();
        }
    }

    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(RefCell::take)
    }

    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }
}

impl<T: 'static> fmt::Debug for LocalKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalKey").finish_non_exhaustive()
    }
}

/// Declares values of the guest thread, each a `static` [`LocalKey`].
pub macro thread_local {
    () => {},
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const $init:block; $($rest:tt)*) => {
        $crate::thread::thread_local!($(#[$attr])* $vis static $name: $t = $init);
        $crate::thread::thread_local!($($rest)*);
    },
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const $init:block) => {
        $crate::thread::thread_local!($(#[$attr])* $vis static $name: $t = $init);
    },
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => {
        $crate::thread::thread_local!($(#[$attr])* $vis static $name: $t = $init);
        $crate::thread::thread_local!($($rest)*);
    },
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => {
        $(#[$attr])*
        $vis static $name: $crate::thread::LocalKey<$t> = $crate::thread::LocalKey::new({
            fn init() -> $t {
                $init
            }
            init
        });
    },
}
