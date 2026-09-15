use std::fs;
use std::path::Path;

use asp_rust::{
    default_asp_rust_config, run_asp_rust_for_scope, run_asp_rust_with_config_for_scope,
};
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, has_rule, write_manifest};

#[test]
fn weak_advice_allow_explanation_is_flagged() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "weak-advice-allowance");
    let config =
        default_asp_rust_config().with_cargo_check_advice_allow_explanation("temporary migration");

    let report =
        run_asp_rust_with_config_for_scope(root, &config, asp_rust::AspRustRunScope::Package)
            .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-PROJECT-017");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("cargo_check_advice_allow_explanation"),
        "{:?}",
        findings[0]
    );
}

#[test]
fn structured_advice_allow_explanation_is_allowed() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "structured-advice-allowance");
    let config = default_asp_rust_config().with_cargo_check_advice_allow_explanation(
        "scope=test build.rs gates; owner=test-harness; finding_category=advisory \
         project-policy migrations; why_safe_now=warnings stay visible in harness output; \
         cleanup_trigger=remove once strict downstream gate is enabled",
    );

    let report =
        run_asp_rust_with_config_for_scope(root, &config, asp_rust::AspRustRunScope::Package)
            .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-017"),
        "{:?}",
        report.findings
    );
}

#[test]
fn fake_cargo_package_identity_fallback_is_flagged() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "fake-cargo-identity");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn cargo_package_name_from_env() -> String {\n    \
         std::env::var(\"CARGO_PKG_NAME\")\n        \
         .unwrap_or_else(|_| String::from(\"unknown-cargo-package\"))\n}\n",
    )
    .expect("write lib");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-018"),
        "{:?}",
        report.findings
    );
}

#[test]
fn redundant_workspace_member_build_gate_alias_is_flagged() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "redundant-build-gate-alias");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn assert_member_build_gate_from_env() {\n    \
         assert_member_harness_build_gate_from_env();\n}\n\n\
         pub fn assert_member_harness_build_gate_from_env() {}\n",
    )
    .expect("write lib");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-019"),
        "{:?}",
        report.findings
    );
}

#[test]
fn silent_evidence_default_is_flagged_in_search_graph_code() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "silent-evidence-default");
    write_sensitive_module(
        root,
        "pub(crate) fn semantic_path(anchor_id: &str) -> Vec<String> {\n    \
         extract_lineage(anchor_id).unwrap_or_default()\n}\n\n\
         fn extract_lineage(_: &str) -> Option<Vec<String>> { None }\n",
    );

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-020"),
        "{:?}",
        report.findings
    );
}

#[test]
fn source_location_sentinel_is_flagged_in_candidate_code() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "source-location-sentinel");
    write_sensitive_module(
        root,
        "pub(crate) fn decode_line(value: u64) -> Location {\n    \
         Location { line: usize::try_from(value).unwrap_or(usize::MAX) }\n}\n\n\
         pub(crate) struct Location { line: usize }\n",
    );

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-021"),
        "{:?}",
        report.findings
    );
}

#[test]
fn candidate_loop_without_rejection_telemetry_is_flagged() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "candidate-loop-telemetry");
    write_sensitive_module(
        root,
        "pub(crate) fn collect_candidates(scores: &[f64], telemetry: &mut Telemetry) {\n    \
         telemetry.observe_batch(scores.len());\n    \
         for score in scores {\n        \
         if *score <= 0.0 {\n            \
         continue;\n        \
         }\n        \
         telemetry.observe_match();\n    \
         }\n}\n\n\
         pub(crate) struct Telemetry;\n\
         impl Telemetry {\n    \
         fn observe_batch(&mut self, _: usize) {}\n    \
         fn observe_match(&mut self) {}\n\
         }\n",
    );

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-022"),
        "{:?}",
        report.findings
    );
}

#[test]
fn candidate_loop_with_rejection_telemetry_is_allowed() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root, "candidate-loop-with-telemetry");
    write_sensitive_module(
        root,
        "pub(crate) fn collect_candidates(scores: &[f64], telemetry: &mut Telemetry) {\n    \
         telemetry.observe_batch(scores.len());\n    \
         for score in scores {\n        \
         if *score <= 0.0 {\n            \
         telemetry.observe_filtered();\n            \
         continue;\n        \
         }\n        \
         telemetry.observe_match();\n    \
         }\n}\n\n\
         pub(crate) struct Telemetry;\n\
         impl Telemetry {\n    \
         fn observe_batch(&mut self, _: usize) {}\n    \
         fn observe_match(&mut self) {}\n    \
         fn observe_filtered(&mut self) {}\n\
         }\n",
    );

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-022"),
        "{:?}",
        report.findings
    );
}

fn write_minimal_project(root: &Path, name: &str) {
    write_manifest(root, name);
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod search;\n").expect("write lib");
}

fn write_sensitive_module(root: &Path, source: &str) {
    fs::create_dir_all(root.join("src/search")).expect("create search dir");
    fs::write(root.join("src/search/mod.rs"), "mod candidates;\n").expect("write search mod");
    fs::write(root.join("src/search/candidates.rs"), source).expect("write candidates");
}
