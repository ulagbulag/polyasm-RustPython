//! The std prelude: the `core` prelude, the `alloc` items, the std macros and [`FloatMath`].
//!
//! [`FloatMath`]: crate::os::polyasm::num::FloatMath

/// Items of every edition prelude beyond `core`.
mod common {
    pub use crate::os::polyasm::num::FloatMath;
    pub use crate::{dbg, eprint, eprintln, format, print, println, thread_local};
    pub use alloc_crate::{
        borrow::ToOwned,
        boxed::Box,
        string::{String, ToString},
        vec::Vec,
    };

    mod macros_only {
        #[expect(hidden_glob_reexports)]
        mod vec {}
        pub use alloc_crate::*;
    }
    pub use macros_only::vec;
}

/// The 2021 edition prelude.
pub mod rust_2021 {
    pub use super::common::*;
    pub use core::prelude::rust_2021::*;
}

/// The 2024 edition prelude.
pub mod rust_2024 {
    pub use super::common::*;
    pub use core::prelude::rust_2024::*;
}
