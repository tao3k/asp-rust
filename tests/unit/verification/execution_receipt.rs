use asp_rust::{
    RustInvariantId, RustVerificationExecutionExitCode, RustVerificationExecutionReceipt,
    RustVerificationExecutionReceiptId, RustVerificationExecutionStatus,
    RustVerificationExecutionSummary, RustVerificationExecutionTaskFingerprint,
    RustVerificationToolAdapter,
};
use serde_json::json;
use std::str::FromStr;

#[test]
fn p1_tool_adapters_expose_default_commands() {
    assert_eq!(
        RustVerificationToolAdapter::CargoCheck.default_argv(),
        vec!["cargo", "check", "--message-format=json"]
    );
    assert_eq!(
        RustVerificationToolAdapter::CargoTest.default_argv(),
        vec!["cargo", "test", "--no-fail-fast"]
    );
    assert_eq!(
        RustVerificationToolAdapter::Clippy.default_argv(),
        vec!["cargo", "clippy", "--message-format=json"]
    );
    assert_eq!(
        RustVerificationToolAdapter::ExpectTest.default_argv(),
        vec!["cargo", "test"]
    );
    assert_eq!(
        RustVerificationToolAdapter::Proptest.default_argv(),
        vec!["cargo", "test", "--all-targets", "--", "--nocapture"]
    );
    assert_eq!(
        RustVerificationToolAdapter::CargoFuzz.default_argv(),
        vec!["cargo", "fuzz", "run"]
    );
    assert_eq!(
        RustVerificationToolAdapter::Kani.default_argv(),
        vec!["cargo", "kani"]
    );
    assert_eq!(
        RustVerificationToolAdapter::Creusot.default_argv(),
        vec!["cargo", "creusot"]
    );
    assert_eq!(
        RustVerificationToolAdapter::Verus.default_argv(),
        vec!["verus"]
    );
}

#[test]
fn p1_tool_adapters_parse_protocol_names() {
    assert_eq!(
        RustVerificationToolAdapter::from_str("cargo-check"),
        Ok(RustVerificationToolAdapter::CargoCheck)
    );
    assert_eq!(
        RustVerificationToolAdapter::from_str("expect-test"),
        Ok(RustVerificationToolAdapter::ExpectTest)
    );
    assert!(RustVerificationToolAdapter::from_str("cargo build").is_err());
}

#[test]
fn p1_execution_receipt_serializes_shared_schema_surface() {
    let mut receipt = RustVerificationExecutionReceipt::from_exit_code(
        RustVerificationExecutionReceiptId("rust.cargo-check.src-model".to_owned()),
        RustVerificationToolAdapter::CargoCheck,
        RustVerificationExecutionExitCode(0),
        RustVerificationExecutionSummary("cargo check completed".to_owned()),
    );
    receipt
        .candidate_ids
        .push(RustInvariantId("agent-r027:src.model.rs:42".to_owned()));
    receipt
        .task_fingerprints
        .push(RustVerificationExecutionTaskFingerprint(
            "regression:src/model.rs".to_owned(),
        ));

    let value = serde_json::to_value(&receipt).expect("receipt json");

    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-verification-receipt"
    );
    assert_eq!(value["schemaVersion"], "1");
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.verification-receipt"
    );
    assert_eq!(value["tool"], "cargo-check");
    assert_eq!(value["status"], "passed");
    assert_eq!(receipt.status, RustVerificationExecutionStatus::Passed);
    assert_eq!(value["producer"]["adapterId"], "rust.cargo-check");
    assert_eq!(
        value["command"],
        json!({
            "argv": ["cargo", "check", "--message-format=json"],
            "outputFormat": "cargo-json"
        })
    );
    assert_eq!(value["exitCode"], 0);
    assert_eq!(value["candidateIds"], json!(["agent-r027:src.model.rs:42"]));
    assert_eq!(
        value["taskFingerprints"],
        json!(["regression:src/model.rs"])
    );
    assert_eq!(value["observations"][0]["kind"], "exit-status");
}
