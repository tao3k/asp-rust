use std::fs;

use asp_rust::{
    RustDiagnosticSeverity, RustRulePack, default_asp_rust_config, render_asp_rust,
    render_asp_rust_agent_snapshot_with_config, run_asp_rust_for_scope,
    run_asp_rust_with_config_for_scope,
};
use tempfile::TempDir;

#[test]
fn policy_config_can_disable_rule_findings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_glob_import_project(root);

    let default_report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run default harness");
    assert!(
        default_report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "RUST-MOD-R010")
    );
    assert!(!default_report.is_clean());

    let config = default_asp_rust_config().with_disabled_rule("RUST-MOD-R010");
    let report =
        run_asp_rust_with_config_for_scope(root, &config, asp_rust::AspRustRunScope::Package)
            .expect("run configured harness");

    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.rule_id != "RUST-MOD-R010")
    );
    assert!(report.is_clean(), "{}", render_asp_rust(&report));
}

#[test]
fn policy_config_can_override_rule_severity() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_glob_import_project(root);
    let config =
        default_asp_rust_config().with_rule_severity("RUST-MOD-R010", RustDiagnosticSeverity::Info);

    let report =
        run_asp_rust_with_config_for_scope(root, &config, asp_rust::AspRustRunScope::Package)
            .expect("run configured harness");
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.rule_id == "RUST-MOD-R010")
        .expect("glob import finding");

    assert_eq!(finding.severity, RustDiagnosticSeverity::Info);
    assert!(report.is_clean(), "{}", render_asp_rust(&report));
}

#[test]
fn policy_config_can_disable_a_rule_pack() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"disable-agent-pack\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owner;\n").expect("write lib");
    fs::write(
        root.join("src/owner.rs"),
        "//! Owner module.\npub struct MissingDoc;\n",
    )
    .expect("write owner");
    let config = default_asp_rust_config().with_disabled_rule_pack(RustRulePack::AgentPolicy);

    let report =
        run_asp_rust_with_config_for_scope(root, &config, asp_rust::AspRustRunScope::Package)
            .expect("run configured harness");

    assert!(
        report
            .findings
            .iter()
            .all(|finding| !finding.rule_id.starts_with("AGENT-")),
        "{:?}",
        report.findings
    );
    assert!(report.is_clean(), "{}", render_asp_rust(&report));
}

#[test]
fn policy_config_can_override_a_rule_pack_severity() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_glob_import_project(root);
    let config = default_asp_rust_config()
        .with_rule_pack_severity(RustRulePack::Modularity, RustDiagnosticSeverity::Info);

    let report =
        run_asp_rust_with_config_for_scope(root, &config, asp_rust::AspRustRunScope::Package)
            .expect("run configured harness");
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.rule_id == "RUST-MOD-R010")
        .expect("glob import finding");

    assert_eq!(finding.severity, RustDiagnosticSeverity::Info);
    assert!(report.is_clean(), "{}", render_asp_rust(&report));
}

#[test]
fn policy_config_single_rule_override_wins_after_rule_pack_expansion() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_glob_import_project(root);
    let config = default_asp_rust_config()
        .with_rule_pack_severity(RustRulePack::Modularity, RustDiagnosticSeverity::Info)
        .with_rule_severity("RUST-MOD-R010", RustDiagnosticSeverity::Warning);

    let report =
        run_asp_rust_with_config_for_scope(root, &config, asp_rust::AspRustRunScope::Package)
            .expect("run configured harness");
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.rule_id == "RUST-MOD-R010")
        .expect("glob import finding");

    assert_eq!(finding.severity, RustDiagnosticSeverity::Warning);
    assert!(!report.is_clean(), "{}", render_asp_rust(&report));
}

#[test]
fn agent_snapshot_uses_policy_configured_findings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_glob_import_project(root);
    let config = default_asp_rust_config().with_disabled_rule("RUST-MOD-R010");

    let rendered = render_asp_rust_agent_snapshot_with_config(root, &config)
        .expect("render configured agent snapshot");

    assert!(!rendered.contains("RUST-MOD-R010"), "{rendered}");
    assert!(!rendered.contains("FindingGroups:"), "{rendered}");
    assert!(!rendered.contains(" - none"), "{rendered}");
    insta::assert_snapshot!("public_api_configured_agent_snapshot", rendered);
}

fn write_glob_import_project(root: &std::path::Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"policy-config\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owner;\n").expect("write lib");
    fs::write(
        root.join("src/owner.rs"),
        "//! Owner module.\nuse super::*;\n",
    )
    .expect("write owner");
}
