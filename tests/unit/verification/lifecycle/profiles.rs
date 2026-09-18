use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationTaskKind,
    default_asp_rust_config, plan_rust_project_verification_with_config,
    render_rust_verification_plan,
};
use tempfile::TempDir;

use crate::verification::support::{
    normalize_temp_root, write_api_project, write_branch_project, write_external_dependency_project,
};

#[test]
fn verification_profile_tasks_render_compact_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config =
        default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [
                RustOwnerResponsibility::PublicApi,
                RustOwnerResponsibility::LatencySensitive,
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::SecurityBoundary,
            ],
        ));

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 4, "{rendered}");
    insta::assert_snapshot!("verification_profile_tasks", rendered);
}

#[test]
fn parser_facts_can_reject_wrong_responsibility_profile() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_external_dependency_project(root);
    let config =
        default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/domain.rs",
            [RustOwnerResponsibility::PureDomainLogic],
        ));

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::ResponsibilityReview
    );
    insta::assert_snapshot!("verification_profile_conflict", rendered);
}

#[test]
fn parser_facts_generate_regression_task_for_large_owner_branch() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_branch_project(root);

    let plan =
        plan_rust_project_verification_with_config(root, &default_asp_rust_config()).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::Regression
    );
    insta::assert_snapshot!("verification_parser_regression_task", rendered);
}
