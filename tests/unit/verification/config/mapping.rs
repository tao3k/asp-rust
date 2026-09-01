use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationTaskKind,
    default_asp_rust_config, plan_rust_project_verification_with_config,
    render_rust_verification_plan,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

#[test]
fn verification_responsibility_mapping_is_configurable() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::PublicApi],
        ))
        .with_verification_responsibility_task_kinds(
            RustOwnerResponsibility::PublicApi,
            [RustVerificationTaskKind::Security],
        );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::Security
    );
    assert!(!rendered.contains("|stress:"), "{rendered}");
    insta::assert_snapshot!("verification_custom_responsibility_mapping", rendered);
}

#[test]
fn verification_responsibility_mapping_can_suppress_default_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::PublicApi],
        ))
        .with_verification_responsibility_task_kinds(
            RustOwnerResponsibility::PublicApi,
            Vec::<RustVerificationTaskKind>::new(),
        );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    assert!(plan.is_clear(), "{plan:?}");
}

#[test]
fn verification_latency_sensitive_profile_requests_performance_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::LatencySensitive]),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(plan.active_tasks().len(), 1, "{rendered}");
    assert_eq!(
        plan.active_tasks()[0].kind,
        RustVerificationTaskKind::Performance
    );
    assert!(
        rendered.contains("benchmark_command,baseline,regression_threshold"),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_latency_sensitive_performance_task", rendered);
}

#[test]
fn verification_availability_critical_profile_requests_stability_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config =
        default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::AvailabilityCritical],
        ));

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);
    let active_kinds = plan
        .active_tasks()
        .into_iter()
        .map(|task| task.kind)
        .collect::<std::collections::BTreeSet<_>>();
    let stability_task = plan
        .tasks
        .iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stability)
        .expect("stability task");
    let required_stability_evidence = stability_task
        .required_evidence
        .iter()
        .map(|requirement| requirement.key.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        active_kinds,
        std::collections::BTreeSet::from([
            RustVerificationTaskKind::Chaos,
            RustVerificationTaskKind::Stability,
        ]),
        "{rendered}"
    );
    assert_eq!(
        required_stability_evidence,
        [
            "stability_command",
            "iteration_window",
            "latency_distribution",
            "resource_delta",
            "state_growth",
            "determinism",
            "stability_artifact",
        ]
    );
    assert!(
        rendered.contains("profile declares availability-critical Rust owner"),
        "{rendered}"
    );
}
