use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, has_rule, write_manifest};

#[test]
fn glob_import_policy_flags_native_use_tree_globs() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "glob-use-tree");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\nuse crate::prelude::*;\nuse super::{sibling::Thing, traits::*};\n",
    )
    .expect("write domain");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R010");
    assert_eq!(findings.len(), 2, "{:?}", report.findings);
    assert!(
        findings
            .iter()
            .all(|finding| finding.summary.contains("glob import"))
    );
}

#[test]
fn glob_import_policy_reports_cfg_test_context_from_parser_stack() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cfg-test-glob-use-tree");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\nfn helper() {}\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn smoke() { helper(); }\n}\n",
    )
    .expect("write domain");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R010");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("parent-scope glob import `super::*` in test context")
    );
    assert_eq!(
        findings[0].label,
        "replace parent-scope glob with explicit imports"
    );
}

#[test]
fn glob_import_policy_flags_absolute_crate_owner_glob() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "absolute-crate-owner-glob");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\nuse crate::gateway::studio::studio_repo_sync_api_tests::*;\n",
    )
    .expect("write domain");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R010");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0].summary.contains(
            "crate-owner glob import `crate::gateway::studio::studio_repo_sync_api_tests::*`"
        ),
        "{}",
        findings[0].summary
    );
    assert_eq!(
        findings[0].label,
        "replace glob import with explicit owner imports"
    );
}

#[test]
fn glob_import_policy_scans_tests_root_with_test_context() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "tests-root-glob-use-tree");
    fs::create_dir(root.join("src")).expect("create src");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(
        root.join("tests/integration.rs"),
        "use crate::prelude::*;\n\n#[test]\nfn smoke() {}\n",
    )
    .expect("write integration test");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R010");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("prelude glob import `crate::prelude::*` in test context")
    );
}

#[test]
fn glob_import_policy_ignores_comments_and_strings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "glob-comment");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\n// use crate::prelude::*;\nconst HELP: &str = \"use crate::prelude::*\";\n",
    )
    .expect("write domain");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R010"), "{:?}", report.findings);
}
