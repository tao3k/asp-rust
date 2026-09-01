use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{
    findings_for_rule, has_rule, private_implementation_pile, write_manifest,
};

#[test]
fn interface_mod_policy_rejects_inline_module_implementation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "inline-mod-implementation");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain interface.\nmod leaf { fn helper() {} }\n",
    )
    .expect("write mod");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R001");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .source_line
            .as_deref()
            .is_some_and(|line| line.contains("mod leaf"))
    );
}

#[test]
fn source_bloat_policy_reports_private_implementation_pile() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "private-implementation-pile");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod pile;\n").expect("write lib");
    fs::write(root.join("src/pile.rs"), private_implementation_pile()).expect("write pile");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R002");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("top-level implementation items")
    );
}

#[test]
fn source_bloat_policy_reports_test_support_pile() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "test-support-pile");
    fs::create_dir(root.join("src")).expect("create src");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(root.join("tests/support.rs"), private_implementation_pile()).expect("write support");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R002");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .location
            .path
            .as_ref()
            .is_some_and(|path| path.ends_with("tests/support.rs"))
    );
}

#[test]
fn source_bloat_policy_reports_absolute_line_pressure_without_item_pressure() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "absolute-line-pressure");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod generated;\n").expect("write lib");
    fs::write(
        root.join("src/generated.rs"),
        absolute_line_pressure_table(),
    )
    .expect("write generated");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R002");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("absolute source line pressure")
    );
}

#[test]
fn source_bloat_policy_allows_999_source_lines_below_1000_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "source-line-boundary");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod boundary;\n").expect("write lib");
    fs::write(
        root.join("src/boundary.rs"),
        source_with_exact_line_count(999),
    )
    .expect("write boundary module");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        findings_for_rule(&report, "RUST-MOD-R002").is_empty(),
        "{:?}",
        report.findings
    );
}

#[test]
fn source_bloat_policy_reports_invalid_source_absolute_line_pressure() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "invalid-absolute-line-pressure");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod broken;\n").expect("write lib");
    fs::write(
        root.join("src/broken.rs"),
        invalid_absolute_line_pressure_table(),
    )
    .expect("write broken");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R002");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("absolute source line pressure")
    );
}

fn absolute_line_pressure_table() -> String {
    source_with_exact_line_count(1000)
}

fn source_with_exact_line_count(line_count: usize) -> String {
    assert!(line_count > 0);
    let mut source = String::new();
    for value in 0..line_count - 1 {
        source.push_str(&format!("// generated fixture row {value}\n"));
    }
    source.push_str("pub const VALUES: &[usize] = &[];\n");
    source
}

fn invalid_absolute_line_pressure_table() -> String {
    let mut source = absolute_line_pressure_table();
    source.push_str("pub fn broken( {\n");
    source
}

#[test]
fn module_source_layout_policy_rejects_file_and_mod_rs_pair() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "shadowed-module-source");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain.rs"), "//! Domain owner.\n").expect("write file form");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain directory owner.\n",
    )
    .expect("write mod form");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R007");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/domain.rs"));
    assert!(findings[0].summary.contains("src/domain/mod.rs"));
}

#[test]
fn inline_source_module_policy_rejects_reasoning_tree_collapse() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "inline-source-module");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain owner.\nmod leaf { fn helper() {} }\n",
    )
    .expect("write domain");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R008");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .source_line
            .as_deref()
            .is_some_and(|line| line.contains("mod leaf"))
    );
}

#[test]
fn orphan_source_file_policy_rejects_unreachable_module_file() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "orphan-source-file");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(root.join("src/forgotten.rs"), "//! Forgotten owner.\n").expect("write orphan");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R009");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .location
            .path
            .as_ref()
            .is_some_and(|path| path.ends_with("src/forgotten.rs"))
    );
}

#[test]
fn orphan_policy_does_not_treat_latest_feature_as_cfg_test() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cfg-feature-latest-reachability");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(feature = \"latest\")]\nmod optional;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/optional.rs"), "//! Optional owner.\n").expect("write optional");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R009"), "{:?}", report.findings);
}
