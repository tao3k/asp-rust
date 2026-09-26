use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::has_rule;

#[test]
fn root_cargo_test_gate_is_the_active_policy_surface() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "root-cargo-test-gate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/policy.rs"),
        "asp_rust::asp_rust_gate!();\n",
    )
    .expect("write root test target");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-006"),
        "{:?}",
        report.findings
    );
    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-009"),
        "{:?}",
        report.findings
    );
    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn configured_source_cargo_test_gate_is_the_active_policy_surface() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "source-cargo-test-gate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nasp_rust::asp_rust_cargo_test_gate!(config = { asp_rust::default_asp_rust_config() });\n",
    )
    .expect("write lib");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-006"),
        "{:?}",
        report.findings
    );
    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-009"),
        "{:?}",
        report.findings
    );
    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

fn write_manifest(root: &std::path::Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nasp-rust = {{ path = \".\" }}\n"
        ),
    )
    .expect("write manifest");
}
