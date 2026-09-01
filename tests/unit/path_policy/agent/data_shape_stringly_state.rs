use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn public_stringly_state_fields_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-stringly-state-fields");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Job snapshot crossing a public boundary.\n\
         pub struct JobSnapshot {\n\
         \tpub status: String,\n\
         \tpub retry_mode: Option<String>,\n\
         }\n\
         /// Event emitted by the API.\n\
         pub enum JobEvent {\n\
         \t/// Job changed state.\n\
         \tChanged { state: String },\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DATA-STATE-028");
    assert_eq!(findings.len(), 2, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`JobSnapshot`"));
    assert!(findings[0].summary.contains("status: String"));
    assert!(findings[0].summary.contains("retry_mode: Option<String>"));
    assert!(findings[1].summary.contains("`JobEvent`"));
    assert!(findings[1].summary.contains("state: String"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_stringly_state_fields_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-stringly-state-fields");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Stringly state boundary: serialized jobs keep catalog tokens from upstream.\n\
         pub struct JobSnapshot {\n\
         \tpub status: String,\n\
         \tpub retry_mode: Option<String>,\n\
         }\n\
         /// Event emitted by the API.\n\
         pub enum JobEvent {\n\
         \t/// Stringly state boundary: serialized events keep upstream state tokens.\n\
         \tChanged { state: String },\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-STATE-028").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn typed_and_static_state_fields_clear_stringly_state_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "typed-state-fields");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Job lifecycle status.\n\
         pub enum JobStatus { Queued, Running, Done }\n\
         /// Retry behavior.\n\
         pub enum RetryMode { None, Fast }\n\
         /// Job snapshot crossing a public boundary.\n\
         pub struct JobSnapshot {\n\
         \tpub status: JobStatus,\n\
         \tpub retry_mode: RetryMode,\n\
         }\n\
         /// Rule descriptor metadata.\n\
         pub struct RuleDescriptor { pub default_mode: &'static str }\n\
         #[cfg(test)]\n\
         pub struct FixtureSnapshot { pub status: String }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-STATE-028").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
