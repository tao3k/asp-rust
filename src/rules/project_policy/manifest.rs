//! Project-policy checks backed by Cargo manifest parser facts.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{CargoManifestFacts, file_location};
use crate::{AspRustFinding, AspRustRule};

use super::RUST_PROJ_R023;

const CURRENT_RUST_EDITION: &str = "2024";

pub(super) fn manifest_findings(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    if !cargo_manifest.has_package {
        return Vec::new();
    }
    let Some(edition) = cargo_manifest.package_edition.as_deref() else {
        return Vec::new();
    };
    if edition == CURRENT_RUST_EDITION {
        return Vec::new();
    }

    let rule = &rules[RUST_PROJ_R023];
    let manifest_path = project_root.join("Cargo.toml");
    vec![AspRustFinding::from_rule(
        rule,
        format!(
            "Cargo.toml declares Rust edition {edition}; current agent-authored packages should use edition {CURRENT_RUST_EDITION}."
        ),
        file_location(&manifest_path),
        Some(format!("edition = \"{edition}\"")),
        "set package edition to \"2024\" or document a bounded compatibility reason and migration plan",
    )]
}
