use asp_rust::{
    RustOwnerResponsibility, RustVerificationPhase, RustVerificationPolicy,
    RustVerificationProfileHint, RustVerificationRequirement, RustVerificationTaskContract,
    RustVerificationTaskKind, default_asp_rust_config, plan_rust_project_verification_with_config,
    render_rust_verification_plan,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

#[test]
fn verification_profile_override_without_rationale_requests_review() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi])
            .with_task_kinds([RustVerificationTaskKind::Security]),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 2, "{rendered}");
    assert!(
        plan.active_tasks()
            .iter()
            .any(|task| task.kind == RustVerificationTaskKind::ResponsibilityReview),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_profile_override_without_rationale", rendered);
}

#[test]
fn verification_profile_suppression_without_rationale_requests_review() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi])
            .without_verification_tasks(),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::ResponsibilityReview
    );
    assert!(!rendered.contains("|stress:"), "{rendered}");
    insta::assert_snapshot!(
        "verification_profile_suppression_without_rationale",
        rendered
    );
}

#[test]
fn verification_profile_empty_responsibilities_request_review() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", Vec::<RustOwnerResponsibility>::new()),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::ResponsibilityReview
    );
    insta::assert_snapshot!("verification_profile_empty_responsibilities", rendered);
}

#[test]
fn verification_profile_disabled_owner_task_requests_review() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let policy = RustVerificationPolicy::default()
        .with_profile_hint(
            RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi])
                .with_task_kinds([RustVerificationTaskKind::Stress])
                .with_rationale("route load test is expected for this owner"),
        )
        .with_disabled_task_kind(RustVerificationTaskKind::Stress);
    let config = default_asp_rust_config().with_verification_policy(policy);

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::ResponsibilityReview
    );
    assert!(!rendered.contains("|stress:"), "{rendered}");
    insta::assert_snapshot!("verification_profile_disabled_owner_task", rendered);
}

#[test]
fn verification_profile_unused_owner_contract_requests_review() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let unused_contract = RustVerificationTaskContract::new(
        RustVerificationPhase::BeforeRelease,
        "security skill must report authz probes",
        [RustVerificationRequirement::new(
            "authz",
            "authorization result",
        )],
    );
    let config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi])
            .with_task_contract(RustVerificationTaskKind::Security, unused_contract),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 2, "{rendered}");
    assert!(
        plan.active_tasks()
            .iter()
            .any(|task| task.kind == RustVerificationTaskKind::ResponsibilityReview),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_profile_unused_owner_contract", rendered);
}
