use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::runner::{
    RustHarnessRunScope, analyze_rust_project_once, run_rust_project_harness_with_config_for_scope,
};
use crate::{RustHarnessConfig, plan_rust_verification_from_harness_analysis};

fn write_file(root: &Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write file");
}

#[test]
fn report_and_verification_share_one_parse_pass() {
    let project = TempDir::new().expect("temp project");
    write_file(
        project.path(),
        "Cargo.toml",
        "[package]\nname = \"single-pass\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write_file(
        project.path(),
        "src/lib.rs",
        "pub fn answer() -> u32 { 42 }\n",
    );
    let config = RustHarnessConfig::default();

    let analysis = analyze_rust_project_once(project.path(), &config, RustHarnessRunScope::Package)
        .expect("analyze project once");
    let _report = analysis.to_report(&config);
    assert_eq!(analysis.parse_pass_count(), 1);
    let _plan = plan_rust_verification_from_harness_analysis(analysis, &config.verification_policy);
}

#[test]
fn package_scope_requires_a_parsed_cargo_graph_anchor() {
    let project = TempDir::new().expect("temp project");
    write_file(project.path(), "src/lib.rs", "pub fn unowned() {}\n");

    let error = run_rust_project_harness_with_config_for_scope(
        project.path(),
        &RustHarnessConfig::default(),
        RustHarnessRunScope::Package,
    )
    .expect_err("manifest-free directory must not become a Cargo package atom");

    assert!(
        error.contains("parse Cargo package graph anchor"),
        "{error}"
    );
    assert!(error.contains("Cargo.toml"), "{error}");
}

#[test]
fn package_scope_does_not_expand_local_path_dependencies() {
    let fixture = TempDir::new().expect("temp dir");
    let app = fixture.path().join("app");
    let dependency = fixture.path().join("dependency");

    write_file(
        &app,
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\ndependency = { path = \"../dependency\" }\n",
    );
    write_file(&app, "src/lib.rs", "pub fn app() {}\n");
    write_file(
        &dependency,
        "Cargo.toml",
        "[package]\nname = \"dependency\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(&dependency, "src/lib.rs", "pub fn dependency() {}\n");

    let config = RustHarnessConfig::default();
    let package_report =
        run_rust_project_harness_with_config_for_scope(&app, &config, RustHarnessRunScope::Package)
            .expect("package report");
    let workspace_report = run_rust_project_harness_with_config_for_scope(
        &app,
        &config,
        RustHarnessRunScope::ProjectWorkspace,
    )
    .expect("workspace report");

    assert!(
        package_report
            .modules
            .iter()
            .all(|module| module.path.starts_with(&app)),
        "package scope must not analyze a local path dependency"
    );
    assert!(
        workspace_report
            .modules
            .iter()
            .any(|module| module.path.starts_with(&dependency)),
        "project/workspace scope must retain dependency-topology discovery"
    );
}
