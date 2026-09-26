use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn private_nested_receipt_traversal_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "private-receipt-traversal");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod receipt;\n").expect("write lib");
    fs::write(root.join("src/receipt.rs"), receipt_traversal_source()).expect("write receipt");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-CFG-IMPL-025");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("control-flow.traversal-knot"));
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("control-flow.traversal-knot")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn named_receipt_traversal_helper_clears_private_nesting_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "named-receipt-traversal");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod receipt;\n").expect("write lib");
    fs::write(
        root.join("src/receipt.rs"),
        named_receipt_traversal_source(),
    )
    .expect("write receipt");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-CFG-IMPL-025").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn private_manual_iterator_boilerplate_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "private-iterator-boilerplate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod receipt;\n").expect("write lib");
    fs::write(root.join("src/receipt.rs"), manual_iterator_source()).expect("write receipt");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-ITER-IMPL-026");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("native-idiom.manual-transform-loop")
    );
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("native-idiom.manual-transform-loop")
    );
    assert!(findings_for_rule(&report, "RUST-AGENT-CFG-IMPL-025").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn named_private_iterator_helper_clears_boilerplate_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "named-private-iterator");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod receipt;\n").expect("write lib");
    fs::write(root.join("src/receipt.rs"), named_iterator_source()).expect("write receipt");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ITER-IMPL-026").is_empty());
    assert!(findings_for_rule(&report, "RUST-AGENT-CFG-IMPL-025").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

fn receipt_traversal_source() -> String {
    "//! Receipt helpers.\n\
     struct ContractReceipt {\n\
     \tsummary: ReceiptSummary,\n\
     \trepositories: Vec<RepositoryReceipt>,\n\
     }\n\
     struct ReceiptSummary { failed_query_count: usize }\n\
     struct RepositoryReceipt { query_receipts: Vec<QueryReceipt> }\n\
     struct QueryReceipt { passed: bool, name: String }\n\
     fn collect_failed_queries(receipt: &ContractReceipt) -> Vec<String> {\n\
     \tlet mut failed = Vec::new();\n\
     \tif receipt.summary.failed_query_count > 0 {\n\
     \t\tfor repository in &receipt.repositories {\n\
     \t\t\tfor query in &repository.query_receipts {\n\
     \t\t\t\tif !query.passed {\n\
     \t\t\t\t\tfailed.push(query.name.clone());\n\
     \t\t\t\t}\n\
     \t\t\t}\n\
     \t\t}\n\
     \t}\n\
     \tfailed\n\
     }\n"
    .to_string()
}

fn manual_iterator_source() -> String {
    "//! Receipt helpers.\n\
     struct QueryReceipt { passed: bool, name: String }\n\
     fn failed_query_names(queries: &[QueryReceipt]) -> Vec<String> {\n\
     \tlet mut names = Vec::new();\n\
     \tfor query in queries {\n\
     \t\tif !query.passed {\n\
     \t\t\tnames.push(query.name.clone());\n\
     \t\t}\n\
     \t}\n\
     \tnames\n\
     }\n"
    .to_string()
}

fn named_iterator_source() -> String {
    "//! Receipt helpers.\n\
     struct QueryReceipt { passed: bool, name: String }\n\
     fn failed_queries<'a>(queries: &'a [QueryReceipt]) -> impl Iterator<Item = &'a QueryReceipt> {\n\
     \tqueries.iter().filter(|query| !query.passed)\n\
     }\n\
     fn failed_query_names(queries: &[QueryReceipt]) -> Vec<String> {\n\
     \tfailed_queries(queries)\n\
     \t\t.map(|query| query.name.clone())\n\
     \t\t.collect()\n\
     }\n"
    .to_string()
}

fn named_receipt_traversal_source() -> String {
    "//! Receipt helpers.\n\
     struct ContractReceipt { repositories: Vec<RepositoryReceipt> }\n\
     struct RepositoryReceipt { query_receipts: Vec<QueryReceipt> }\n\
     struct QueryReceipt { passed: bool, name: String }\n\
     fn failed_queries<'a>(receipt: &'a ContractReceipt) -> impl Iterator<Item = &'a QueryReceipt> {\n\
     \treceipt\n\
     \t\t.repositories\n\
     \t\t.iter()\n\
     \t\t.flat_map(|repository| repository.query_receipts.iter())\n\
     \t\t.filter(|query| !query.passed)\n\
     }\n\
     fn collect_failed_queries(receipt: &ContractReceipt) -> Vec<String> {\n\
     \tfailed_queries(receipt)\n\
     \t\t.map(|query| query.name.clone())\n\
     \t\t.collect()\n\
     }\n"
    .to_string()
}
