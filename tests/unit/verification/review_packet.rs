use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use rust_lang_project_harness::{
    RustDiagnosticSeverity, RustHarnessReport, RustInvariantCandidate,
    RustInvariantCandidateStatus, RustInvariantEvidence, RustInvariantEvidenceKind,
    RustInvariantId, RustInvariantKind, RustInvariantReceiptKind, RustInvariantRulePackId,
    RustInvariantSourceRuleId, RustReviewPacketInput, RustReviewPacketReceiptKind,
    RustReviewPacketWaiver, RustReviewPacketWaiverStatus, RustVerificationExecutionExitCode,
    RustVerificationExecutionReceipt, RustVerificationExecutionReceiptId,
    RustVerificationExecutionSummary, RustVerificationToolAdapter, SourceLocation,
    build_rust_review_packet,
};

#[test]
fn p5_review_packet_summarizes_invariants_receipts_and_stale_waivers() {
    let candidate_id = RustInvariantId("agent-r027:src.model.rs:42".to_owned());
    let report = RustHarnessReport {
        modules: Vec::new(),
        findings: Vec::new(),
        invariant_candidates: vec![RustInvariantCandidate {
            invariant_id: candidate_id.clone(),
            source_rule_id: RustInvariantSourceRuleId("RUST-AGENT-API-TYPE-ALIAS-027".to_owned()),
            rule_pack_id: RustInvariantRulePackId("agent".to_owned()),
            kind: RustInvariantKind::PublicDataPrimitiveFields,
            status: RustInvariantCandidateStatus::Candidate,
            severity: RustDiagnosticSeverity::Warning,
            title: "public data primitive fields".to_owned(),
            hypothesis: "public data shape should expose named semantic fields".to_owned(),
            location: SourceLocation {
                path: Some(PathBuf::from("src/model.rs")),
                line: 42,
                column: 0,
            },
            evidence: vec![RustInvariantEvidence {
                kind: RustInvariantEvidenceKind::Finding,
                summary: "RUST-AGENT-API-TYPE-ALIAS-027 matched public data field".to_owned(),
                location: None,
                fields: BTreeMap::new(),
            }],
            required_receipts: vec![
                RustInvariantReceiptKind::CargoCheck,
                RustInvariantReceiptKind::ExpectTest,
            ],
            proof_targets: Vec::new(),
            fields: BTreeMap::new(),
        }],
        root_paths: vec![PathBuf::from(".")],
        blocking_severities: BTreeSet::new(),
        project_resolution: None,
        workspace_member_scopes: Vec::new(),
    };
    let mut receipt = RustVerificationExecutionReceipt::from_exit_code(
        RustVerificationExecutionReceiptId("rust.cargo-check.src-model".to_owned()),
        RustVerificationToolAdapter::CargoCheck,
        RustVerificationExecutionExitCode(0),
        RustVerificationExecutionSummary("cargo check completed".to_owned()),
    );
    receipt.candidate_ids.push(candidate_id.clone());
    let stale_waiver = RustReviewPacketWaiver {
        waiver_id: "waiver.agent-r027.src-model.expect-test".to_owned(),
        invariant_id: candidate_id.0.clone(),
        receipt_kind: RustReviewPacketReceiptKind::ExpectTest,
        status: RustReviewPacketWaiverStatus::Stale,
        owner: "reviewer".to_owned(),
        reason: "snapshot migration pending".to_owned(),
        expires_at: Some("2026-05-01".to_owned()),
        fields: BTreeMap::new(),
    };

    let packet = build_rust_review_packet(RustReviewPacketInput {
        project_root: PathBuf::from("."),
        report,
        receipts: vec![receipt],
        behavior_snapshots: Vec::new(),
        determinism_readiness: Vec::new(),
        proof_pilots: Vec::new(),
        waivers: vec![stale_waiver],
    });

    let value = serde_json::to_value(&packet).expect("review packet json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-review-packet"
    );
    assert_eq!(value["summary"]["changedInvariants"], 1);
    assert_eq!(value["summary"]["missingReceipts"], 1);
    assert_eq!(value["summary"]["staleWaivers"], 1);
    assert_eq!(value["missingReceipts"][0]["receiptKind"], "expect-test");
    assert!(
        value["reviewActions"]
            .as_array()
            .expect("review actions")
            .iter()
            .any(
                |action| action["actionId"] == "run-receipt.agent-r027-src-model-rs-42.expect-test"
            )
    );
}
