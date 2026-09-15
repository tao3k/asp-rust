use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationSkillBinding, RustVerificationSkillDescriptor, RustVerificationTaskKind,
    RustVerificationTaskState, RustVerificationWaiver, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
    render_rust_verification_plan_json,
};
use tempfile::TempDir;

use crate::verification::support::{
    normalize_temp_root, public_api_profile_config, write_api_project,
};

#[test]
fn verification_receipt_clears_matching_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");
    let resolved_config = public_api_profile_config().with_verification_receipt(
        RustVerificationReceipt::passed(task.fingerprint.clone(), RustVerificationTaskKind::Stress),
    );

    let resolved_plan =
        plan_rust_project_verification_with_config(root, &resolved_config).expect("resolved plan");
    let stress = resolved_plan
        .tasks
        .iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");

    assert_eq!(stress.state, RustVerificationTaskState::Satisfied);
    assert!(resolved_plan.is_clear());
    assert_eq!(render_rust_verification_plan(&resolved_plan), "");
}

#[test]
fn performance_receipt_keeps_structured_state_searchable_after_compact_clears() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::LatencySensitive],
        ))
        .with_verification_skill_binding(
            RustVerificationTaskKind::Performance,
            RustVerificationSkillBinding::new("rust-verification-performance")
                .with_adapter("criterion"),
        )
        .with_verification_skill_descriptor(
            RustVerificationSkillDescriptor::criterion_performance(),
        );
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Performance)
        .expect("performance task");
    let resolved_config = config.with_verification_receipt(
        RustVerificationReceipt::passed(
            task.fingerprint.clone(),
            RustVerificationTaskKind::Performance,
        )
        .with_evidence("benchmark_command", "cargo bench --bench parser_hot_path")
        .with_evidence("baseline", "main@b0a8a7a")
        .with_evidence("regression_threshold", "5%")
        .with_evidence("latency_or_throughput", "-1.4% latency")
        .with_evidence("allocation_profile", "allocs/op unchanged")
        .with_evidence(
            "profile_artifact",
            "target/criterion/parser_hot_path/report/index.html",
        )
        .with_evidence_uri("target/criterion/parser_hot_path/report/index.html")
        .with_observed_at("2026-05-01T20:00:00Z"),
    );

    let resolved_plan =
        plan_rust_project_verification_with_config(root, &resolved_config).expect("resolved plan");
    let json = render_rust_verification_plan_json(&resolved_plan).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let performance = value["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .find(|task| task["kind"] == "performance")
        .expect("performance json task");

    assert!(resolved_plan.is_clear());
    assert_eq!(render_rust_verification_plan(&resolved_plan), "");
    assert_eq!(performance["state"], "satisfied");
    assert_eq!(
        performance["receipt_summary"],
        "passed (target/criterion/parser_hot_path/report/index.html)"
    );
    assert_eq!(
        performance["receipt_evidence_uri"],
        "target/criterion/parser_hot_path/report/index.html"
    );
    assert_eq!(performance["receipt_observed_at"], "2026-05-01T20:00:00Z");
    assert_eq!(
        performance["receipt_evidence"][0]["label"],
        "benchmark_command"
    );
    assert_eq!(
        performance["receipt_evidence"][0]["value"],
        "cargo bench --bench parser_hot_path"
    );
    assert_eq!(performance["receipt_evidence"][1]["label"], "baseline");
    assert_eq!(performance["receipt_evidence"][1]["value"], "main@b0a8a7a");
    assert_eq!(
        performance["receipt_evidence"][3]["label"],
        "latency_or_throughput"
    );
    assert_eq!(performance["receipt_evidence"][3]["value"], "-1.4% latency");
    assert!(value.get("skill_descriptors").is_none());
}

#[test]
fn failed_verification_receipt_remains_active() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan.active_tasks()[0];
    let failed_config =
        public_api_profile_config().with_verification_receipt(RustVerificationReceipt::failed(
            task.fingerprint.clone(),
            RustVerificationTaskKind::Stress,
            "p99 exceeded SLA at step 4",
        ));

    let failed_plan =
        plan_rust_project_verification_with_config(root, &failed_config).expect("failed plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&failed_plan), root);

    assert!(!failed_plan.is_clear());
    assert!(rendered.contains("|stress: failed"), "{rendered}");
    assert!(
        rendered.contains("p99 exceeded SLA at step 4"),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_failed_receipt_resolution", rendered);
}

#[test]
fn complete_verification_waiver_clears_matching_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan.active_tasks()[0];
    let waived_config =
        public_api_profile_config().with_verification_waiver(RustVerificationWaiver::new(
            task.fingerprint.clone(),
            "platform",
            "covered by upstream gateway test for this release",
            "2026-06-01",
        ));

    let waived_plan =
        plan_rust_project_verification_with_config(root, &waived_config).expect("waived plan");
    let stress = waived_plan
        .tasks
        .iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");

    assert_eq!(stress.state, RustVerificationTaskState::Waived);
    assert!(waived_plan.is_clear());
    assert_eq!(render_rust_verification_plan(&waived_plan), "");
}

#[test]
fn incomplete_verification_waiver_keeps_task_active_with_resolution_note() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan.active_tasks()[0];
    let incomplete_config = public_api_profile_config().with_verification_waiver(
        RustVerificationWaiver::new(task.fingerprint.clone(), "", "", ""),
    );

    let plan = plan_rust_project_verification_with_config(root, &incomplete_config).expect("plan");
    let stress = plan
        .tasks
        .iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(stress.state, RustVerificationTaskState::Pending);
    assert!(!plan.is_clear());
    assert!(
        rendered
            .contains("resolution: stress.waiver=incomplete: missing owner, reason, expires_at"),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_incomplete_waiver_resolution", rendered);
}
