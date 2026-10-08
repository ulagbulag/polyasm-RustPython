//! The std prelude: the `core` prelude, the `alloc` items, the std macros and [`FloatMath`].
//!
//! [`FloatMath`]: crate::os::polyasm::num::FloatMath

/// Items of every edition prelude beyond `core`.
mod common {
    pub use alloc_crate::{
        borrow::ToOwned,
        boxed::Box,
        string::{String, ToString},
        vec::Vec,
    };

    pub use crate::{
        dbg, eprint, eprintln, format, os::polyasm::num::FloatMath, print, println, thread_local,
    };

    mod macros_only {
        #[expect(hidden_glob_reexports)]
        mod vec {}
        pub use alloc_crate::*;
    }
    pub use self::macros_only::vec;
}

/// The 2021 edition prelude.
pub mod rust_2021 {
    pub use core::prelude::rust_2021::*;

    pub use super::common::*;
}

/// The 2024 edition prelude.
pub mod rust_2024 {
    pub use core::prelude::rust_2024::*;

    pub use super::common::*;
}
