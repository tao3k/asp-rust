use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn public_application_error_boundary_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "application-error-boundary");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Loads a user.\n\
         pub fn load_user() -> anyhow::Result<String> { todo!() }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-API-ERROR-013");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`load_user`"));
    assert!(findings[0].summary.contains("`anyhow::Result`"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn typed_and_test_error_boundaries_are_not_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "typed-error-boundary");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// API error.\n\
         pub enum ApiError {}\n\
         /// Loads a user.\n\
         pub fn load_user() -> Result<String, ApiError> { todo!() }\n\
         #[cfg(test)]\n\
         pub fn fixture_user() -> eyre::Result<String> { todo!() }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-ERROR-013").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
