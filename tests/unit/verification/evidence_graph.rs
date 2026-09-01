use std::path::PathBuf;

use asp_rust::{
    RustEvidenceGraphAnalysisInput, RustEvidenceGraphInput, RustReviewPacket,
    build_rust_evidence_graph, build_rust_evidence_graph_analysis_request,
};
use serde_json::Value;

#[test]
fn p6_evidence_graph_links_review_packet_evidence() {
    let packet = serde_json::from_value::<RustReviewPacket>(review_packet_json())
        .expect("review packet fixture");
    let graph = build_rust_evidence_graph(RustEvidenceGraphInput {
        project_root: PathBuf::from("."),
        review_packets: vec![packet],
    });
    let value = serde_json::to_value(&graph).expect("evidence graph json");

    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-evidence-graph"
    );
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.evidence-graph"
    );
    assert_eq!(value["summary"]["owners"], 1);
    assert_eq!(value["summary"]["claims"], 1);
    assert_eq!(value["summary"]["staleItems"], 1);
    assert_eq!(value["summary"]["gaps"], 1);

    let node_kinds = value["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|node| node["kind"].as_str().expect("node kind"))
        .collect::<Vec<_>>();
    assert!(node_kinds.contains(&"review-packet"), "{value}");
    assert!(node_kinds.contains(&"owner"), "{value}");
    assert!(node_kinds.contains(&"invariant-candidate"), "{value}");
    assert!(node_kinds.contains(&"behavior-snapshot"), "{value}");
    assert!(node_kinds.contains(&"waiver"), "{value}");
    assert!(node_kinds.contains(&"review-action"), "{value}");

    let edge_kinds = value["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|edge| edge["kind"].as_str().expect("edge kind"))
        .collect::<Vec<_>>();
    assert!(edge_kinds.contains(&"derived-from"), "{value}");
    assert!(edge_kinds.contains(&"observed-by"), "{value}");
    assert!(edge_kinds.contains(&"waived-by"), "{value}");
    assert!(edge_kinds.contains(&"requires-evidence"), "{value}");
    assert!(edge_kinds.contains(&"suggests-action"), "{value}");
}

#[test]
fn p6_evidence_analysis_request_projects_graph_turbo_shape() {
    let packet = serde_json::from_value::<RustReviewPacket>(review_packet_json())
        .expect("review packet fixture");
    let graph = build_rust_evidence_graph(RustEvidenceGraphInput {
        project_root: PathBuf::from("."),
        review_packets: vec![packet],
    });
    let request = build_rust_evidence_graph_analysis_request(RustEvidenceGraphAnalysisInput {
        project_root: PathBuf::from("."),
        evidence_graphs: vec![graph],
    });
    let value = serde_json::to_value(&request).expect("analysis request json");

    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-graph-turbo-request"
    );
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.semantic-language"
    );
    assert_eq!(value["surface"], "evidence-analyze");
    assert_eq!(value["profile"], "rust-evidence-quality");
    assert_eq!(value["algorithm"], "typed-ppr-diverse");
    assert!(!value["seedIds"].as_array().expect("seed ids").is_empty());

    let graph = &value["graphs"][0];
    let invariant = graph["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["kind"] == "invariant-candidate")
        .expect("invariant node");
    assert!(invariant.get("id").is_some(), "{invariant}");
    assert!(invariant.get("role").is_some(), "{invariant}");
    assert!(invariant.get("value").is_some(), "{invariant}");
    assert!(invariant.get("nodeId").is_none(), "{invariant}");
    assert!(invariant.get("label").is_none(), "{invariant}");

    let edge = graph["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .find(|edge| edge["relation"] == "derived-from")
        .expect("graph-turbo edge");
    assert!(edge.get("source").is_some(), "{edge}");
    assert!(edge.get("target").is_some(), "{edge}");
    assert!(edge.get("fromNodeId").is_none(), "{edge}");
    assert!(edge.get("toNodeId").is_none(), "{edge}");
    assert!(edge.get("kind").is_none(), "{edge}");
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
