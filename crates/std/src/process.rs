//! The guest process: its exit and its identity.

use crate::{
    consts::PROCESS_ID,
    host::host,
    io::{Write, stdout},
};

/// Flushes standard output and ends the guest with exit status `code`.
pub fn exit(code: i32) -> ! {
    let _ = stdout().flush();
    host().exit(code)
}

/// Ends the guest at once with a trap.
pub fn abort() -> ! {
    core::process::abort_immediate()
}

/// The process id the guest reports for itself.
#[must_use]
pub const fn id() -> u32 {
    PROCESS_ID
}

/// An exit status a program reports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExitCode(u8);

impl ExitCode {
    pub const SUCCESS: Self = Self(0);
    pub const FAILURE: Self = Self(1);

    /// Ends the guest with this status.
    pub fn exit_process(self) -> ! {
        exit(i32::from(self.0))
    }

    /// The status as the number the host receives.
    #[must_use]
    pub const fn to_i32(self) -> i32 {
        self.0 as i32
    }
}

impl From<u8> for ExitCode {
    fn from(code: u8) -> Self {
        Self(code)
    }
}

impl Default for ExitCode {
    fn default() -> Self {
        Self::SUCCESS
    }
}
