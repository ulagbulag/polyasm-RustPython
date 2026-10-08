use core::{cell::Cell, fmt};

#[derive(Clone, Copy, Eq, PartialEq)]
enum State {
    Incomplete,
    Running,
    Complete,
}

/// A one-time initialization for the one guest thread.
pub struct Once {
    state: Cell<State>,
}

// SAFETY: a PolyASM guest runs one thread, so the state is reached from that thread alone.
unsafe impl Sync for Once {}
// SAFETY: as above.
unsafe impl Send for Once {}

/// The state [`Once::call_once_force`] hands its closure.
#[derive(Debug)]
pub struct OnceState {
    _private: (),
}

impl OnceState {
    /// Whether an earlier call panicked; a panic ends the guest, so this answers `false`.
    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        false
    }
}

impl Once {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: Cell::new(State::Incomplete),
        }
    }

    /// Runs `f` on the first call. A call from inside `f` traps.
    pub fn call_once<F: FnOnce()>(&self, f: F) {
        self.call_once_force(|_| f());
    }

    pub fn call_once_force<F: FnOnce(&OnceState)>(&self, f: F) {
        match self.state.get() {
            State::Complete => {}
            State::Running => panic!("Once::call_once runs once per Once, outside its closure"),
            State::Incomplete => {
                self.state.set(State::Running);
                f(&OnceState { _private: () });
                self.state.set(State::Complete);
            }
        }
    }

    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.state.get() == State::Complete
    }
}

impl Default for Once {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Once {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Once").finish_non_exhaustive()
    }
}
