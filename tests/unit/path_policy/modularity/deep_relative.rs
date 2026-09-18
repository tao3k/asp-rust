use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, has_rule, write_manifest};

#[test]
fn deep_relative_import_policy_uses_native_use_trees() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "deep-relative-use-tree");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\npub use super::{super::MissingOwner, sibling::Thing};\n",
    )
    .expect("write domain");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R003");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("deep relative import `super::super::MissingOwner`"),
        "{}",
        findings[0].summary
    );
    let source_line = findings[0].source_line.as_deref().expect("source line");
    assert!(source_line.contains("super::{super::MissingOwner, sibling::Thing}"));
}

#[test]
fn deep_relative_import_policy_reports_pub_super_prefix_group_without_crate_suggestion() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "deep-relative-pub-super-prefix-group");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\npub(super) use super::super::{assert_workspace_patch_intent_artifact, local_gxi};\n",
    )
    .expect("write lib");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R003");
    assert_eq!(findings.len(), 2, "{:?}", report.findings);
    assert!(
        findings.iter().any(|finding| finding.summary.contains(
            "deep relative import `super::super::assert_workspace_patch_intent_artifact`"
        )),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|finding| finding
            .summary
            .contains("deep relative import `super::super::local_gxi`")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .all(|finding| finding.summary.contains("parser could not derive")),
        "{findings:?}"
    );
}

#[test]
fn deep_relative_import_policy_ignores_comments_and_strings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "deep-relative-comment");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\n// use super::super::MissingOwner;\nconst HELP: &str = \"use super::super::MissingOwner\";\n",
    )
    .expect("write domain");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R003"), "{:?}", report.findings);
}
