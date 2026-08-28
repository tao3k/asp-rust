use std::fs;
use std::path::Path;

use rust_lang_project_harness::{
    RustHarnessConfig, render_rust_project_harness, run_rust_project_harness_for_scope,
    run_rust_project_harness_with_config_for_scope,
};
use tempfile::TempDir;

#[test]
fn project_local_policy_cannot_allow_root_file_exceptions() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root);
    fs::write(
        root.join("tests/custom_gate.rs"),
        "rust_lang_project_harness::rust_project_harness_gate!();\n",
    )
    .expect("write custom gate");

    write_policy(
        root,
        "[tests]\nallowed_root_files = [\n  { name = \"custom_gate.rs\", explanation = \"explicit harness aggregate\" },\n]\n",
    );
    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-001"));
}

#[test]
fn project_local_policy_cannot_allow_directory_exceptions() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root);
    fs::create_dir(root.join("tests/contract")).expect("create contract tests");
    fs::write(root.join("tests/contract/fixtures.rs"), "fn helper() {}\n")
        .expect("write contract fixture");

    write_policy(
        root,
        "[tests]\nallowed_directories = [\n  { name = \"contract\", explanation = \"contract fixtures mounted by a root gate\" },\n]\n",
    );
    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-002"));
}

#[test]
fn harness_scope_policy_requires_explanations_for_custom_source_paths() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root);
    fs::create_dir_all(root.join("src/integration_support")).expect("create custom source dir");
    fs::write(
        root.join("src/integration_support/search_strategy_flow.rs"),
        "//! Focused support owner.\n",
    )
    .expect("write custom source");

    let config = RustHarnessConfig {
        source_dir_names: vec![
            "src/lib.rs".to_owned(),
            "src/integration_support/search_strategy_flow.rs".to_owned(),
        ],
        ..RustHarnessConfig::default()
    };
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert_eq!(rule_count(&report, "RUST-AGENT-PROJECT-013"), 2);
    let mut focused_report = report.clone();
    focused_report
        .findings
        .retain(|finding| finding.rule_id == "RUST-AGENT-PROJECT-013");
    let rendered = normalize_temp_root(&render_rust_project_harness(&focused_report), root);
    insta::assert_snapshot!("custom_scope_paths_require_explanations", rendered);

    let config = RustHarnessConfig::default()
        .with_source_path(
            "src/lib.rs",
            "cargo-check build gate keeps the crate facade inside harness policy",
        )
        .with_source_path(
            "src/integration_support/search_strategy_flow.rs",
            "temporary focused migration owner while the integration support branch is split",
        );
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(!has_rule(&report, "RUST-AGENT-PROJECT-013"));
}

#[test]
fn harness_scope_policy_requires_explanations_for_custom_test_paths() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root);
    fs::create_dir_all(root.join("tests/contracts")).expect("create custom test dir");
    fs::write(
        root.join("tests/contracts/api.rs"),
        "fn contract_fixture() {}\n",
    )
    .expect("write contract test");

    let config = RustHarnessConfig {
        test_dir_names: vec!["tests/contracts".to_owned()],
        ..RustHarnessConfig::default()
    };
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-013"));

    let config = RustHarnessConfig::default().with_test_path(
        "tests/contracts",
        "contract fixtures are mounted through explicit root test targets",
    );
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(!has_rule(&report, "RUST-AGENT-PROJECT-013"));
}

#[test]
fn harness_scope_policy_requires_explanations_for_default_source_reduction() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root);
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod live;\n").expect("write lib");
    fs::write(root.join("src/live.rs"), "//! Live owner.\n").expect("write live owner");

    let config = RustHarnessConfig {
        source_dir_names: vec!["src/lib.rs".to_owned()],
        ..RustHarnessConfig::default()
    }
    .with_source_path(
        "src/lib.rs",
        "cargo-check build gate keeps the crate facade inside harness policy",
    );
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    let mut focused_report = report.clone();
    focused_report
        .findings
        .retain(|finding| finding.rule_id == "RUST-AGENT-PROJECT-014");
    let rendered = normalize_temp_root(&render_rust_project_harness(&focused_report), root);
    insta::assert_snapshot!("default_scope_reduction_requires_explanations", rendered);
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-014"));

    let config = RustHarnessConfig {
        source_dir_names: vec!["src/lib.rs".to_owned()],
        ..RustHarnessConfig::default()
    }
    .with_source_path(
        "src/lib.rs",
        "cargo-check build gate keeps the crate facade inside harness policy",
    )
    .with_source_path_excluded(
        "src",
        "temporary migration keeps only the crate facade until live.rs is split",
    );
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(!has_rule(&report, "RUST-AGENT-PROJECT-014"));
}

#[test]
fn harness_scope_policy_requires_explanations_for_test_scope_reduction() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_minimal_project(root);
    fs::write(root.join("tests/unit_test.rs"), "fn root_test() {}\n").expect("write root test");

    let config = RustHarnessConfig {
        include_tests: false,
        ..RustHarnessConfig::default()
    };
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    let mut focused_report = report.clone();
    focused_report
        .findings
        .retain(|finding| finding.rule_id == "RUST-AGENT-PROJECT-014");
    let rendered = normalize_temp_root(&render_rust_project_harness(&focused_report), root);
    insta::assert_snapshot!("test_scope_reduction_requires_explanations", rendered);
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-014"));

    let config = RustHarnessConfig::default()
        .with_tests_excluded("fixture intentionally checks project policy without parsing tests");
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(!has_rule(&report, "RUST-AGENT-PROJECT-014"));

    let config = RustHarnessConfig {
        test_dir_names: Vec::new(),
        ..RustHarnessConfig::default()
    };
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-014"));

    let config = RustHarnessConfig::default().with_test_path_excluded(
        "tests",
        "root tests are mounted through a separate CI shard",
    );
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(!has_rule(&report, "RUST-AGENT-PROJECT-014"));
}

#[test]
fn harness_scope_policy_requires_explanations_for_manifest_test_targets() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest_with_test_target(root);
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("contracts")).expect("create contracts");
    fs::write(root.join("contracts/api.rs"), "fn contract_test() {}\n")
        .expect("write manifest test target");

    let config = RustHarnessConfig {
        include_tests: false,
        ..RustHarnessConfig::default()
    };
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-014"));
    assert!(
        report.findings.iter().any(|finding| {
            finding.rule_id == "RUST-AGENT-PROJECT-014"
                && finding.summary.contains("contracts/api.rs")
        }),
        "{:?}",
        report.findings
    );

    let config = RustHarnessConfig::default()
        .with_tests_excluded("manifest test target is executed by a separate contract shard");
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    assert!(!has_rule(&report, "RUST-AGENT-PROJECT-014"));
}

fn write_minimal_project(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"policy-config\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
}

fn write_manifest_with_test_target(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"policy-config\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[test]]\nname = \"contract\"\npath = \"contracts/api.rs\"\n",
    )
    .expect("write manifest");
}

fn write_policy(root: &Path, content: &str) {
    fs::write(root.join("tests/attempted-local-policy.toml"), content)
        .expect("write ignored project-local policy attempt");
}

fn has_rule(report: &rust_lang_project_harness::RustHarnessReport, rule_id: &str) -> bool {
    report
        .findings
        .iter()
        .any(|finding| finding.rule_id == rule_id)
}

fn rule_count(report: &rust_lang_project_harness::RustHarnessReport, rule_id: &str) -> usize {
    report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == rule_id)
        .count()
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
