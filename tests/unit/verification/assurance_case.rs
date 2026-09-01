use std::path::PathBuf;

use asp_rust::{
    RustAssuranceCaseInput, RustEvidenceGraphInput, RustReviewPacket,
    build_rust_assurance_case_set, build_rust_evidence_graph,
};
use serde_json::Value;

#[test]
fn p6_2_assurance_case_summarizes_claim_support_and_gaps() {
    let packet = serde_json::from_value::<RustReviewPacket>(review_packet_json())
        .expect("review packet fixture");
    let graph = build_rust_evidence_graph(RustEvidenceGraphInput {
        project_root: PathBuf::from("."),
        review_packets: vec![packet],
    });
    let case_set = build_rust_assurance_case_set(RustAssuranceCaseInput {
        project_root: PathBuf::from("."),
        evidence_graphs: vec![graph],
    });
    let value = serde_json::to_value(&case_set).expect("assurance case json");

    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-assurance-case"
    );
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.assurance-case"
    );
    assert_eq!(value["summary"]["cases"], 2);
    assert_eq!(value["summary"]["claims"], 2);
    assert_eq!(value["summary"]["supportedClaims"], 1);
    assert_eq!(value["summary"]["openGaps"], 1);
    assert_eq!(value["summary"]["staleItems"], 1);

    let cases = value["cases"].as_array().expect("cases");
    let invariant_case = cases
        .iter()
        .find(|case| case["claim"]["kind"] == "invariant")
        .expect("invariant case");
    assert_eq!(invariant_case["status"], "needs-review");
    assert_eq!(
        invariant_case["claim"]["claimId"],
        "claim:agent-r027:src.model.rs:42"
    );
    assert_eq!(
        invariant_case["ownerPath"],
        serde_json::json!("src/model.rs")
    );
    assert_eq!(
        invariant_case["observedBy"]
            .as_array()
            .expect("observedBy")
            .len(),
        1
    );
    assert_eq!(
        invariant_case["waivedBy"]
            .as_array()
            .expect("waivedBy")
            .len(),
        1
    );
    assert_eq!(
        invariant_case["actions"].as_array().expect("actions").len(),
        1
    );
    assert_eq!(invariant_case["gaps"].as_array().expect("gaps").len(), 1);

    let proof_case = cases
        .iter()
        .find(|case| case["claim"]["kind"] == "proof")
        .expect("proof case");
    assert_eq!(proof_case["status"], "supported");
    assert_eq!(
        proof_case["supportedBy"]
            .as_array()
            .expect("supportedBy")
            .len(),
        1
    );
}

fn review_packet_json() -> Value {
    serde_json::json!({
        "schemaId": "agent.semantic-protocols.semantic-review-packet",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.review-packet",
        "protocolVersion": "1",
        "packetId": "rust.review.packet",
        "producer": {
            "languageId": "rust",
            "providerId": "asp-rust",
            "namespace": "agent.semantic-protocols.languages.rust.asp-rust"
        },
        "project": {"root": "."},
        "summary": {
            "changedInvariants": 1,
            "changedBehavior": 1,
            "missingReceipts": 1,
            "staleWaivers": 1,
            "determinismObservations": 2,
            "proofClaims": 1
        },
        "changedInvariants": [
            {
                "invariantId": "agent-r027:src.model.rs:42",
                "sourceRuleId": "RUST-AGENT-API-TYPE-ALIAS-027",
                "kind": "public-data-primitive-fields",
                "severity": "warning",
                "title": "semantic fields need named type",
                "hypothesis": "public data shape should not expose stringly fields",
                "location": {"path": "src/model.rs", "line": 42, "column": 0},
                "requiredReceipts": ["cargo-check", "expect-test"]
            }
        ],
        "changedBehavior": [
            {
                "snapshotId": "rust.behavior.src-model",
                "status": "changed",
                "subject": "src/model.rs",
                "summary": "expect-test output changed",
                "receiptIds": ["rust.expect-test.src-model"],
                "candidateIds": ["agent-r027:src.model.rs:42"]
            }
        ],
        "missingReceipts": [
            {
                "invariantId": "agent-r027:src.model.rs:42",
                "receiptKind": "expect-test",
                "reason": "no passed expect-test receipt linked to candidate"
            }
        ],
        "staleWaivers": [
            {
                "waiverId": "waiver.agent-r027.src-model",
                "invariantId": "agent-r027:src.model.rs:42",
                "receiptKind": "expect-test",
                "status": "stale",
                "owner": "reviewer",
                "reason": "snapshot migration is pending",
                "expiresAt": "2026-05-01"
            }
        ],
        "determinismReadiness": [
            {
                "readinessId": "rust.determinism-readiness.project",
                "status": "needs-injection",
                "observations": 2,
                "suggestions": 2
            }
        ],
        "proofPilots": [
            {
                "proofId": "rust.proof.dependency-graph-acyclicity",
                "target": "owner dependency graph cycle detection",
                "status": "proved-bounded",
                "claims": 1,
                "checks": 1
            }
        ],
        "reviewActions": [
            {
                "actionId": "run-receipt.agent-r027.src-model.expect-test",
                "kind": "run-receipt",
                "priority": "p0",
                "summary": "Run expect-test for agent-r027:src.model.rs:42",
                "targetId": "agent-r027:src.model.rs:42"
            }
        ]
    })
}
