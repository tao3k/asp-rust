use asp_rust::{
    RustOwnerResponsibility, RustVerificationApiPathBaseline, RustVerificationProfileHint,
    RustVerificationReceipt, RustVerificationStabilityPictureConfig,
    RustVerificationStabilityRunReceipt, RustVerificationTaskKind,
    build_rust_verification_stability_picture, compare_rust_verification_stability_runs,
    default_asp_rust_config, plan_rust_project_verification_with_config,
    render_rust_verification_stability_picture, render_rust_verification_stability_picture_json,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

#[test]
fn stability_picture_renders_configured_agent_actions() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = availability_critical_config().with_verification_stability_picture(
        RustVerificationStabilityPictureConfig::new()
            .with_min_iterations(10_000)
            .with_min_duration_seconds(900),
    );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let picture_config = config
        .verification_policy
        .stability_picture
        .as_ref()
        .expect("picture config");

    let picture = build_rust_verification_stability_picture(&plan, picture_config);
    let rendered = normalize_temp_root(&render_rust_verification_stability_picture(&picture), root);
    let record = picture.records.first().expect("picture record");

    assert_eq!(picture.records.len(), 1);
    assert_eq!(picture.actionable_records().len(), 1);
    assert_eq!(
        record.required_evidence_keys,
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
        rendered.contains("[stability-picture] records=1 actionable=1"),
        "{rendered}"
    );
    assert!(rendered.contains("|min_iterations: 10000"), "{rendered}");
    assert!(
        rendered.contains("add long-running simulation receipt iterations>=10000 duration_s>=900"),
        "{rendered}"
    );
    assert!(
        rendered.contains("add performance interface latency distribution"),
        "{rendered}"
    );
}

#[test]
fn stability_picture_respects_downstream_axis_configuration() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = availability_critical_config().with_verification_stability_picture(
        RustVerificationStabilityPictureConfig::new()
            .with_performance_interface_required(false)
            .with_resource_delta_required(false)
            .with_state_growth_required(false),
    );
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stability)
        .expect("stability task");
    let resolved_config =
        config.with_verification_receipt(partial_stability_receipt(&task.fingerprint));
    let plan = plan_rust_project_verification_with_config(root, &resolved_config).expect("plan");
    let picture_config = resolved_config
        .verification_policy
        .stability_picture
        .as_ref()
        .expect("picture config");

    let picture = build_rust_verification_stability_picture(&plan, picture_config);
    let record = picture.records.first().expect("picture record");

    assert_eq!(
        record.required_evidence_keys,
        [
            "stability_command",
            "iteration_window",
            "determinism",
            "stability_artifact",
        ]
    );
    assert!(record.missing_evidence_keys.is_empty(), "{record:?}");
    assert!(record.next_actions.is_empty(), "{record:?}");
}

#[test]
fn stability_picture_json_is_deterministic_for_same_plan() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = availability_critical_config().with_verification_stability_picture(
        RustVerificationStabilityPictureConfig::new().with_min_iterations(5000),
    );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let picture_config = config
        .verification_policy
        .stability_picture
        .as_ref()
        .expect("picture config");

    let first = build_rust_verification_stability_picture(&plan, picture_config);
    let second = build_rust_verification_stability_picture(&plan, picture_config);
    let first_json = render_rust_verification_stability_picture_json(&first).expect("json");
    let second_json = render_rust_verification_stability_picture_json(&second).expect("json");

    assert_eq!(first_json, second_json);
    assert!(first_json.contains("\"min_iterations\":5000"));
}

#[test]
fn stability_picture_surfaces_configuration_warnings() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let picture_config = RustVerificationStabilityPictureConfig::new()
        .with_long_running_simulation_required(false)
        .with_performance_interface_required(false)
        .with_resource_delta_required(false)
        .with_state_growth_required(false)
        .with_determinism_required(false)
        .with_stability_artifact_required(false)
        .with_min_iterations(100);
    let config =
        availability_critical_config().with_verification_stability_picture(picture_config.clone());
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    let picture = build_rust_verification_stability_picture(&plan, &picture_config);
    let rendered = render_rust_verification_stability_picture(&picture);
    let record = picture.records.first().expect("picture record");

    assert!(record.required_evidence_keys.is_empty());
    assert_eq!(record.config_warnings.len(), 2);
    assert!(
        rendered.contains("config_warnings: no_required_axes,min_iterations_without_long_run"),
        "{rendered}"
    );
}

#[test]
fn stability_picture_uses_owner_and_api_path_local_overrides() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let owner_picture = RustVerificationStabilityPictureConfig::new()
        .with_performance_interface_required(false)
        .with_resource_delta_required(false)
        .with_state_growth_required(false);
    let api_picture = RustVerificationStabilityPictureConfig::new()
        .with_long_running_simulation_required(false)
        .with_performance_interface_required(false)
        .with_resource_delta_required(false)
        .with_state_growth_required(false)
        .with_stability_artifact_required(false);
    let config = default_asp_rust_config()
        .with_verification_profile_hint(
            RustVerificationProfileHint::new(
                "src/api.rs",
                [RustOwnerResponsibility::AvailabilityCritical],
            )
            .with_stability_picture(owner_picture),
        )
        .with_verification_api_path_baseline(
            RustVerificationApiPathBaseline::new("src/api.rs", "GET", "/health")
                .with_responsibility(RustOwnerResponsibility::AvailabilityCritical)
                .with_task_kinds([RustVerificationTaskKind::Stability])
                .with_stability_picture(api_picture),
        )
        .with_verification_stability_picture(RustVerificationStabilityPictureConfig::new());
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let picture_config = config
        .verification_policy
        .stability_picture
        .as_ref()
        .expect("picture config");

    let picture = asp_rust::build_rust_verification_stability_picture_with_policy(
        &plan,
        &config.verification_policy,
        picture_config,
    );
    let api_record = picture
        .records
        .iter()
        .find(|record| {
            record
                .required_evidence_keys
                .contains(&"determinism".to_string())
                && !record
                    .required_evidence_keys
                    .contains(&"stability_artifact".to_string())
        })
        .expect("api-local picture record");

    assert_eq!(api_record.required_evidence_keys, ["determinism"]);
}

#[test]
fn stability_run_receipt_compares_against_baseline() {
    let baseline =
        RustVerificationStabilityRunReceipt::new("cargo run --bin api-long-run", 1000, 60)
            .with_resource_deltas(1024, 1, 0)
            .with_state_delta_bytes(2048)
            .with_determinism_hash("abc");
    let current =
        RustVerificationStabilityRunReceipt::new("cargo run --bin api-long-run", 1200, 75)
            .with_resource_deltas(4096, 2, 1)
            .with_state_delta_bytes(4096)
            .with_determinism_hash("def");

    let evidence = current.receipt_evidence();
    let delta = compare_rust_verification_stability_runs(&baseline, &current);

    assert!(evidence.iter().any(|(key, _)| *key == "iteration_window"));
    assert_eq!(delta.iteration_delta.as_i64(), 200);
    assert_eq!(delta.duration_delta_seconds.as_i64(), 15);
    assert_eq!(delta.rss_delta_bytes.expect("rss delta").as_i64(), 3072);
    assert_eq!(delta.fd_delta.expect("fd delta").as_i64(), 1);
    assert_eq!(delta.thread_delta.expect("thread delta").as_i64(), 1);
    assert_eq!(delta.state_delta_bytes.expect("state delta").as_i64(), 2048);
    assert!(delta.determinism_changed);
}

fn availability_critical_config() -> asp_rust::AspRustConfig {
    default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
        "src/api.rs",
        [RustOwnerResponsibility::AvailabilityCritical],
    ))
}

fn partial_stability_receipt(fingerprint: &str) -> RustVerificationReceipt {
    RustVerificationReceipt::passed(fingerprint, RustVerificationTaskKind::Stability)
        .with_evidence(
            "stability_command",
            "cargo run --bin api-long-run -- --iterations 10000",
        )
        .with_evidence("iteration_window", "10000 iterations warmup=500 samples=20")
        .with_evidence("determinism", "20/20 replay fingerprints matched")
        .with_evidence("stability_artifact", "target/stability/api-long-run.json")
        .with_evidence_uri("target/stability/api-long-run.json")
}
