use std::fs;
use std::path::Path;

use asp_rust::{
    RustOwnerResponsibility, RustVerificationAnalysisProfile, RustVerificationProfileHint,
    RustVerificationReportBundle, RustVerificationReportEntryAction,
    RustVerificationReportEntryAdvice, RustVerificationReportEntryArtifact,
    RustVerificationReportOptions, RustVerificationReportSelectionAdvice,
    RustVerificationReportWriteConfig, RustVerificationReportWriteReceipt,
    RustVerificationSkillBinding, RustVerificationTaskKind,
    build_rust_verification_analysis_profile_with_config,
    build_rust_verification_report_bundle_with_options,
    build_rust_verification_report_entry_advice_with_receipt,
    build_rust_verification_report_selection_advice, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_report_entry_advice,
    render_rust_verification_report_entry_advice_json,
    render_rust_verification_report_selection_advice,
    render_rust_verification_report_selection_advice_json,
    render_rust_verification_report_write_receipt,
    render_rust_verification_report_write_receipt_json,
    write_rust_verification_reports_with_options,
};
use tempfile::TempDir;

#[test]
fn report_projection_public_api_surface_smoke() {
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
    let profile: RustVerificationAnalysisProfile =
        build_rust_verification_analysis_profile_with_config(root, &config).expect("profile");
    let options = RustVerificationReportOptions::default()
        .with_analysis_profile_artifact()
        .with_selection_advice_sidecar();
    let bundle: RustVerificationReportBundle =
        build_rust_verification_report_bundle_with_options(&plan, &options);
    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");
    let receipt: RustVerificationReportWriteReceipt = write_rust_verification_reports_with_options(
        &plan,
        &config,
        &RustVerificationReportWriteConfig::new(root, &source_dir, &cache_dir),
        &options,
    )
    .expect("write reports");
    let selection: RustVerificationReportSelectionAdvice =
        build_rust_verification_report_selection_advice(&bundle, Some(&profile));
    let entry: RustVerificationReportEntryAdvice =
        build_rust_verification_report_entry_advice_with_receipt(
            &bundle,
            Some(&profile),
            Some(&receipt),
        );

    let entry_compact =
        normalize_temp_root(&render_rust_verification_report_entry_advice(&entry), root);
    let entry_json = render_rust_verification_report_entry_advice_json(&entry).expect("entry json");
    let selection_compact =
        render_rust_verification_report_selection_advice(&bundle, Some(&profile));
    let selection_json =
        render_rust_verification_report_selection_advice_json(&selection).expect("selection json");
    let receipt_compact = normalize_temp_root(
        &render_rust_verification_report_write_receipt(&receipt),
        root,
    );
    let receipt_json =
        render_rust_verification_report_write_receipt_json(&receipt).expect("receipt json");
    let persisted_manifest =
        fs::read_to_string(cache_dir.join("verification_report_manifest.json"))
            .expect("persisted runtime manifest");
    let downstream_bundle: RustVerificationReportBundle =
        serde_json::from_str(&persisted_manifest).expect("downstream manifest decode");
    let downstream_receipt: RustVerificationReportWriteReceipt =
        serde_json::from_str(&receipt_json).expect("downstream receipt decode");
    let downstream_entry = build_rust_verification_report_entry_advice_with_receipt(
        &downstream_bundle,
        Some(&profile),
        Some(&downstream_receipt),
    );
    let downstream_first: RustVerificationReportEntryArtifact = downstream_entry
        .first_artifact
        .clone()
        .expect("downstream first artifact");
    let downstream_payload =
        fs::read_to_string(&downstream_first.path).expect("downstream selected artifact");
    let downstream_payload_value: serde_json::Value =
        serde_json::from_str(&downstream_payload).expect("downstream selected artifact json");
    let entry_value: serde_json::Value =
        serde_json::from_str(&entry_json).expect("parse entry json");
    let selection_value: serde_json::Value =
        serde_json::from_str(&selection_json).expect("parse selection json");
    let receipt_value: serde_json::Value =
        serde_json::from_str(&receipt_json).expect("parse receipt json");

    assert_eq!(
        entry.action,
        RustVerificationReportEntryAction::LoadSelectionAdviceSidecar
    );
    assert_eq!(entry_value["action"], "load_selection_advice_sidecar");
    assert_eq!(selection_value["first"]["key"], "performance_index_json");
    assert_eq!(
        receipt_value["sidecar_paths"][0]["role"],
        "selection_advice"
    );
    let receipt_artifacts = receipt_value["artifact_paths"]
        .as_array()
        .expect("artifact paths");
    assert!(
        receipt_artifacts
            .iter()
            .any(|artifact| artifact["key"] == "performance_index_json")
    );
    assert_eq!(
        receipt.artifact_path("performance_index_json"),
        Some(&source_dir.join("performance_index.json"))
    );
    assert_eq!(
        receipt.sidecar_path("selection_advice_json"),
        Some(&cache_dir.join("selection_advice.json"))
    );
    assert_eq!(
        receipt.source_manifest_path(),
        Some(&source_dir.join("verification_report_manifest.json"))
    );
    assert_eq!(
        receipt.runtime_manifest_path(),
        Some(&cache_dir.join("verification_report_manifest.json"))
    );
    assert_eq!(
        entry.first_artifact.as_ref().expect("first artifact").path,
        source_dir.join("performance_index.json")
    );
    assert_eq!(downstream_entry.action, entry.action);
    assert_eq!(downstream_first.key, "performance_index_json");
    assert_eq!(
        downstream_payload_value["records"]
            .as_array()
            .expect("performance records")
            .len(),
        1
    );
    assert!(entry_compact.contains("first=performance_index_json role=baseline_evidence"));
    assert!(entry_compact.contains("artifact: key=performance_index_json role=baseline_evidence"));
    assert!(
        entry_compact.contains("path=$TEMP/.cache/agent/verification/sample/selection_advice.json")
    );
    assert!(
        entry_compact.contains("path=$TEMP/resources/verification/reports/performance_index.json")
    );
    assert!(selection_compact.contains("reason=load_baseline_evidence_for_active_tasks"));
    assert!(
        receipt_compact
            .contains("[verify-report-write] schema=rust_verification_report_manifest/1")
    );
    assert!(receipt_compact.contains("source_manifest: $TEMP/resources/verification/reports"));
    assert!(receipt_json.contains("selection_advice_path"));
}

fn write_api_project(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"report-projection-public-api\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\npub fn handle_request() {}\n",
    )
    .expect("write api");
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
