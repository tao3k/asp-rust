use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use super::support::{findings_for_rule, has_rule, write_manifest};

#[path = "project/build_gate.rs"]
mod build_gate;
#[path = "project/manifest.rs"]
mod manifest;
#[path = "project/quality.rs"]
mod quality;
#[path = "project/retired_gate.rs"]
mod retired_gate;
#[path = "project/verification_integration/mod.rs"]
mod verification_integration;

#[test]
fn source_test_policy_does_not_treat_latest_feature_as_cfg_test() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cfg-feature-latest-source");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(feature = \"latest\")]\nmod optional;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/optional.rs"), "//! Optional owner.\n").expect("write optional");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-003"),
        "{:?}",
        report.findings
    );
    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-004"),
        "{:?}",
        report.findings
    );
}

#[test]
fn source_test_policy_accepts_nested_external_path_mounted_unit_test() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "external-path-mounted-source-test");
    fs::create_dir_all(root.join("tests/unit/db/engine")).expect("create tests/unit/db/engine");
    fs::create_dir_all(root.join("src/engine")).expect("create src/engine");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod engine;\n").expect("write lib");
    fs::write(
        root.join("src/engine/mod.rs"),
        "#[cfg(test)]\n#[path = \"../../tests/unit/db/engine/private.rs\"]\nmod private_tests;\n",
    )
    .expect("write nested source owner");
    fs::write(
        root.join("tests/unit/db/engine/private.rs"),
        "#[test]\nfn private_contract() {}\n",
    )
    .expect("write external unit test");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-003"),
        "{:?}",
        report.findings
    );
}

#[test]
fn root_test_target_accepts_embedded_cargo_test_gate_macro() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "embedded-gate-target");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "asp_rust::asp_rust_cargo_test_gate!();\n",
    )
    .expect("write root test target");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-006"),
        "{:?}",
        report.findings
    );
}

#[test]
fn root_test_target_accepts_library_cargo_test_gate_macro() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "embedded-lib-gate-targets");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n#[cfg(test)]\nasp_rust::asp_rust_cargo_test_gate!();\n",
    )
    .expect("write lib");
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
}

#[test]
fn root_test_target_comment_mentions_do_not_count_as_structure() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "comment-mention-root-gate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "//! Mentioning asp_rust_gate!() here is not a gate.\nconst NOTE: &str = \"run_asp_rust_for_scope(., asp_rust::AspRustRunScope::Package)\";\n",
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
        has_rule(&report, "RUST-AGENT-PROJECT-007"),
        "{:?}",
        report.findings
    );
}

#[test]
fn harness_dev_dependency_requires_cargo_check_build_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "missing-embedded-lib-gate");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"missing-embedded-lib-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nasp-rust = { path = \".\" }\n",
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
fn library_target_ignores_comment_mentions_of_embedded_cargo_test_gate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"comment-mention-lib-gate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nasp-rust = { path = \".\" }\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Mentioning asp_rust_cargo_test_gate!() here is not a gate.\nconst NOTE: &str = \"asp_rust_cargo_test_gate!()\";\n",
    )
    .expect("write lib");

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
fn manifest_comment_does_not_enable_library_harness_policy() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"manifest-comment-only\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n# asp-rust is mentioned in prose, not dependencies.\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

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
fn manifest_package_field_uses_the_canonical_harness_identity() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"manifest-package-field\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies.local-harness]\npackage = \"asp-rust\"\npath = \".\"\n",
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
fn target_dependency_table_uses_canonical_harness_identity() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"target-dependency-table\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[target.'cfg(unix)'.dev-dependencies]\nasp-rust = { path = \".\" }\n",
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
fn large_unit_test_leaf_is_reported_from_parser_source_metrics() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "large-unit-test-leaf");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir_all(root.join("tests/unit")).expect("create tests/unit");
    fs::write(root.join("tests/unit/large.rs"), large_unit_test_leaf()).expect("write large leaf");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-PROJECT-005");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("large.rs"));
}

#[test]
fn large_test_support_module_is_reported_from_parser_source_metrics() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "large-test-support-module");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir_all(root.join("tests/unit/scenario_performance_gate"))
        .expect("create test support dir");
    fs::write(
        root.join("tests/unit/scenario_performance_gate/support.rs"),
        large_test_support_module(),
    )
    .expect("write large test support module");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-PROJECT-024");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("support.rs"));
}

fn large_unit_test_leaf() -> String {
    let mut source = String::new();
    for index in 0..8 {
        if index % 2 == 0 {
            source.push_str("#[test]\n");
        } else {
            source.push_str("#[tokio::test]\n");
        }
        source.push_str(&format!("fn large_test_{index}() {{\n"));
        for value in 0..70 {
            source.push_str(&format!("    let value_{value} = {value};\n"));
            source.push_str(&format!("    assert_eq!(value_{value}, {value});\n"));
        }
        source.push_str("}\n");
    }
    source
}

fn large_test_support_module() -> String {
    let mut source = String::from("//! Large test support fixture.\n");
    for index in 0..1100 {
        source.push_str(&format!("pub fn helper_{index}() -> usize {{ {index} }}\n"));
    }
    source
}

#[test]
fn all_standard_rust_files_enter_agent_policy_analysis() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "all-rust-files-policy");

    for relative_path in [
        "src/orphan.rs",
        "build.rs",
        "examples/demo.rs",
        "benches/throughput.rs",
        "tests/unit_test.rs",
    ] {
        if let Some(parent) = root.join(relative_path).parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(root.join(relative_path), process_command_probe_source())
            .expect("write process command probe");
    }

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");
    let findings = findings_for_rule(&report, "RUST-AGENT-PROC-001");

    for relative_path in [
        "src/orphan.rs",
        "build.rs",
        "examples/demo.rs",
        "benches/throughput.rs",
        "tests/unit_test.rs",
    ] {
        assert!(
            findings.iter().any(|finding| finding
                .location
                .path
                .as_ref()
                .is_some_and(|path| path.ends_with(relative_path))),
            "{relative_path} was not analyzed by agent policy: {:?}",
            findings
        );
    }
}

fn process_command_probe_source() -> &'static str {
    r#"use std::process::Command;

pub fn process_command_probe() {
    let _ = Command::new("sed")
        .args([
            "-n",
            "1,220p",
            "/Users/guangtao/.agents/skills/brainstorming/SKILL.md",
        ])
        .status();
}
"#
}
