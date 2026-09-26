use std::collections::BTreeSet;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use asp_rust::{
    AspRustWorkspacePolicy, asp_rust_workspace_build_dag_with_metrics,
    assert_asp_rust_workspace_build_dag_policy_with, default_asp_rust_config,
};
use tempfile::TempDir;

#[test]
fn cargo_dag_evaluates_each_package_atom_once() {
    let temp = TempDir::new().expect("temp dir");
    write_workspace(temp.path(), false);
    let config = default_asp_rust_config();
    let derivation = asp_rust_workspace_build_dag_with_metrics(temp.path(), &config)
        .expect("derive workspace Build DAG");
    assert_eq!(derivation.metrics.admitted_package_count, 3);
    assert_eq!(derivation.metrics.parsed_manifest_count, 4);

    let policy = AspRustWorkspacePolicy::new("fixture", config);
    let report = assert_asp_rust_workspace_build_dag_policy_with(
        derivation.build_dag,
        &policy,
        |_, config| config,
    );
    let labels = report
        .members
        .iter()
        .map(|member| member.crate_label.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(report.members.len(), 3);
    assert_eq!(labels.len(), 3, "each Cargo package must be scanned once");
    assert_eq!(report.members[0].crate_label, "fixture-core");
}

#[test]
fn workspace_gate_rejects_oversized_reachable_production_module() {
    let temp = TempDir::new().expect("temp dir");
    write_workspace(temp.path(), true);
    let config = default_asp_rust_config();
    let derivation = asp_rust_workspace_build_dag_with_metrics(temp.path(), &config)
        .expect("derive workspace Build DAG");
    let policy = AspRustWorkspacePolicy::new("fixture", config);

    let panic = catch_unwind(AssertUnwindSafe(|| {
        assert_asp_rust_workspace_build_dag_policy_with(
            derivation.build_dag,
            &policy,
            |_, config| config,
        )
    }))
    .expect_err("oversized module must fail the workspace gate");
    let message = if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        String::new()
    };
    assert!(message.contains("RUST-MOD-R002"), "{message}");
    assert!(message.contains("src/oversized.rs"), "{message}");
}

fn write_workspace(root: &Path, oversized: bool) {
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"core\", \"left\", \"right\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    write_package(root, "core", "fixture-core", None, oversized);
    write_package(root, "left", "fixture-left", Some("../core"), false);
    write_package(root, "right", "fixture-right", Some("../core"), false);
}

fn write_package(
    root: &Path,
    directory: &str,
    package_name: &str,
    core_dependency: Option<&str>,
    oversized: bool,
) {
    let package_root = root.join(directory);
    fs::create_dir_all(package_root.join("src")).expect("create package source");
    let dependency = core_dependency.map_or_else(String::new, |path| {
        format!("\n[dependencies]\nfixture-core = {{ path = \"{path}\" }}\n")
    });
    fs::write(
        package_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{dependency}"
        ),
    )
    .expect("write package manifest");
    let lib = if oversized {
        "//! Fixture crate.\nmod oversized;\n"
    } else {
        "//! Fixture crate.\n"
    };
    fs::write(package_root.join("src/lib.rs"), lib).expect("write package lib");
    if oversized {
        let source = std::iter::repeat_n("// source pressure\n", 1_001).collect::<String>();
        fs::write(package_root.join("src/oversized.rs"), source)
            .expect("write oversized source module");
    }
}
