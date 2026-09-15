use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn tokio_runtime_boundary_without_owner_doc_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "tokio-runtime-boundary");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod worker;\n").expect("write lib");
    fs::write(
        root.join("src/worker.rs"),
        "//! Worker module.\n\
         pub async fn launch_background_task() {\n\
             tokio::spawn(async {});\n\
         }\n",
    )
    .expect("write worker");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-TOKIO-RUNTIME-002");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0].summary.contains("tokio::spawn"),
        "{:?}",
        report.findings
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn tokio_runtime_boundary_with_owner_doc_is_not_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "tokio-runtime-owner-doc");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod worker;\n").expect("write lib");
    fs::write(
        root.join("src/worker.rs"),
        "//! Worker module.\n\
         /// Runtime facade for tracked background tasks.\n\
         pub async fn launch_background_task() {\n\
             tokio::spawn(async {});\n\
         }\n",
    )
    .expect("write worker");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-TOKIO-RUNTIME-002");
    assert_eq!(findings.len(), 0, "{:?}", report.findings);
    assert!(report.is_clean(), "{:?}", report.findings);
}
