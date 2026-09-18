use std::fs;
use std::path::Path;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::has_rule;

#[test]
fn dev_dependency_with_source_test_gate_completes_policy_activation() {
    let temp = fixture(
        "source-test-gate",
        "[dev-dependencies]\nasp-rust = { path = \".\" }\n",
        "//! Test crate.\n#[cfg(test)]\nasp_rust::asp_rust_cargo_test_gate!(config = { asp_rust::default_asp_rust_config() });\n",
    );
    let report = run(temp.path());

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
fn dev_dependency_with_test_target_gate_completes_policy_activation() {
    let temp = fixture(
        "target-test-gate",
        "[dev-dependencies]\nasp-rust = { path = \".\" }\n",
        "//! Test crate.\n",
    );
    fs::create_dir(temp.path().join("tests")).expect("create tests");
    fs::write(
        temp.path().join("tests/policy.rs"),
        "asp_rust::asp_rust_gate!();\n",
    )
    .expect("write policy target");
    let report = run(temp.path());

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-006"),
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
fn cargo_test_gate_without_dev_dependency_is_incomplete() {
    let temp = fixture(
        "missing-dev-dependency",
        "",
        "//! Test crate.\n#[cfg(test)]\nasp_rust::asp_rust_cargo_test_gate!(config = { asp_rust::default_asp_rust_config() });\n",
    );
    let report = run(temp.path());

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn normal_dependency_is_rejected_even_with_a_test_gate() {
    let temp = fixture(
        "normal-dependency",
        "[dependencies]\nasp-rust = { path = \".\" }\n",
        "//! Test crate.\n#[cfg(test)]\nasp_rust::asp_rust_cargo_test_gate!(config = { asp_rust::default_asp_rust_config() });\n",
    );
    let report = run(temp.path());

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-006"),
        "{:?}",
        report.findings
    );
    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn build_dependency_and_build_script_gate_are_rejected() {
    let temp = fixture(
        "build-dependency",
        "[build-dependencies]\nasp-rust = { path = \".\" }\n",
        "//! Test crate.\n",
    );
    fs::write(
        temp.path().join("build.rs"),
        "fn main() { asp_rust::assert_asp_rust_cargo_check_clean_from_env(); }\n",
    )
    .expect("write build script");
    let report = run(temp.path());

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-006"),
        "{:?}",
        report.findings
    );
    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-009"),
        "{:?}",
        report.findings
    );
    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn dev_dependency_without_activation_requires_dev_gate() {
    let temp = fixture(
        "test-tooling-only",
        "[dev-dependencies]\nasp-rust = { path = \".\" }\n",
        "//! Test crate.\n",
    );
    let report = run(temp.path());

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn build_support_workspace_test_gate_completes_policy_activation() {
    let temp = fixture(
        "workspace-member",
        "[dev-dependencies]\nworkspace-build-support = { path = \"build-support\" }\n",
        "//! Test crate.\n",
    );
    fs::create_dir(temp.path().join("tests")).expect("create tests");
    fs::write(
        temp.path().join("tests/asp_rust_policy.rs"),
        "workspace_build_support::asp_workspace_policy_gate!();\n",
    )
    .expect("write policy target");
    let report = run(temp.path());

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

fn fixture(name: &str, dependency_tables: &str, lib: &str) -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n{dependency_tables}"
        ),
    )
    .expect("write manifest");
    fs::create_dir(temp.path().join("src")).expect("create src");
    fs::write(temp.path().join("src/lib.rs"), lib).expect("write lib");
    temp
}

fn run(root: &Path) -> asp_rust::AspRustReport {
    run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package).expect("run project harness")
}
