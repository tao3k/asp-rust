use asp_rust::{
    RustVerificationReportOptions, RustVerificationReportSelectionReason,
    build_rust_verification_analysis_profile_with_config,
    build_rust_verification_report_bundle_with_options,
    build_rust_verification_report_selection_advice, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_report_selection_advice,
    render_rust_verification_report_selection_advice_json,
};
use tempfile::TempDir;

use crate::verification::support::{
    latency_sensitive_performance_config, write_api_project, write_workspace_with_api_members,
};

#[test]
fn report_selection_prefers_analysis_profile_when_scale_is_unknown() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let options = RustVerificationReportOptions::default().with_analysis_profile_artifact();
    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);

    let advice = render_rust_verification_report_selection_advice(&bundle, None);

    assert!(advice.starts_with(
        "[verify-report-select] artifacts=4 first=analysis_profile_json role=analysis_profile"
    ));
    assert!(advice.contains("reason=load_analysis_profile_before_payload_selection"));
    assert!(advice.contains(
        "|order: analysis_profile_json -> performance_index_json -> task_index_json -> verification_plan_json"
    ));
}

#[test]
fn report_selection_advice_json_preserves_structured_order() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let options = RustVerificationReportOptions::default().with_analysis_profile_artifact();
    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);

    let advice = build_rust_verification_report_selection_advice(&bundle, None);
    let json = render_rust_verification_report_selection_advice_json(&advice).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(advice.artifact_count, 4);
    assert_eq!(
        advice.reason,
        RustVerificationReportSelectionReason::LoadAnalysisProfileBeforePayloadSelection
    );
    assert_eq!(
        advice.first.as_ref().expect("first").key,
        "analysis_profile_json"
    );
    assert_eq!(advice.order[0].key, "analysis_profile_json");
    assert_eq!(advice.order[1].key, "performance_index_json");
    assert!(advice.scale.is_none());
    assert_eq!(
        value["reason"],
        "load_analysis_profile_before_payload_selection"
    );
    assert_eq!(value["first"]["role"], "analysis_profile");
    assert_eq!(value["order"][1]["persistence"], "source_baseline");
}

#[test]
fn report_selection_loads_baseline_first_for_known_small_projects() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let profile =
        build_rust_verification_analysis_profile_with_config(root, &config).expect("profile");
    let options = RustVerificationReportOptions::default().with_analysis_profile_artifact();
    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);

    let advice = render_rust_verification_report_selection_advice(&bundle, Some(&profile));

    assert!(advice.starts_with(
        "[verify-report-select] artifacts=4 first=performance_index_json role=baseline_evidence"
    ));
    assert!(advice.contains("reason=load_baseline_evidence_for_active_tasks"));
    assert!(advice.contains("|scale: packages=1 rust_files=2 source_modules=2 owner_branches=1"));
    assert!(advice.contains(
        "|order: performance_index_json -> task_index_json -> verification_plan_json -> analysis_profile_json"
    ));
}

#[test]
fn report_selection_uses_analysis_profile_first_for_workspace_scale() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_workspace_with_api_members(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let profile =
        build_rust_verification_analysis_profile_with_config(root, &config).expect("profile");
    let options = RustVerificationReportOptions::default().with_analysis_profile_artifact();
    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);

    let advice = render_rust_verification_report_selection_advice(&bundle, Some(&profile));

    assert!(advice.starts_with(
        "[verify-report-select] artifacts=4 first=analysis_profile_json role=analysis_profile"
    ));
    assert!(advice.contains("reason=large_analysis_surface_scope_window_first"));
    assert!(advice.contains("|scale: packages=2 rust_files=4 source_modules=4 owner_branches=2"));
    assert!(advice.contains(
        "|order: analysis_profile_json -> performance_index_json -> task_index_json -> verification_plan_json"
    ));
}

#[test]
fn report_selection_advice_reports_empty_bundles_structurally() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan =
        plan_rust_project_verification_with_config(root, &default_asp_rust_config()).expect("plan");
    let bundle = build_rust_verification_report_bundle_with_options(
        &plan,
        &RustVerificationReportOptions::default(),
    );

    let advice = build_rust_verification_report_selection_advice(&bundle, None);
    let compact = render_rust_verification_report_selection_advice(&bundle, None);
    let json = render_rust_verification_report_selection_advice_json(&advice).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(advice.artifact_count, 0);
    assert!(advice.first.is_none());
    assert!(advice.order.is_empty());
    assert_eq!(
        advice.reason,
        RustVerificationReportSelectionReason::NoActiveReportArtifacts
    );
    assert_eq!(
        compact,
        "[verify-report-select] artifacts=0 first=<none> reason=no_active_report_artifacts"
    );
    assert!(value.get("first").is_none());
    assert_eq!(value["reason"], "no_active_report_artifacts");
    assert_eq!(value["order"], serde_json::json!([]));
}
