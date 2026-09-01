use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationTaskKind, RustVerificationTaskState, build_rust_verification_report_bundle,
    build_rust_verification_stability_index, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_report_artifact_json,
    render_rust_verification_stability_index, render_rust_verification_stability_index_json,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

#[test]
fn stability_index_renders_pending_availability_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan = plan_rust_project_verification_with_config(root, &availability_critical_config())
        .expect("plan");

    let index = build_rust_verification_stability_index(&plan);
    let rendered = normalize_temp_root(&render_rust_verification_stability_index(&index), root);
    let record = index.records.first().expect("stability record");

    assert_eq!(index.records.len(), 1);
    assert_eq!(record.state, RustVerificationTaskState::Pending);
    assert_eq!(index.records_for_owner("src/api.rs").len(), 1);
    assert_eq!(
        record.missing_receipt_evidence_keys(),
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
        rendered.contains("[stability-state] src/api.rs"),
        "{rendered}"
    );
    assert!(
        rendered.contains("phase=scheduled_regression"),
        "{rendered}"
    );
    assert!(
        rendered.contains("|missing: stability_command"),
        "{rendered}"
    );
}

#[test]
fn stability_index_keeps_receipt_evidence_searchable() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = availability_critical_config();
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stability)
        .expect("stability task");
    let resolved_config = config.with_verification_receipt(stability_receipt(&task.fingerprint));
    let resolved_plan =
        plan_rust_project_verification_with_config(root, &resolved_config).expect("plan");

    let index = build_rust_verification_stability_index(&resolved_plan);
    let json = render_rust_verification_stability_index_json(&index).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let record = index.records.first().expect("stability record");

    assert_eq!(record.state, RustVerificationTaskState::Satisfied);
    assert_eq!(
        record.receipt_evidence_value("resource_delta"),
        Some("rss +1.8 MiB fd +0 threads +0")
    );
    assert_eq!(
        index
            .records_with_receipt_evidence("stability_artifact")
            .len(),
        1
    );
    assert_eq!(value["records"][0]["state"], "satisfied");
    assert_eq!(
        value["records"][0]["receipt_evidence_uri"],
        "target/stability/api-long-run.json"
    );
}

#[test]
fn stability_report_artifact_is_source_baseline() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan = plan_rust_project_verification_with_config(root, &availability_critical_config())
        .expect("plan");

    let bundle = build_rust_verification_report_bundle(&plan);
    let artifact = bundle
        .artifact("stability_index_json")
        .expect("stability artifact");
    let json = render_rust_verification_report_artifact_json(&plan, "stability_index_json")
        .expect("render stability artifact")
        .expect("stability artifact");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse stability json");

    assert_eq!(artifact.task_count(), 1);
    assert_eq!(artifact.artifact_name, "stability_index.json");
    assert_eq!(artifact.task_kinds.len(), 1);
    assert!(
        artifact
            .task_kinds
            .contains(&RustVerificationTaskKind::Stability)
    );
    assert_eq!(
        value["records"][0]["required_evidence_keys"][0],
        "stability_command"
    );
}

fn availability_critical_config() -> asp_rust::AspRustConfig {
    default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
        "src/api.rs",
        [RustOwnerResponsibility::AvailabilityCritical],
    ))
}

fn stability_receipt(fingerprint: &str) -> RustVerificationReceipt {
    RustVerificationReceipt::passed(fingerprint, RustVerificationTaskKind::Stability)
        .with_evidence(
            "stability_command",
            "cargo run --bin api-long-run -- --iterations 10000",
        )
        .with_evidence("iteration_window", "10000 iterations warmup=500 samples=20")
        .with_evidence("latency_distribution", "p50=4ms p95=9ms p99=14ms max=21ms")
        .with_evidence("resource_delta", "rss +1.8 MiB fd +0 threads +0")
        .with_evidence("state_growth", "cache rows +0 artifact bytes +4096")
        .with_evidence("determinism", "20/20 replay fingerprints matched")
        .with_evidence("stability_artifact", "target/stability/api-long-run.json")
        .with_evidence_uri("target/stability/api-long-run.json")
        .with_observed_at("2026-05-02T20:00:00Z")
}
