use asp_rust::{
    RustVerificationReportOptions, RustVerificationReportWriteConfig,
    build_rust_verification_analysis_profile_with_config,
    build_rust_verification_report_bundle_with_options,
    build_rust_verification_report_entry_advice_with_receipt,
    plan_rust_project_verification_with_config, render_rust_verification_report_entry_advice,
    render_rust_verification_report_selection_advice,
    render_rust_verification_report_write_receipt, write_rust_verification_reports_with_options,
};
use tempfile::TempDir;

use crate::verification::support::{
    latency_sensitive_performance_config, normalize_temp_root, write_api_project,
};

#[test]
fn verification_report_agent_projection_contract_snapshot() {
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
    let entry = build_rust_verification_report_entry_advice_with_receipt(
        &bundle,
        Some(&profile),
        Some(&receipt),
    );
    let rendered = format!(
        "[entry]\n{}\n\n[selection]\n{}\n\n[write-receipt]\n{}",
        render_rust_verification_report_entry_advice(&entry),
        render_rust_verification_report_selection_advice(&bundle, Some(&profile)),
        render_rust_verification_report_write_receipt(&receipt),
    );

    insta::assert_snapshot!(
        "verification_report_agent_projection_contract",
        normalize_temp_root(&rendered, root)
    );
}
