use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn public_tuple_api_surface_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-tuple-api-surface");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Loads a page of users.\n\
         pub fn load_users(cursor: (String, usize, bool)) -> Result<(String, usize), LoadError> { todo!() }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-API-SHAPE-023");
    assert_eq!(findings.len(), 2, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`load_users`"));
    assert!(findings[0].summary.contains("parameter `cursor`"));
    assert!(findings[0].summary.contains("String, usize, bool"));
    assert!(findings[1].summary.contains("return value"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_tuple_api_surface_clears_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-tuple-api-surface");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Tuple API boundary: this generated bridge mirrors an external cursor pair.\n\
         pub fn load_users(cursor: (String, usize, bool)) -> Result<(String, usize), LoadError> { todo!() }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-SHAPE-023").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn named_api_surface_clears_tuple_api_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "named-api-surface");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User cursor.\n\
         pub struct UserCursor;\n\
         /// User page.\n\
         pub struct UserPage;\n\
         /// Loads a page of users.\n\
         pub fn load_users(cursor: UserCursor) -> Result<UserPage, LoadError> { todo!() }\n\
         #[cfg(test)]\n\
         pub fn fixture_users(cursor: (String, usize)) -> (String, usize) { todo!() }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-SHAPE-023").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
