use std::fs;

use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint,
    build_rust_verification_profile_index_with_config, default_asp_rust_config,
    render_rust_verification_profile_index,
};
use tempfile::TempDir;

use crate::verification::support::normalize_temp_root;

use super::fixtures::{project_dependency_signal_config, write_api_fixture};

#[test]
fn configured_profile_hint_clears_profile_index_reminder() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_api_fixture(root);

    let config =
        default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::AvailabilityCritical,
            ],
        ));
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");

    assert!(index.is_clear());
    assert!(!index.needs_profile_configuration());
    assert_eq!(render_rust_verification_profile_index(&index), "");
}

#[test]
fn partial_profile_hint_renders_profile_drift() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_api_fixture(root);

    let config = project_dependency_signal_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi]),
    );
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");
    let rendered = normalize_temp_root(&render_rust_verification_profile_index(&index), root);

    assert!(!index.is_clear());
    assert!(!index.needs_profile_configuration());
    assert!(
        !rendered.contains("[verify-profile] profile_hints"),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_profile_index_drift", rendered);
}

#[test]
fn workspace_profile_hint_paths_are_project_relative() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/api\"]\n",
    )
    .expect("write workspace manifest");
    let package_root = root.join("crates/api");
    fs::create_dir_all(&package_root).expect("create member");
    write_api_fixture(&package_root);

    let config = project_dependency_signal_config();
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");
    let hints = index.active_profile_hints();

    assert_eq!(hints.len(), 1);
    assert_eq!(
        hints[0].owner_path,
        std::path::PathBuf::from("crates/api/src/api.rs")
    );
}
