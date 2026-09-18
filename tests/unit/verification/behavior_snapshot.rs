use std::path::PathBuf;

use asp_rust::{
    RustBehaviorSnapshot, RustBehaviorSnapshotExpectTestInput, RustBehaviorSnapshotId,
    RustBehaviorSnapshotStatus, RustBehaviorSnapshotSymbol, RustBehaviorSnapshotValue,
    RustInvariantId,
};
use serde_json::json;

#[test]
fn p2_behavior_snapshot_serializes_shared_schema_surface() {
    let mut snapshot =
        RustBehaviorSnapshot::matched_expect_test(RustBehaviorSnapshotExpectTestInput {
            snapshot_id: RustBehaviorSnapshotId("rust.expect-test.public-api-shape".to_string()),
            subject_path: PathBuf::from("src/lib.rs"),
            symbol: Some(RustBehaviorSnapshotSymbol(
                "parse_public_api_shape".to_string(),
            )),
            expected: RustBehaviorSnapshotValue::text("pub fn parse_public_api_shape(...)"),
            actual: RustBehaviorSnapshotValue::text("pub fn parse_public_api_shape(...)"),
        });
    snapshot
        .receipt_ids
        .push("rust.expect-test:expect-test:passed".to_string());
    snapshot
        .candidate_ids
        .push(RustInvariantId("agent-r027:src/lib.rs:1".to_string()));
    let value = serde_json::to_value(&snapshot).expect("snapshot json");
    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-behavior-snapshot"
    );
    assert_eq!(value["schemaVersion"], "1");
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.behavior-snapshot"
    );
    assert_eq!(value["status"], "matched");
    assert_eq!(snapshot.status, RustBehaviorSnapshotStatus::Matched);
    assert_eq!(value["subject"]["kind"], "public-api");
    assert_eq!(value["subject"]["path"], "src/lib.rs");
    assert_eq!(value["observations"][0]["kind"], "snapshot");
    assert_eq!(
        value["expected"],
        json!({
            "format": "text",
            "value": "pub fn parse_public_api_shape(...)"
        })
    );
    assert_eq!(
        value["receiptIds"],
        json!(["rust.expect-test:expect-test:passed"])
    );
    assert_eq!(value["candidateIds"], json!(["agent-r027:src/lib.rs:1"]));
}
