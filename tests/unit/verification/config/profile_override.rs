use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationTaskKind,
    default_asp_rust_config, plan_rust_project_verification_with_config,
    render_rust_verification_plan,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

#[test]
fn verification_profile_can_request_owner_local_task_kinds() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi])
            .with_task_kinds([RustVerificationTaskKind::Security])
            .with_rationale("this slice changes route authorization"),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::Security
    );
    assert!(!rendered.contains("|stress:"), "{rendered}");
    insta::assert_snapshot!("verification_profile_owner_local_task_kinds", rendered);
}

#[test]
fn verification_profile_can_suppress_only_that_owner() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi])
            .without_verification_tasks()
            .with_rationale("covered by upstream gateway verification for this slice"),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    assert!(plan.is_clear(), "{plan:?}");
}
