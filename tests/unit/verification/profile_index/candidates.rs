use asp_rust::{
    build_rust_verification_profile_index, build_rust_verification_profile_index_with_config,
    render_rust_verification_profile_index, render_rust_verification_profile_index_json,
};
use tempfile::TempDir;

use crate::verification::support::normalize_temp_root;

use super::fixtures::{
    project_dependency_signal_config, write_api_fixture, write_branch_fixture,
    write_facade_export_fixture, write_nested_branch_fixture, write_responsibility_fixture,
};

#[test]
fn verification_profile_index_reports_parser_suggested_responsibilities() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_responsibility_fixture(root);

    let config = project_dependency_signal_config();
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");
    let rendered = normalize_temp_root(&render_rust_verification_profile_index(&index), root);

    assert!(!index.is_clear());
    assert!(index.needs_profile_configuration());
    assert!(!rendered.contains("hint_path"), "{rendered}");
    assert!(
        rendered.contains("[verify-profile] profile_hints"),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_profile_index_candidates", rendered);
}

#[test]
fn verification_profile_index_json_preserves_structured_candidates() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_api_fixture(root);

    let index = build_rust_verification_profile_index(root).expect("profile index");
    let json = render_rust_verification_profile_index_json(&index).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(
        value["candidates"][0]["hint_path"]
            .as_str()
            .expect("hint path")
            .replace('\\', "/"),
        "src/api.rs"
    );
    assert_eq!(value["candidates"][0]["state"], "missing_profile");
}

#[test]
fn branch_profile_candidate_aggregates_child_owner_signals() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_branch_fixture(root);

    let config = project_dependency_signal_config();
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");
    let rendered = normalize_temp_root(&render_rust_verification_profile_index(&index), root);

    assert!(index.needs_profile_configuration());
    assert_eq!(index.active_candidates().len(), 1, "{rendered}");
    assert_eq!(
        index.active_profile_hints()[0].owner_path,
        std::path::PathBuf::from("src/gateway/mod.rs")
    );
    insta::assert_snapshot!("verification_profile_index_branch_aggregate", rendered);
}

#[test]
fn nested_branch_profile_uses_nearest_owner_without_parent_duplication() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_nested_branch_fixture(root);

    let config = project_dependency_signal_config();
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");
    let rendered = normalize_temp_root(&render_rust_verification_profile_index(&index), root);

    assert!(
        rendered.contains("[verify-profile] src/gateway/mod.rs"),
        "{rendered}"
    );
    assert!(
        rendered.contains("[verify-profile] src/gateway/studio/mod.rs"),
        "{rendered}"
    );
    let parent_block = rendered
        .split("[verify-profile] src/gateway/studio/mod.rs")
        .next()
        .expect("parent block");
    assert!(
        !parent_block.contains("network_roots=axum::Router"),
        "{rendered}"
    );
}

#[test]
fn crate_root_facade_exports_do_not_shadow_owner_profiles() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_facade_export_fixture(root);

    let index = build_rust_verification_profile_index(root).expect("profile index");
    let hints = index.active_profile_hints();

    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].owner_path, std::path::PathBuf::from("src/api.rs"));
}
