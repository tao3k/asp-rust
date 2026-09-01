use std::collections::BTreeSet;

use asp_rust::{
    RustOwnerResponsibility, RustVerificationApiPathBaseline, RustVerificationReceipt,
    RustVerificationSkillBinding, RustVerificationTaskKind, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

#[test]
fn verification_api_path_baseline_requests_path_scoped_tasks() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_api_path_baseline(
        RustVerificationApiPathBaseline::new("src/api.rs", "get", "/v1/search")
            .with_responsibility(RustOwnerResponsibility::LatencySensitive)
            .with_responsibility(RustOwnerResponsibility::SecurityBoundary)
            .with_rationale("public search route has auth and latency SLO"),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);
    let active_kinds = plan
        .active_tasks()
        .into_iter()
        .map(|task| task.kind)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        active_kinds,
        BTreeSet::from([
            RustVerificationTaskKind::Stress,
            RustVerificationTaskKind::Performance,
            RustVerificationTaskKind::Security,
        ]),
        "{rendered}"
    );
    assert!(rendered.contains("api=GET:/v1/search"), "{rendered}");
    insta::assert_snapshot!("verification_api_path_baseline_tasks", rendered);
}

#[test]
fn verification_api_path_baseline_receipt_clears_only_matching_path() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config()
        .with_verification_api_path_baseline(
            RustVerificationApiPathBaseline::new("src/api.rs", "post", "/v1/orders")
                .with_task_kinds([RustVerificationTaskKind::Security])
                .with_rationale("order creation changes tenant authorization"),
        )
        .with_verification_api_path_baseline(
            RustVerificationApiPathBaseline::new("src/api.rs", "get", "/v1/orders/{id}")
                .with_task_kinds([RustVerificationTaskKind::Security])
                .with_rationale("order read path has separate authorization evidence"),
        );
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let first_path_task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| {
            task.kind == RustVerificationTaskKind::Security
                && task
                    .evidence
                    .iter()
                    .any(|fact| fact.label == "api_path" && fact.value == "POST /v1/orders")
        })
        .expect("POST /v1/orders task");
    let resolved_config = config.with_verification_receipt(RustVerificationReceipt::passed(
        first_path_task.fingerprint.clone(),
        RustVerificationTaskKind::Security,
    ));

    let resolved_plan =
        plan_rust_project_verification_with_config(root, &resolved_config).expect("resolved plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&resolved_plan), root);

    assert_eq!(initial_plan.active_tasks().len(), 2);
    assert_eq!(resolved_plan.active_tasks().len(), 1, "{rendered}");
    assert!(!rendered.contains("api=POST:/v1/orders"), "{rendered}");
    assert!(rendered.contains("api=GET:/v1/orders/{id}"), "{rendered}");
    insta::assert_snapshot!(
        "verification_api_path_baseline_receipt_clears_one_path",
        rendered
    );
}

#[test]
fn verification_api_path_baseline_with_skill_binding_stays_compact() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config()
        .with_verification_api_path_baseline(
            RustVerificationApiPathBaseline::new("src/api.rs", "post", "/v1/orders")
                .with_task_kinds([RustVerificationTaskKind::Security])
                .with_rationale("order creation changes tenant authorization"),
        )
        .with_verification_skill_binding(
            RustVerificationTaskKind::Security,
            RustVerificationSkillBinding::new("rust-verification-security")
                .with_adapter("security-scan"),
        );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert!(rendered.contains("skill=rust-verification-security@security-scan"));
    assert!(rendered.contains("api=POST:/v1/orders"));
    assert!(!rendered.contains("|fact: security.api_path"), "{rendered}");
    insta::assert_snapshot!(
        "verification_api_path_baseline_skill_binding_compact",
        rendered
    );
}

#[test]
fn verification_api_path_baseline_can_be_suppressed_with_rationale() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_api_path_baseline(
        RustVerificationApiPathBaseline::new("src/api.rs", "get", "/internal/health")
            .without_verification_tasks()
            .with_rationale("covered by platform health-check verification"),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    assert!(plan.is_clear(), "{plan:?}");
}

#[test]
fn verification_api_path_baseline_override_without_rationale_requests_review() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_api_path_baseline(
        RustVerificationApiPathBaseline::new("src/api.rs", "delete", "/v1/orders/{id}")
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
    insta::assert_snapshot!(
        "verification_api_path_baseline_override_without_rationale",
        rendered
    );
}

#[test]
fn verification_api_path_baseline_unmatched_owner_requests_review() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_api_path_baseline(
        RustVerificationApiPathBaseline::new("src/missing.rs", "get", "/v1/missing")
            .with_task_kinds([RustVerificationTaskKind::Security])
            .with_rationale("route config should point at the parser-known owner"),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::ResponsibilityReview
    );
    insta::assert_snapshot!("verification_api_path_baseline_unmatched_owner", rendered);
}
