use std::path::{Path, PathBuf};

use asp_rust::{
    RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID, RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION,
    RustVerificationReportArtifactRole, RustVerificationReportOptions,
    RustVerificationReportPersistence, RustVerificationReportSidecarRole,
    RustVerificationReportWriteConfig, plan_rust_project_verification_with_config,
    render_rust_verification_report_write_receipt,
    render_rust_verification_report_write_receipt_json, write_rust_verification_reports,
    write_rust_verification_reports_with_options,
};
use tempfile::TempDir;

use crate::verification::support::{latency_sensitive_performance_config, write_api_project};

#[test]
fn verification_report_writer_splits_source_baseline_from_runtime_cache() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");

    let receipt = write_rust_verification_reports(
        &plan,
        &RustVerificationReportWriteConfig::new(root, &source_dir, &cache_dir),
    )
    .expect("write reports");

    assert!(
        source_dir
            .join("verification_report_manifest.json")
            .exists()
    );
    assert!(source_dir.join("performance_index.json").exists());
    assert!(source_dir.join("task_index.json").exists());
    assert!(!source_dir.join("verification_plan.json").exists());
    assert!(cache_dir.join("verification_report_manifest.json").exists());
    assert!(cache_dir.join("verification_plan.json").exists());
    assert_eq!(
        receipt.manifest_schema.schema_id,
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID
    );
    assert_eq!(
        receipt.manifest_schema.schema_version,
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(receipt.source_baseline_paths.len(), 3);
    assert_eq!(receipt.runtime_cache_paths.len(), 2);
    assert_eq!(receipt.artifact_paths.len(), 3);
    assert!(receipt.materialization_advice.is_empty());
    assert_eq!(
        receipt.artifact_path("performance_index_json"),
        Some(&source_dir.join("performance_index.json"))
    );
    assert_eq!(
        receipt.artifact_path("verification_plan_json"),
        Some(&cache_dir.join("verification_plan.json"))
    );
    assert!(receipt.sidecar_paths.is_empty());

    let source_manifest =
        std::fs::read_to_string(source_dir.join("verification_report_manifest.json"))
            .expect("source manifest");
    let cache_manifest =
        std::fs::read_to_string(cache_dir.join("verification_report_manifest.json"))
            .expect("cache manifest");
    let performance_index = std::fs::read_to_string(source_dir.join("performance_index.json"))
        .expect("performance index");
    let source_manifest_value: serde_json::Value =
        serde_json::from_str(&source_manifest).expect("parse source manifest");
    let cache_manifest_value: serde_json::Value =
        serde_json::from_str(&cache_manifest).expect("parse cache manifest");

    assert_eq!(
        source_manifest_value["schema"]["schema_id"],
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID
    );
    assert_eq!(
        cache_manifest_value["schema"]["schema_version"],
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION
    );
    assert!(source_manifest.contains("performance_index_json"));
    assert!(source_manifest.contains("task_index_json"));
    assert!(!source_manifest.contains("selection_advice_json"));
    assert!(!source_manifest.contains("verification_plan_json"));
    assert!(cache_manifest.contains("verification_plan_json"));
    assert!(cache_manifest.contains("task_index_json"));
    assert!(cache_manifest.contains("performance_index_json"));
    assert!(!cache_manifest.contains("selection_advice_json"));
    assert!(performance_index.contains("$CRATE_ROOT"));
    assert!(!performance_index.contains(&root.display().to_string()));
}

#[test]
fn verification_report_writer_advises_when_source_baseline_dir_is_temp_outside_repo() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path().join("project");
    std::fs::create_dir(&root).expect("create project");
    write_api_project(&root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(&root, &config).expect("plan");
    let source_dir = temp.path().join("tmp-verification-reports");
    let cache_dir = root.join(".cache/agent/verification/sample");

    let receipt = write_rust_verification_reports(
        &plan,
        &RustVerificationReportWriteConfig::new(&root, &source_dir, &cache_dir),
    )
    .expect("write reports");

    let advice = receipt
        .materialization_advice
        .first()
        .expect("materialization advice");
    let compact = render_rust_verification_report_write_receipt(&receipt);
    let compact_snapshot = redact_agent_path(
        &redact_agent_path(&compact, &root, "$CRATE_ROOT"),
        temp.path(),
        "$TEMP",
    );
    let json = render_rust_verification_report_write_receipt_json(&receipt).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(
        advice.persistence,
        RustVerificationReportPersistence::SourceBaseline
    );
    assert_eq!(advice.path, source_dir);
    assert_eq!(
        advice.recommended_dir,
        root.join("resources/verification/reports")
    );
    assert_eq!(advice.reason, "source_baseline_dir_outside_project_root");
    assert_eq!(advice.action, "move_source_baseline_reports_to_repo");
    assert_eq!(
        advice.artifact_keys,
        ["task_index_json", "performance_index_json"]
    );
    assert!(compact.contains("materialize: source_baseline"));
    assert!(compact.contains("reason=source_baseline_dir_outside_project_root"));
    assert!(compact.contains("action=move_source_baseline_reports_to_repo"));
    assert!(compact.contains("artifacts=task_index_json,performance_index_json"));
    insta::assert_snapshot!(
        "verification_report_writer_materialization_advice",
        compact_snapshot
    );
    assert_eq!(
        value["materialization_advice"][0]["recommended_dir"],
        root.join("resources/verification/reports")
            .display()
            .to_string()
    );
}

#[test]
fn verification_report_writer_advises_when_source_baseline_dir_is_runtime_cache() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let cache_dir = root.join(".cache/agent/verification/sample");
    let source_dir = cache_dir.join("baselines");

    let receipt = write_rust_verification_reports(
        &plan,
        &RustVerificationReportWriteConfig::new(root, &source_dir, &cache_dir),
    )
    .expect("write reports");

    let advice = receipt
        .materialization_advice
        .first()
        .expect("materialization advice");
    assert_eq!(advice.reason, "source_baseline_dir_under_runtime_cache");
    assert_eq!(
        advice.recommended_dir,
        root.join("resources/verification/reports")
    );
}

#[test]
fn verification_report_writer_can_persist_explicit_analysis_profile_runtime_artifact() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");
    let options = RustVerificationReportOptions::default().with_analysis_profile_artifact();

    let receipt = write_rust_verification_reports_with_options(
        &plan,
        &config,
        &RustVerificationReportWriteConfig::new(root, &source_dir, &cache_dir),
        &options,
    )
    .expect("write reports");

    assert!(!source_dir.join("analysis_profile.json").exists());
    assert!(cache_dir.join("analysis_profile.json").exists());
    assert!(!cache_dir.join("selection_advice.json").exists());
    assert_eq!(receipt.source_baseline_paths.len(), 3);
    assert_eq!(receipt.runtime_cache_paths.len(), 3);
    assert_eq!(receipt.artifact_paths.len(), 4);
    assert_eq!(
        receipt.artifact_path("analysis_profile_json"),
        Some(&cache_dir.join("analysis_profile.json"))
    );
    assert!(receipt.selection_advice_path.is_none());
    assert!(receipt.sidecar_paths.is_empty());

    let source_manifest =
        std::fs::read_to_string(source_dir.join("verification_report_manifest.json"))
            .expect("source manifest");
    let cache_manifest =
        std::fs::read_to_string(cache_dir.join("verification_report_manifest.json"))
            .expect("cache manifest");
    let analysis_profile =
        std::fs::read_to_string(cache_dir.join("analysis_profile.json")).expect("analysis profile");

    assert!(!source_manifest.contains("analysis_profile_json"));
    assert!(cache_manifest.contains("analysis_profile_json"));
    assert!(analysis_profile.contains("$CRATE_ROOT"));
    assert!(!analysis_profile.contains(&root.display().to_string()));
}

#[test]
fn verification_report_writer_can_persist_selection_advice_runtime_sidecar() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");
    let options = RustVerificationReportOptions::default()
        .with_analysis_profile_artifact()
        .with_selection_advice_sidecar();

    let receipt = write_rust_verification_reports_with_options(
        &plan,
        &config,
        &RustVerificationReportWriteConfig::new(root, &source_dir, &cache_dir),
        &options,
    )
    .expect("write reports");

    let selection_path = cache_dir.join("selection_advice.json");
    assert!(!source_dir.join("selection_advice.json").exists());
    assert!(selection_path.exists());
    assert_eq!(
        receipt.selection_advice_path.as_ref(),
        Some(&selection_path)
    );
    assert_eq!(receipt.sidecar_paths.len(), 1);
    assert_eq!(receipt.sidecar_paths[0].key, "selection_advice_json");
    assert_eq!(
        receipt.sidecar_paths[0].role,
        RustVerificationReportSidecarRole::SelectionAdvice
    );
    assert_eq!(receipt.sidecar_paths[0].path, selection_path);
    assert_eq!(receipt.source_baseline_paths.len(), 3);
    assert_eq!(receipt.runtime_cache_paths.len(), 4);
    assert_eq!(receipt.artifact_paths.len(), 4);
    assert_eq!(
        receipt
            .artifact_paths
            .iter()
            .find(|artifact| artifact.key == "performance_index_json")
            .expect("performance artifact")
            .role,
        RustVerificationReportArtifactRole::BaselineEvidence
    );
    assert_eq!(
        receipt
            .artifact_paths
            .iter()
            .find(|artifact| artifact.key == "performance_index_json")
            .expect("performance artifact")
            .persistence,
        RustVerificationReportPersistence::SourceBaseline
    );
    assert_eq!(
        receipt.artifact_path("performance_index_json"),
        Some(&source_dir.join("performance_index.json"))
    );
    assert_eq!(
        receipt.sidecar_path("selection_advice_json"),
        Some(&selection_path)
    );

    let source_manifest =
        std::fs::read_to_string(source_dir.join("verification_report_manifest.json"))
            .expect("source manifest");
    let cache_manifest =
        std::fs::read_to_string(cache_dir.join("verification_report_manifest.json"))
            .expect("cache manifest");
    let cache_manifest_value: serde_json::Value =
        serde_json::from_str(&cache_manifest).expect("parse cache manifest");
    let selection_advice =
        std::fs::read_to_string(selection_path).expect("selection advice sidecar");
    let value: serde_json::Value =
        serde_json::from_str(&selection_advice).expect("parse selection advice");

    assert!(!source_manifest.contains("selection_advice_json"));
    assert_eq!(
        cache_manifest_value["schema"]["schema_id"],
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID
    );
    assert_eq!(
        cache_manifest_value["sidecars"][0]["key"],
        "selection_advice_json"
    );
    assert_eq!(
        cache_manifest_value["sidecars"][0]["artifact_name"],
        "selection_advice.json"
    );
    assert_eq!(
        cache_manifest_value["sidecars"][0]["role"],
        "selection_advice"
    );
    assert_eq!(value["reason"], "load_baseline_evidence_for_active_tasks");
    assert_eq!(value["first"]["key"], "performance_index_json");
    assert_eq!(value["scale"]["package_count"], 1);
    assert_eq!(value["order"][0]["key"], "performance_index_json");
    assert_eq!(value["order"][3]["key"], "analysis_profile_json");
}

#[test]
fn verification_report_write_receipt_renders_agent_paths() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");
    let options = RustVerificationReportOptions::default()
        .with_analysis_profile_artifact()
        .with_selection_advice_sidecar();

    let receipt = write_rust_verification_reports_with_options(
        &plan,
        &config,
        &RustVerificationReportWriteConfig::new(root, &source_dir, &cache_dir),
        &options,
    )
    .expect("write reports");

    let compact = render_rust_verification_report_write_receipt(&receipt);
    let json = render_rust_verification_report_write_receipt_json(&receipt).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let decoded: asp_rust::RustVerificationReportWriteReceipt =
        serde_json::from_str(&json).expect("decode receipt");

    assert_eq!(decoded, receipt);
    assert_eq!(
        receipt.source_manifest_path(),
        Some(&source_dir.join("verification_report_manifest.json"))
    );
    assert_eq!(
        receipt.runtime_manifest_path(),
        Some(&cache_dir.join("verification_report_manifest.json"))
    );
    assert!(compact.starts_with(
        "[verify-report-write] schema=rust_verification_report_manifest/1 source_paths=3 runtime_paths=4 sidecars=1"
    ));
    assert!(compact.contains(&format!(
        "|source_manifest: {}",
        display_agent_path(source_dir.join("verification_report_manifest.json").as_path())
    )));
    assert!(compact.contains(&format!(
        "|runtime_manifest: {}",
        display_agent_path(cache_dir.join("verification_report_manifest.json").as_path())
    )));
    assert!(compact.contains(&format!(
        "|selection_advice: {}",
        display_agent_path(cache_dir.join("selection_advice.json").as_path())
    )));
    assert!(compact.contains("sidecar: role=selection_advice key=selection_advice_json"));
    assert_eq!(
        value["manifest_schema"]["schema_id"],
        RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID
    );
    assert_eq!(
        value["source_baseline_paths"]
            .as_array()
            .expect("paths")
            .len(),
        3
    );
    assert_eq!(
        value["runtime_cache_paths"]
            .as_array()
            .expect("paths")
            .len(),
        4
    );
    let artifact_paths = value["artifact_paths"].as_array().expect("artifact paths");
    let performance_artifact = artifact_paths
        .iter()
        .find(|artifact| artifact["key"] == "performance_index_json")
        .expect("performance artifact");
    assert_eq!(performance_artifact["role"], "baseline_evidence");
    assert_eq!(performance_artifact["persistence"], "source_baseline");
    assert_eq!(value["sidecar_paths"][0]["role"], "selection_advice");
}

fn display_agent_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn redact_agent_path(rendered: &str, path: &Path, replacement: &str) -> String {
    let path_text = path.display().to_string();
    rendered
        .replace(&path_text, replacement)
        .replace(&path_text.replace('\\', "/"), replacement)
}

#[test]
fn verification_report_writer_compacts_windows_json_escaped_project_root() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = latency_sensitive_performance_config();
    let mut plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let windows_root = PathBuf::from(r"C:\agent\work\verification-api");
    let windows_api = PathBuf::from(r"C:\agent\work\verification-api\src\api.rs");
    plan.project_root = windows_root.clone();
    for task in &mut plan.tasks {
        task.package_root = windows_root.clone();
        task.owner_path = windows_api.clone();
    }

    let source_dir = root.join("resources/verification/reports");
    let cache_dir = root.join(".cache/agent/verification/sample");
    write_rust_verification_reports(
        &plan,
        &RustVerificationReportWriteConfig::new(&windows_root, &source_dir, &cache_dir),
    )
    .expect("write reports");

    let performance_index = std::fs::read_to_string(source_dir.join("performance_index.json"))
        .expect("performance index");

    assert!(performance_index.contains("$CRATE_ROOT"));
    assert!(!performance_index.contains(r"C:\agent\work"));
    assert!(!performance_index.contains(r"C:\\agent\\work"));
}
