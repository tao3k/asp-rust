use std::fs;
use std::path::Path;

use asp_rust::{
    build_rust_verification_analysis_profile_with_config, default_asp_rust_config,
    render_rust_verification_analysis_profile, render_rust_verification_analysis_profile_json,
};
use tempfile::TempDir;

use crate::verification::support::write_workspace_with_api_members;

#[test]
fn verification_analysis_profile_reports_project_scale_and_json() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_analysis_fixture(root);

    let profile =
        build_rust_verification_analysis_profile_with_config(root, &default_asp_rust_config())
            .expect("analysis profile");
    let rendered = render_rust_verification_analysis_profile(&profile);
    let json = render_rust_verification_analysis_profile_json(&profile).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(profile.package_count, 1);
    assert_eq!(profile.rust_file_count, 2);
    assert_eq!(profile.source_module_count, 2);
    assert!(profile.owner_branch_count >= 1, "{profile:?}");
    assert_eq!(profile.cargo_dependency_count, 2);
    assert_eq!(profile.packages.len(), 1);
    assert_eq!(profile.packages[0].rust_file_count, 2);
    assert_eq!(profile.packages[0].cargo_dependency_count, 2);
    assert!(rendered.starts_with("[verify-analysis] packages=1 rust_files=2"));
    assert!(rendered.contains("|package: . rust_files=2"));
    assert!(rendered.contains("cargo_dependencies=2"));
    assert_eq!(value["package_count"], 1);
    assert_eq!(value["packages"][0]["cargo_dependency_count"], 2);
}

#[test]
fn verification_analysis_profile_reports_workspace_member_scale() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_workspace_with_api_members(root);

    let profile =
        build_rust_verification_analysis_profile_with_config(root, &default_asp_rust_config())
            .expect("analysis profile");
    let rendered = render_rust_verification_analysis_profile(&profile);

    assert_eq!(profile.package_count, 2);
    assert_eq!(profile.rust_file_count, 4);
    assert_eq!(profile.source_module_count, 4);
    assert!(rendered.contains("|package: crates/api rust_files=2"));
    assert!(rendered.contains("|package: crates/worker rust_files=2"));
}

fn write_analysis_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"verification-analysis\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrayon = \"0.1\"\nsha2 = \"0.1\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\nuse rayon::prelude::*;\nuse sha2::Digest;\npub fn handle_request() {}\n",
    )
    .expect("write api");
}
