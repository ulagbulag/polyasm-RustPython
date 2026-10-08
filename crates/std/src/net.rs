//! Network address types of `core::net`, with [`Shutdown`].

pub use core::net::*;

/// The halves of a socket connection [`crate::host::Host::shutdown`] closes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Shutdown {
    /// The reading half.
    Read,
    /// The writing half.
    Write,
    /// Both halves.
    Both,
}
