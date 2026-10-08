//! Collections of `alloc`, with [`HashMap`] and [`HashSet`] keyed by `RandomState`.

pub mod hash_map;
pub mod hash_set;

pub use alloc_crate::collections::*;
pub use hash_map::HashMap;
pub use hash_set::HashSet;
