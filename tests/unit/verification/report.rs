use asp_rust::{
    RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID, RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION,
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationReportArtifactRole,
    RustVerificationReportOptions, RustVerificationReportPersistence,
    RustVerificationReportSidecarRole, RustVerificationReportTraceConfig,
    RustVerificationSkillBinding, RustVerificationStabilityPictureConfig, RustVerificationTaskKind,
    RustVerificationTraceMaxSeconds, build_rust_verification_report_bundle,
    build_rust_verification_report_bundle_with_options, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
    render_rust_verification_report_artifact_json,
    render_rust_verification_report_artifact_json_with_config,
    render_rust_verification_report_bundle, render_rust_verification_report_bundle_json,
};
use tempfile::TempDir;

use crate::verification::support::write_api_project;

#[test]
fn verification_report_bundle_materializes_required_artifacts() {
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
        );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    let rendered = render_rust_verification_plan(&plan);
    let bundle = build_rust_verification_report_bundle(&plan);
    let compact_bundle = render_rust_verification_report_bundle(&bundle);
    let json = render_rust_verification_report_bundle_json(&plan).expect("report json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let perf_json = render_rust_verification_report_artifact_json(&plan, "performance_index_json")
        .expect("render performance artifact")
        .expect("performance artifact");
    let perf_value: serde_json::Value = serde_json::from_str(&perf_json).expect("parse perf json");
    let task_json = render_rust_verification_report_artifact_json(&plan, "task_index_json")
        .expect("render task artifact")
        .expect("task artifact");
    let task_value: serde_json::Value = serde_json::from_str(&task_json).expect("parse task json");

    assert!(rendered.contains("render_rust_verification_report_bundle_json"));
    assert!(compact_bundle.starts_with("[verify-report-bundle] artifacts=3"));
    assert!(compact_bundle.contains("schema=rust_verification_report_manifest/1"));
    assert!(compact_bundle.contains(
        "|artifact: role=baseline_evidence key=performance_index_json persistence=source_baseline file=performance_index.json tasks=1 trace=performance max_s=300 sample_ms=250 raw=true template=performance-index"
    ));
    assert!(compact_bundle.contains(
        "|renderer: performance_index_json=build_rust_verification_performance_index + render_rust_verification_performance_index_json"
    ));
    assert_eq!(bundle.artifacts.len(), 3);
    assert!(bundle.artifact("analysis_profile_json").is_none());
    assert_eq!(
        bundle
            .artifact("verification_plan_json")
            .expect("plan artifact")
            .task_count(),
        1
    );
    assert_eq!(
        bundle
            .artifact("task_index_json")
            .expect("task artifact")
            .role,
        RustVerificationReportArtifactRole::SkillDispatchIndex
    );
    assert_eq!(
        bundle
            .artifact("task_index_json")
            .expect("task artifact")
            .persistence,
        RustVerificationReportPersistence::SourceBaseline
    );
    assert_eq!(
        bundle
            .artifact("performance_index_json")
            .expect("performance artifact")
            .role,
        RustVerificationReportArtifactRole::BaselineEvidence
    );
    assert_eq!(
        bundle
            .artifact("performance_index_json")
            .expect("performance artifact")
            .persistence,
        RustVerificationReportPersistence::SourceBaseline
    );
    assert_eq!(
        bundle
            .artifact("verification_plan_json")
            .expect("plan artifact")
            .role,
        RustVerificationReportArtifactRole::PromptState
    );
    assert_eq!(
        bundle
            .artifact("verification_plan_json")
            .expect("plan artifact")
            .persistence,
        RustVerificationReportPersistence::RuntimeCache
    );
    assert_eq!(bundle.source_baseline_artifacts().len(), 2);
    assert_eq!(bundle.runtime_cache_artifacts().len(), 1);
    assert_eq!(
        bundle
            .artifacts_for_role(RustVerificationReportArtifactRole::BaselineEvidence)
            .len(),
        1
    );
    assert_eq!(
        bundle
            .artifact("performance_index_json")
            .expect("performance artifact")
            .trace
            .as_ref()
            .expect("trace")
            .max_seconds,
        Some(RustVerificationTraceMaxSeconds::new(300))
    );
    assert_eq!(perf_value["records"][0]["state"], "pending");
    assert_eq!(task_value["records"][0]["kind"], "performance");
    assert_eq!(
        value["artifacts"][0]["artifact_name"],
        "verification_plan.json"
    );
    assert_eq!(
        value["schema"]["schema_id"],
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID
    );
    assert_eq!(
        value["schema"]["schema_version"],
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(value["artifacts"][1]["artifact_name"], "task_index.json");
    assert_eq!(
        value["artifacts"][2]["artifact_name"],
        "performance_index.json"
    );
    assert_eq!(
        value["artifacts"][2]["template"]["template_id"],
        "performance-index"
    );
    assert_eq!(value["artifacts"][2]["role"], "baseline_evidence");
    assert!(value["artifacts"][2].get("payload").is_none());
    assert!(value.get("sidecars").is_none());
}

#[test]
fn verification_report_bundle_can_include_analysis_profile_artifact_explicitly() {
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
        );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let options = RustVerificationReportOptions::default().with_analysis_profile_artifact();

    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);
    let compact_bundle = render_rust_verification_report_bundle(&bundle);
    let analysis = bundle
        .artifact("analysis_profile_json")
        .expect("analysis artifact");
    let analysis_json = render_rust_verification_report_artifact_json_with_config(
        &plan,
        &config,
        "analysis_profile_json",
    )
    .expect("render analysis artifact")
    .expect("analysis artifact");
    let value: serde_json::Value = serde_json::from_str(&analysis_json).expect("parse analysis");

    assert_eq!(bundle.artifacts.len(), 4);
    assert!(compact_bundle.contains(
        "|artifact: role=analysis_profile key=analysis_profile_json persistence=runtime_cache file=analysis_profile.json tasks=0 trace=analysis max_s=60 sample_ms=1000 template=verification-analysis-profile"
    ));
    assert_eq!(analysis.artifact_name, "analysis_profile.json");
    assert_eq!(
        analysis.role,
        RustVerificationReportArtifactRole::AnalysisProfile
    );
    assert_eq!(analysis.task_count(), 0);
    assert_eq!(
        bundle
            .artifacts_for_role(RustVerificationReportArtifactRole::AnalysisProfile)
            .len(),
        1
    );
    assert_eq!(
        analysis.persistence,
        RustVerificationReportPersistence::RuntimeCache
    );
    assert_eq!(
        analysis.template.as_ref().expect("template").template_id,
        "verification-analysis-profile"
    );
    assert_eq!(analysis.trace.as_ref().expect("trace").profile, "analysis");
    assert_eq!(value["package_count"], 1);
    assert_eq!(value["rust_file_count"], 2);
}

#[test]
fn verification_report_bundle_materializes_stability_picture_artifact() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::AvailabilityCritical],
        ))
        .with_verification_stability_picture(
            RustVerificationStabilityPictureConfig::new().with_min_iterations(5000),
        );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    let bundle = build_rust_verification_report_bundle(&plan);
    let compact_bundle = render_rust_verification_report_bundle(&bundle);
    let picture = bundle
        .artifact("stability_picture_json")
        .expect("stability picture artifact");
    let picture_json = render_rust_verification_report_artifact_json_with_config(
        &plan,
        &config,
        "stability_picture_json",
    )
    .expect("render stability picture")
    .expect("stability picture");
    let value: serde_json::Value = serde_json::from_str(&picture_json).expect("parse picture");

    assert_eq!(bundle.artifacts.len(), 3);
    assert!(bundle.artifact("stability_index_json").is_some());
    assert_eq!(picture.artifact_name, "stability_picture.json");
    assert_eq!(
        picture.role,
        RustVerificationReportArtifactRole::PromptState
    );
    assert_eq!(
        picture.persistence,
        RustVerificationReportPersistence::RuntimeCache
    );
    assert!(compact_bundle.contains(
        "|artifact: role=prompt_state key=stability_picture_json persistence=runtime_cache file=stability_picture.json tasks=1 trace=stability-picture max_s=60 sample_ms=1000 template=stability-picture"
    ));
    assert_eq!(value["config"]["min_iterations"], 5000);
    assert_eq!(
        value["records"][0]["required_evidence_keys"][0],
        "stability_command"
    );
}

#[test]
fn verification_report_bundle_allows_agent_trace_overrides() {
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
        );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let options = RustVerificationReportOptions::default().with_artifact_trace(
        "performance_index_json",
        RustVerificationReportTraceConfig::new("agent-fast-performance")
            .with_max_seconds(45)
            .with_sample_interval_ms(500),
    );

    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);
    let performance = bundle
        .artifact("performance_index_json")
        .expect("performance artifact");

    assert_eq!(
        performance.trace.as_ref().expect("trace").profile,
        "agent-fast-performance"
    );
    assert_eq!(
        performance.trace.as_ref().expect("trace").max_seconds,
        Some(RustVerificationTraceMaxSeconds::new(45))
    );
}

#[test]
fn verification_report_bundle_exposes_selection_advice_sidecar_contract() {
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
        );
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let options = RustVerificationReportOptions::default().with_selection_advice_sidecar();

    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);
    let compact_bundle = render_rust_verification_report_bundle(&bundle);
    let value = serde_json::to_value(&bundle).expect("bundle json");
    let sidecar = bundle
        .sidecar("selection_advice_json")
        .expect("selection sidecar");

    assert_eq!(bundle.sidecars.len(), 1);
    assert_eq!(
        bundle.schema.schema_id,
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID
    );
    assert_eq!(
        bundle.schema.schema_version,
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(bundle.runtime_cache_sidecars().len(), 1);
    assert_eq!(bundle.source_baseline_sidecars().len(), 0);
    assert_eq!(
        sidecar.role,
        RustVerificationReportSidecarRole::SelectionAdvice
    );
    assert_eq!(sidecar.artifact_name, "selection_advice.json");
    assert_eq!(
        bundle
            .sidecars_for_role(RustVerificationReportSidecarRole::SelectionAdvice)
            .len(),
        1
    );
    assert!(compact_bundle.contains(
        "|sidecar: role=selection_advice key=selection_advice_json persistence=runtime_cache file=selection_advice.json"
    ));
    assert_eq!(value["sidecars"][0]["key"], "selection_advice_json");
    assert_eq!(value["sidecars"][0]["role"], "selection_advice");
}

#[test]
fn verification_report_bundle_is_empty_without_active_tasks() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan =
        plan_rust_project_verification_with_config(root, &default_asp_rust_config()).expect("plan");

    let bundle = build_rust_verification_report_bundle(&plan);
    let json = render_rust_verification_report_bundle_json(&plan).expect("report json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert!(bundle.is_empty());
    assert_eq!(value["artifacts"], serde_json::json!([]));
}
