use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use rust_lang_project_harness::{
    RustDiagnosticSeverity, RustHarnessFinding, RustHarnessReport, RustInvariantCandidate,
    RustInvariantCandidateStatus, RustInvariantEvidence, RustInvariantEvidenceKind,
    RustInvariantId, RustInvariantKind, RustInvariantReceiptKind, RustInvariantRulePackId,
    RustInvariantSourceRuleId, RustModuleReport, SourceLocation, render_rust_project_harness,
    render_rust_project_harness_failure_frontier, render_rust_project_harness_json,
};

#[test]
fn compact_text_render_matches_snapshot() {
    let rendered = render_rust_project_harness(&snapshot_report());

    assert_eq!(
        rendered,
        include_str!("snapshots/rust_project_harness_compact_text.snap")
    );
}

#[test]
fn json_render_matches_snapshot() {
    let rendered = render_rust_project_harness_json(&snapshot_report()).expect("render json");

    assert_eq!(
        rendered,
        include_str!("snapshots/rust_project_harness_json.snap").trim_end()
    );
}

#[test]
fn failure_frontier_render_points_to_deduplicated_hot_blocks() {
    let temp_root = PathBuf::from("/tmp/project");
    let first_path = temp_root.join("src/lib.rs");
    let second_path = temp_root.join("src/query.rs");
    let mut report = snapshot_report();
    report.invariant_candidates = vec![
        invariant_candidate(
            "candidate-1",
            "RUST-AGENT-PROJECT-003",
            first_path.clone(),
            20,
        ),
        invariant_candidate("candidate-2", "RUST-AGENT-PROJECT-004", first_path, 28),
        invariant_candidate("candidate-3", "RUST-MOD-R001", second_path, 3),
    ];

    let rendered = render_rust_project_harness_failure_frontier(&report, &temp_root, 4);

    assert!(rendered.contains(
        "[fail] rust blockingFindings=1 advisoryFindings=0 changedInvariants=3 hotBlocks=2"
    ));
    assert!(rendered.contains("|failureFrontier status=ready source=rust-check hotBlocks=2"));
    assert!(rendered.contains("directSourceReadCode<=2"));
    assert!(rendered.contains(
        "|hotBlock selector=src/lib.rs:8-32 source=invariant rule=RUST-AGENT-PROJECT-003 line=20"
    ));
    assert!(rendered.contains(
        "|hotBlock selector=src/query.rs:1-15 source=invariant rule=RUST-MOD-R001 line=3"
    ));
    assert!(!rendered.contains("RUST-AGENT-PROJECT-004"));
    assert!(rendered.contains(
        "query --from-hook direct-source-read --code --workspace . --selector 'src/lib.rs:8-32'"
    ));
}

fn snapshot_report() -> RustHarnessReport {
    let source_path = PathBuf::from("$TEMP/src/lib.rs");
    RustHarnessReport {
        modules: vec![RustModuleReport {
            path: source_path.clone(),
            is_valid: true,
            parse_error: None,
        }],
        findings: vec![RustHarnessFinding {
            rule_id: "RUST-AGENT-PROJECT-003".to_string(),
            pack_id: "rust.project_policy".to_string(),
            severity: RustDiagnosticSeverity::Warning,
            title: "Inline source test module".to_string(),
            summary: "$TEMP/src/lib.rs contains inline #[cfg(test)] module `tests`.".to_string(),
            location: SourceLocation::new(Some(source_path), 5, 4),
            requirement: "Keep test bodies out of src files; mount source-backed unit tests from tests/unit with #[path].".to_string(),
            source_line: Some("    #[cfg(test)] mod tests { #[test] fn it_works() {} }".to_string()),
            label: "move this test module to tests/unit and mount it with #[path]".to_string(),
            labels: BTreeMap::from([
                ("domain".to_string(), "project-policy".to_string()),
                ("language".to_string(), "rust".to_string()),
            ]),
        }],
        invariant_candidates: Vec::new(),
        root_paths: vec![PathBuf::from("$TEMP/src"), PathBuf::from("$TEMP/tests")],
        blocking_severities: BTreeSet::from([
            RustDiagnosticSeverity::Warning,
            RustDiagnosticSeverity::Error,
        ]),
        project_scope: None,
        workspace_member_scopes: Vec::new(),
    }
}

fn invariant_candidate(
    id: &str,
    rule_id: &str,
    path: PathBuf,
    line: usize,
) -> RustInvariantCandidate {
    RustInvariantCandidate {
        invariant_id: RustInvariantId(id.to_string()),
        source_rule_id: RustInvariantSourceRuleId(rule_id.to_string()),
        rule_pack_id: RustInvariantRulePackId("rust.project_policy".to_string()),
        kind: RustInvariantKind::ParserFact,
        status: RustInvariantCandidateStatus::Candidate,
        severity: RustDiagnosticSeverity::Warning,
        title: "Review source fact".to_string(),
        hypothesis: "the source fact needs review".to_string(),
        location: SourceLocation::new(Some(path), line, 0),
        evidence: vec![RustInvariantEvidence {
            kind: RustInvariantEvidenceKind::Finding,
            summary: "finding raised candidate".to_string(),
            location: None,
            fields: BTreeMap::new(),
        }],
        required_receipts: vec![RustInvariantReceiptKind::CargoCheck],
        proof_targets: Vec::new(),
        fields: BTreeMap::new(),
    }
}
