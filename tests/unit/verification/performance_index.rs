use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationSkillBinding, RustVerificationSkillDescriptor, RustVerificationTaskKind,
    RustVerificationTaskState, build_rust_verification_performance_index, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_performance_index,
    render_rust_verification_performance_index_json, render_rust_verification_plan,
};
use tempfile::TempDir;

use crate::verification::support::{
    normalize_temp_root, public_api_profile_config, write_api_project,
    write_workspace_with_api_members,
};

#[test]
fn performance_index_keeps_satisfied_receipt_searchable_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = rust_native_performance_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Performance)
        .expect("performance task");
    let resolved_config = config.with_verification_receipt(performance_receipt(&task.fingerprint));
    let resolved_plan =
        plan_rust_project_verification_with_config(root, &resolved_config).expect("resolved plan");

    let index = build_rust_verification_performance_index(&resolved_plan);
    let rendered = normalize_temp_root(&render_rust_verification_performance_index(&index), root);
    let json = render_rust_verification_performance_index_json(&index).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let record = index.records.first().expect("performance record");

    assert_eq!(index.records.len(), 1);
    assert_eq!(record.state, RustVerificationTaskState::Satisfied);
    assert_eq!(
        record.receipt_evidence_value("baseline"),
        Some("main@b0a8a7a")
    );
    assert_eq!(
        record.receipt_evidence_value("benchmark_command"),
        Some("cargo bench --bench query_search_microbench")
    );
    assert_eq!(
        record.receipt_evidence_value("latency_or_throughput"),
        Some("-1.4% latency")
    );
    assert_eq!(index.records_for_owner("src/api.rs").len(), 1);
    assert_eq!(
        index
            .records_with_receipt_evidence("profile_artifact")
            .len(),
        1
    );
    assert_eq!(value["records"][0]["state"], "satisfied");
    assert_eq!(
        value["records"][0]["receipt_observed_at"],
        "2026-05-01T20:00:00Z"
    );
    assert_eq!(
        value["records"][0]["receipt_evidence_uri"],
        "target/criterion/parser_hot_path/report/index.html"
    );
    assert_eq!(
        value["records"][0]["receipt_evidence"][0]["label"],
        "benchmark_command"
    );
    insta::assert_snapshot!("performance_index_satisfied_receipt", rendered);
}

#[test]
fn performance_index_renders_pending_task_without_skill_manual_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan = plan_rust_project_verification_with_config(root, &rust_native_performance_config())
        .expect("plan");

    let index = build_rust_verification_performance_index(&plan);
    let rendered = normalize_temp_root(&render_rust_verification_performance_index(&index), root);
    let record = index.records.first().expect("performance record");

    assert_eq!(record.state, RustVerificationTaskState::Pending);
    assert_eq!(
        record.missing_receipt_evidence_keys(),
        [
            "benchmark_command",
            "baseline",
            "regression_threshold",
            "latency_or_throughput",
            "allocation_profile",
            "profile_artifact",
        ]
    );
    assert_eq!(
        record.required_evidence_keys,
        [
            "benchmark_command",
            "baseline",
            "regression_threshold",
            "latency_or_throughput",
            "allocation_profile",
            "profile_artifact",
        ]
    );
    assert!(!rendered.contains("[skill-contract]"), "{rendered}");
    assert!(!rendered.contains("|standard:"), "{rendered}");
    insta::assert_snapshot!("performance_index_pending_task", rendered);
}

#[test]
fn performance_policy_requires_plan_and_performance_report_obligations() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan = plan_rust_project_verification_with_config(root, &rust_native_performance_config())
        .expect("plan");
    let rendered = render_rust_verification_plan(&plan);

    assert_eq!(plan.report_obligations.len(), 3);
    assert_eq!(plan.report_obligations[0].key, "verification_plan_json");
    assert_eq!(plan.report_obligations[0].task_count(), 1);
    assert_eq!(plan.report_obligations[1].key, "task_index_json");
    assert_eq!(plan.report_obligations[1].task_count(), 1);
    assert_eq!(plan.report_obligations[2].key, "performance_index_json");
    assert_eq!(plan.report_obligations[2].task_count(), 1);
    assert_eq!(
        plan.report_obligations[2].renderer,
        "build_rust_verification_performance_index + render_rust_verification_performance_index_json"
    );
    assert!(
        rendered.contains(
            "[verify-report]\n   |bundle: renderer=render_rust_verification_report_bundle_json artifact=verification_report_bundle.json artifacts=3\n   |required: verification_plan_json renderer=render_rust_verification_plan_json artifact=verification_plan.json tasks=1 kinds=performance\n   |required: task_index_json renderer=build_rust_verification_task_index + render_rust_verification_task_index_json artifact=task_index.json tasks=1 kinds=performance\n   |required: performance_index_json renderer=build_rust_verification_performance_index + render_rust_verification_performance_index_json artifact=performance_index.json tasks=1 kinds=performance"
        ),
        "{rendered}"
    );
}

#[test]
fn performance_index_marks_partial_failed_receipt_missing_keys_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = rust_native_performance_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Performance)
        .expect("performance task");
    let resolved_config = config.with_verification_receipt(
        RustVerificationReceipt::failed(
            task.fingerprint.clone(),
            RustVerificationTaskKind::Performance,
            "latency regression exceeded threshold",
        )
        .with_evidence("benchmark_command", "cargo bench --bench parser_hot_path")
        .with_evidence("regression_threshold", "5%")
        .with_evidence("latency_or_throughput", "+11.2% latency"),
    );
    let plan =
        plan_rust_project_verification_with_config(root, &resolved_config).expect("resolved plan");

    let index = build_rust_verification_performance_index(&plan);
    let rendered = normalize_temp_root(&render_rust_verification_performance_index(&index), root);
    let record = index.records.first().expect("performance record");

    assert_eq!(record.state, RustVerificationTaskState::Failed);
    assert_eq!(
        index
            .records_in_state(RustVerificationTaskState::Failed)
            .len(),
        1
    );
    assert_eq!(
        record.missing_receipt_evidence_keys(),
        ["baseline", "allocation_profile", "profile_artifact"]
    );
    insta::assert_snapshot!("performance_index_partial_failed_receipt", rendered);
}

#[test]
fn performance_index_workspace_package_queries_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_workspace_with_api_members(root);
    let config = default_asp_rust_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::LatencySensitive],
        ))
        .with_verification_skill_binding(
            RustVerificationTaskKind::Performance,
            RustVerificationSkillBinding::new("rust-verification-performance")
                .with_adapter("criterion"),
        );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    let index = build_rust_verification_performance_index(&plan);
    let rendered = normalize_temp_root(&render_rust_verification_performance_index(&index), root);

    assert_eq!(index.records.len(), 2);
    assert_eq!(index.records_for_owner("src/api.rs").len(), 2);
    assert_eq!(index.records_for_package("crates/api").len(), 1);
    assert_eq!(index.records_for_package("crates/worker").len(), 1);
    assert_eq!(
        index
            .records_in_state(RustVerificationTaskState::Pending)
            .len(),
        2
    );
    insta::assert_snapshot!("performance_index_workspace_package_queries", rendered);
}

#[test]
fn performance_index_omits_non_performance_tasks() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan = plan_rust_project_verification_with_config(root, &public_api_profile_config())
        .expect("plan");

    let index = build_rust_verification_performance_index(&plan);

    assert!(index.is_empty());
    assert_eq!(render_rust_verification_performance_index(&index), "");
}

fn rust_native_performance_config() -> asp_rust::AspRustConfig {
    default_asp_rust_config()
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
        )
}

fn performance_receipt(fingerprint: &str) -> RustVerificationReceipt {
    RustVerificationReceipt::passed(fingerprint, RustVerificationTaskKind::Performance)
        .with_evidence(
            "benchmark_command",
            "cargo bench --bench query_search_microbench",
        )
        .with_evidence("baseline", "main@b0a8a7a")
        .with_evidence("regression_threshold", "5%")
        .with_evidence("latency_or_throughput", "-1.4% latency")
        .with_evidence("allocation_profile", "allocs/op unchanged")
        .with_evidence(
            "profile_artifact",
            "target/criterion/parser_hot_path/report/index.html",
        )
        .with_evidence_uri("target/criterion/parser_hot_path/report/index.html")
        .with_observed_at("2026-05-01T20:00:00Z")
}
