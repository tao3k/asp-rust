use std::fs;
use std::path::Path;

use asp_rust::{render_asp_rust, run_asp_rust_for_scope};
use tempfile::TempDir;

use crate::path_policy::support::{has_rule, write_manifest};

#[test]
fn complete_build_gate_clears_source_cargo_test_gate_requirement() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_build_harness_manifest(root, "build-gate-only");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    write_configured_build_gate(root);

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

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
fn downstream_policy_build_gate_clears_build_gate_requirement() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_build_harness_manifest(root, "downstream-policy-build-gate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    write_downstream_policy_build_gate(root);

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn workspace_wrapper_build_gate_clears_direct_harness_dependency_requirement() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_build_harness_manifest(root, "workspace-wrapper-build-gate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    write_workspace_wrapper_build_gate(root);

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn ordinary_workspace_alias_does_not_clear_build_gate_requirement() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_build_harness_manifest(root, "ordinary-workspace-alias-build-gate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    write_ordinary_workspace_alias_build_gate(root);

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn complete_build_gate_clears_root_test_target_gate_requirement() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_build_harness_manifest(root, "build-gate-root-test");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    write_configured_build_gate(root);
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "//! Thin root target.\n#[path = \"unit/suite.rs\"]\nmod suite;\n",
    )
    .expect("write root test target");
    fs::create_dir_all(root.join("tests/unit")).expect("create test suite dir");
    fs::write(
        root.join("tests/unit/suite.rs"),
        "#[test]\nfn suite_runs() {}\n",
    )
    .expect("write suite");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

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
fn harness_enabled_build_script_requires_build_gate_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"missing-build-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nasp-rust = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nasp_rust::asp_rust_cargo_test_gate!(config = { asp_rust::default_asp_rust_config() });\n",
    )
    .expect("write lib");
    fs::write(root.join("build.rs"), "fn main() {}\n").expect("write build script");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");
    let rendered = normalize_temp_root(&render_asp_rust(&report), root);

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
    insta::assert_snapshot!("harness_enabled_build_script_requires_build_gate", rendered);
}

#[test]
fn harness_dependency_requires_cargo_check_build_gate_without_build_script() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"missing-cargo-check-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nasp-rust = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-009"),
        "{:?}",
        report.findings
    );
}

#[test]
fn harness_build_dependency_requires_root_build_script() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_build_harness_manifest(root, "missing-build-rs");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn build_gate_default_config_requires_explicit_verification_config() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_build_harness_manifest(root, "default-build-gate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(
        root.join("build.rs"),
        "fn main() {\n    asp_rust::assert_asp_rust_build_clean_from_env();\n}\n",
    )
    .expect("write build script");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-011"),
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
fn build_gate_call_requires_build_dependency() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "missing-build-dependency");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    write_configured_build_gate(root);

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

#[test]
fn non_harness_build_script_does_not_require_build_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "plain-build-script");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(root.join("build.rs"), "fn main() {}\n").expect("write build script");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-012"),
        "{:?}",
        report.findings
    );
}

fn write_build_harness_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[build-dependencies]\nasp-rust = {{ path = \".\" }}\n",
        ),
    )
    .expect("write manifest");
}

fn write_configured_build_gate(root: &Path) {
    fs::write(
        root.join("build.rs"),
        "fn main() {\n    let config = asp_rust::default_asp_rust_config();\n    asp_rust::assert_asp_rust_cargo_check_clean_from_env_with_config(&config);\n}\n",
    )
    .expect("write build script");
}

fn write_workspace_wrapper_build_gate(root: &Path) {
    fs::write(
        root.join("build.rs"),
        "fn main() {\n    xiuxian_rust_workspace_harness::assert_member_harness_build_gate_from_env();\n}\n",
    )
    .expect("write build script");
}

fn write_ordinary_workspace_alias_build_gate(root: &Path) {
    fs::write(
        root.join("build.rs"),
        "fn main() {\n    xiuxian_rust_workspace_harness::assert_member_build_gate_from_env();\n}\n",
    )
    .expect("write build script");
}

fn write_downstream_policy_build_gate(root: &Path) {
    fs::write(
        root.join("build.rs"),
        "fn main() {\n    let config = asp_rust::default_asp_rust_config();\n    let policy = asp_rust::AspRustWorkspacePolicy::new(\"test-workspace\", config).member_crate(\"downstream-policy-build-gate\");\n    asp_rust::assert_asp_rust_downstream_policy_from_env(&policy);\n}\n",
    )
    .expect("write build script");
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
