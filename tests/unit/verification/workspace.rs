use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
};
use tempfile::TempDir;

use super::support::{normalize_temp_root, write_workspace_with_api_members};

#[test]
fn workspace_root_relative_profile_hint_targets_one_member_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_workspace_with_api_members(root);
    let config =
        default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
            "crates/api/src/api.rs",
            [RustOwnerResponsibility::PublicApi],
        ));

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert!(rendered.contains("crates/api/src/api.rs"), "{rendered}");
    assert!(!rendered.contains("crates/worker/src/api.rs"), "{rendered}");
    insta::assert_snapshot!("verification_workspace_root_relative_hint", rendered);
}

#[test]
fn workspace_package_relative_profile_hint_keeps_distinct_fingerprints() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_workspace_with_api_members(root);
    let config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi]),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let fingerprints = plan
        .active_tasks()
        .into_iter()
        .map(|task| task.fingerprint.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 2, "{rendered}");
    assert_eq!(fingerprints.len(), 2, "{rendered}");
    assert!(rendered.contains("crates/api/src/api.rs"), "{rendered}");
    assert!(rendered.contains("crates/worker/src/api.rs"), "{rendered}");
    insta::assert_snapshot!("verification_workspace_package_relative_hint", rendered);
}

#[test]
fn unmatched_workspace_profile_hint_renders_once() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_workspace_with_api_members(root);
    let config =
        default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
            "crates/missing/src/api.rs",
            [RustOwnerResponsibility::PublicApi],
        ));

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert!(
        rendered.contains("|responsibility_review: pending"),
        "{rendered}"
    );
    assert!(rendered.contains("crates/missing/src/api.rs"), "{rendered}");
    insta::assert_snapshot!("verification_workspace_unmatched_hint", rendered);
}
