//! Shared, dependency-free support for dev-gate owners.

use std::path::PathBuf;

pub(super) fn cargo_manifest_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR is not set"))
}

pub(super) fn has_explanation(explanation: Option<&str>) -> bool {
    explanation.is_some_and(|explanation| !explanation.trim().is_empty())
}
