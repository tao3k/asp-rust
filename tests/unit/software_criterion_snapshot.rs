use std::fs;
use std::path::{Path, PathBuf};

use asp_rust::{AspRustFinding, render_asp_rust, run_asp_rust_for_scope};
use serde::Serialize;
use tempfile::TempDir;

const CONTROL_FLOW_SCENARIO: &str = "tests/unit/scenarios/software_criteria/control_flow_v1";
const LINEAR_MEMBERSHIP_SCENARIO: &str =
    "tests/unit/scenarios/software_criteria/data_structure_linear_membership_scan_v1";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FindingSnapshot {
    rule_id: String,
    summary: String,
    line: usize,
    label: String,
    software_criteria: Option<String>,
}

#[test]
fn agent_r015_control_flow_v1_snapshot() {
    insta::assert_snapshot!(
        "agent_r015_control_flow_v1",
        rule_snapshot("RUST-AGENT-CFG-PUBLIC-015", CONTROL_FLOW_SCENARIO)
    );
}

#[test]
fn agent_r016_control_flow_v1_snapshot() {
    insta::assert_snapshot!(
        "agent_r016_control_flow_v1",
        rule_snapshot("RUST-AGENT-CFG-PUBLIC-016", CONTROL_FLOW_SCENARIO)
    );
}

#[test]
fn agent_r017_control_flow_v1_snapshot() {
    insta::assert_snapshot!(
        "agent_r017_control_flow_v1",
        rule_snapshot("RUST-AGENT-ITER-PUBLIC-017", CONTROL_FLOW_SCENARIO)
    );
}

#[test]
fn agent_r025_control_flow_v1_snapshot() {
    insta::assert_snapshot!(
        "agent_r025_control_flow_v1",
        rule_snapshot("RUST-AGENT-CFG-IMPL-025", CONTROL_FLOW_SCENARIO)
    );
}

#[test]
fn agent_r026_control_flow_v1_snapshot() {
    insta::assert_snapshot!(
        "agent_r026_control_flow_v1",
        rule_snapshot("RUST-AGENT-ITER-IMPL-026", CONTROL_FLOW_SCENARIO)
    );
}

#[test]
fn agent_r029_data_structure_linear_membership_scan_v1_snapshot() {
    insta::assert_snapshot!(
        "agent_r029_data_structure_linear_membership_scan_v1",
        rule_snapshot("RUST-AGENT-DATA-MEMBERSHIP-029", LINEAR_MEMBERSHIP_SCENARIO)
    );
}

fn rule_snapshot(rule_id: &str, scenario: &str) -> String {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    copy_inputs(Path::new(scenario).join("inputs"), root);

    let mut report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");
    report.findings.retain(|finding| finding.rule_id == rule_id);
    let findings = report
        .findings
        .iter()
        .map(|finding| finding_snapshot(finding, root))
        .collect::<Vec<_>>();
    assert!(
        !findings.is_empty(),
        "expected {rule_id} finding in {scenario} scenario"
    );
    let rendered = normalize_temp_root(&render_asp_rust(&report), root);

    let findings_json =
        serde_json::to_string_pretty(&findings).expect("serialize findings snapshot");
    format!("{findings_json}\n\n--- rendered ---\n{rendered}")
}

fn finding_snapshot(finding: &AspRustFinding, root: &Path) -> FindingSnapshot {
    FindingSnapshot {
        rule_id: finding.rule_id.clone(),
        summary: normalize_temp_root(&finding.summary, root),
        line: finding.location.line,
        label: finding.label.clone(),
        software_criteria: finding.labels.get("softwareCriteria").cloned(),
    }
}

fn copy_inputs(source_dir: impl AsRef<Path>, destination_dir: &Path) {
    for source in walk_files(source_dir.as_ref()) {
        let destination = destination_dir.join(
            source
                .strip_prefix(source_dir.as_ref())
                .expect("relative input path"),
        );
        fs::create_dir_all(destination.parent().expect("input parent"))
            .expect("create input parent");
        fs::copy(&source, &destination).expect("copy scenario input");
    }
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .expect("read scenario inputs")
        .flat_map(|entry| {
            let path = entry.expect("scenario input entry").path();
            if path.is_dir() {
                walk_files(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
