use std::fs;
use std::path::Path;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn unused_test_support_reexports_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "unused-test-support-reexports");
    write_test_support_fixture(root, true);

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-TEST-SUPPORT-014");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`UnusedType`"));
    assert!(!findings[0].summary.contains("`LocalType`"));
    assert!(!findings[0].summary.contains("`SupportType`"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn consumed_test_support_reexports_are_not_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "consumed-test-support-reexports");
    write_test_support_fixture(root, false);

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-TEST-SUPPORT-014").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn consumed_name_in_one_support_scope_does_not_clear_sibling_support_scope() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "support-scope-specific-consumption");
    write_sibling_support_fixture(root);

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-TEST-SUPPORT-014");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`SharedType`"));
    assert!(
        findings[0]
            .location
            .path
            .as_ref()
            .is_some_and(|path| path.ends_with("tests/unit/beta/support.rs"))
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

fn write_test_support_fixture(root: &Path, include_unused: bool) {
    fs::create_dir(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests/unit/search/service")).expect("create tests");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\npub mod domain;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/domain.rs"), domain_source(include_unused)).expect("write domain");
    fs::write(
        root.join("tests/unit/search/service/support.rs"),
        support_source(include_unused),
    )
    .expect("write support");
    fs::write(
        root.join("tests/unit/search/service/consumer.rs"),
        "fn smoke(value: super::support::SupportType) { let _ = value; }\n",
    )
    .expect("write consumer");
}

fn write_sibling_support_fixture(root: &Path) {
    fs::create_dir(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests/unit/alpha")).expect("create alpha tests");
    fs::create_dir_all(root.join("tests/unit/beta")).expect("create beta tests");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\npub mod domain;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain test types.\npub struct SharedType;\n",
    )
    .expect("write domain");
    fs::write(
        root.join("tests/unit/alpha/support.rs"),
        "pub(super) use crate::domain::SharedType;\n",
    )
    .expect("write alpha support");
    fs::write(
        root.join("tests/unit/alpha/consumer.rs"),
        "use super::support::SharedType;\nfn smoke(value: SharedType) { let _ = value; }\n",
    )
    .expect("write alpha consumer");
    fs::write(
        root.join("tests/unit/beta/support.rs"),
        "pub(super) use crate::domain::SharedType;\n",
    )
    .expect("write beta support");
}

fn domain_source(include_unused: bool) -> &'static str {
    if include_unused {
        "//! Domain test types.\n\
         pub struct LocalType;\n\
         pub struct SupportType;\n\
         pub struct UnusedType;\n"
    } else {
        "//! Domain test types.\n\
         pub struct LocalType;\n\
         pub struct SupportType;\n"
    }
}

fn support_source(include_unused: bool) -> &'static str {
    if include_unused {
        "pub(super) use crate::domain::{LocalType, SupportType, UnusedType};\n\
         pub(super) fn helper(value: LocalType) -> LocalType { value }\n"
    } else {
        "pub(super) use crate::domain::{LocalType, SupportType};\n\
         pub(super) fn helper(value: LocalType) -> LocalType { value }\n"
    }
}
