use std::path::Path;

use asp_rust::{
    RustVerificationReportEntryAction, RustVerificationReportOptions,
    RustVerificationReportSidecarRole, RustVerificationReportWriteConfig,
    build_rust_verification_analysis_profile_with_config,
    build_rust_verification_report_bundle_with_options,
    build_rust_verification_report_entry_advice,
    build_rust_verification_report_entry_advice_with_receipt, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_report_entry_advice,
    render_rust_verification_report_entry_advice_json,
    write_rust_verification_reports_with_options,
};
use tempfile::TempDir;

use crate::verification::support::{latency_sensitive_performance_config, write_api_project};

#[test]
fn report_entry_prefers_persisted_selection_sidecar_when_receipt_is_available() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let profile =
        build_rust_verification_analysis_profile_with_config(root, &config).expect("profile");
    let options = RustVerificationReportOptions::default()
        .with_analysis_profile_artifact()
        .with_selection_advice_sidecar();
    let bundle = build_rust_verification_report_bundle_with_options(&plan, &options);
    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");
    let receipt = write_rust_verification_reports_with_options(
        &plan,
        &config,
        &RustVerificationReportWriteConfig::new(root, &source_dir, &cache_dir),
        &options,
    )
    .expect("write reports");
    let selection_path = cache_dir.join("selection_advice.json");

    let advice = build_rust_verification_report_entry_advice_with_receipt(
        &bundle,
        Some(&profile),
        Some(&receipt),
    );
    let compact = render_rust_verification_report_entry_advice(&advice);
    let json = render_rust_verification_report_entry_advice_json(&advice).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(
        advice.action,
        RustVerificationReportEntryAction::LoadSelectionAdviceSidecar
    );
    assert!(advice.schema_compatibility.is_supported());
    assert_eq!(
        advice
            .selection
            .as_ref()
            .and_then(|selection| selection.first.as_ref())
            .expect("first artifact")
            .key,
        "performance_index_json"
    );
    assert_eq!(
        advice.selection_sidecar.as_ref().expect("sidecar").role,
        RustVerificationReportSidecarRole::SelectionAdvice
    );
    assert_eq!(
        advice.selection_sidecar.as_ref().expect("sidecar").path,
        selection_path
    );
    assert_eq!(
        advice.first_artifact.as_ref().expect("first artifact").key,
        "performance_index_json"
    );
    assert_eq!(
        advice.first_artifact.as_ref().expect("first artifact").path,
        source_dir.join("performance_index.json")
    );
    assert!(compact.contains("action=load_selection_advice_sidecar"));
    assert!(compact.contains("first=performance_index_json role=baseline_evidence"));
    assert!(compact.contains("sidecar: key=selection_advice_json role=selection_advice"));
    assert!(compact.contains("artifact: key=performance_index_json role=baseline_evidence"));
    assert!(compact.contains(&display_agent_path(
        source_dir.join("performance_index.json").as_path()
    )));
    assert!(compact.contains(&display_agent_path(&selection_path)));
    assert!(compact.contains(
        "|order: performance_index_json -> task_index_json -> verification_plan_json -> analysis_profile_json"
    ));
    assert_eq!(value["action"], "load_selection_advice_sidecar");
    assert_eq!(value["first_artifact"]["key"], "performance_index_json");
    assert_eq!(value["selection_sidecar"]["key"], "selection_advice_json");
}

fn display_agent_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[test]
fn report_entry_stops_before_selection_when_schema_is_unsupported() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let options = RustVerificationReportOptions::default().with_analysis_profile_artifact();
    let mut bundle = build_rust_verification_report_bundle_with_options(&plan, &options);
    bundle.schema.schema_version = "2".to_string();

    let advice = build_rust_verification_report_entry_advice(&bundle, None);
    let compact = render_rust_verification_report_entry_advice(&advice);
    let json = render_rust_verification_report_entry_advice_json(&advice).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(
        advice.action,
        RustVerificationReportEntryAction::StopAndRefreshHarnessContract
    );
    assert!(!advice.schema_compatibility.is_supported());
    assert!(advice.selection.is_none());
    assert!(advice.first_artifact.is_none());
    assert!(advice.selection_sidecar.is_none());
    assert_eq!(
        compact,
        "[verify-report-entry] state=unsupported_schema_version action=stop_and_refresh_harness_contract expected=1 actual=2 reason=\"unsupported manifest schema version\""
    );
    assert_eq!(
        value["schema_compatibility"]["state"],
        "unsupported_schema_version"
    );
    assert!(value.get("selection").is_none());
}

#[test]
fn report_entry_reports_empty_supported_bundles_without_payload_action() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan =
        plan_rust_project_verification_with_config(root, &default_asp_rust_config()).expect("plan");
    let bundle = build_rust_verification_report_bundle_with_options(
        &plan,
        &RustVerificationReportOptions::default(),
    );

    let advice = build_rust_verification_report_entry_advice(&bundle, None);
    let compact = render_rust_verification_report_entry_advice(&advice);

    assert_eq!(
        advice.action,
        RustVerificationReportEntryAction::NoActiveReportArtifacts
    );
    assert!(advice.schema_compatibility.is_supported());
    assert!(
        advice
            .selection
            .as_ref()
            .expect("selection")
            .first
            .is_none()
    );
    assert_eq!(
        compact,
        "[verify-report-entry] schema=rust_verification_report_manifest/1 state=supported action=no_active_report_artifacts first=<none> reason=no_active_report_artifacts"
    );
}
