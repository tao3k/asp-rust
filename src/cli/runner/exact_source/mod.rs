//! Exact-source query execution over a pinned source snapshot.

mod core;
mod encoding;
mod model;
pub(crate) use core::rust_structural_selector;
pub(crate) use encoding::decode_canonical_base64;

pub(super) use core::run_exact_source_query;
