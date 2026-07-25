//! Exact-source query execution over a pinned source snapshot.

mod core;
mod model;
mod parse_artifact;

pub(super) use core::run_exact_source_query;
