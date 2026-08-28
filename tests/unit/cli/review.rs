use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

use super::support::run_cli;

#[test]
fn cli_review_packet_renders_json_contract() {
    let temp = TempDir::new().expect("temp dir");
    let output = run_cli([
        "review".as_ref(),
        "packet".as_ref(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<Value>(&stdout).expect("review packet json");

    assert_eq!(
        value["schemaId"],
        "agent.semantic-protocols.semantic-review-packet"
    );
    assert_eq!(
        value["protocolId"],
        "agent.semantic-protocols.review-packet"
    );
    assert_eq!(value["summary"]["changedInvariants"], 0);
    assert_eq!(value["reviewActions"], serde_json::json!([]));
}

#[test]
fn cli_review_packet_consumes_new_evidence_api_json_inputs() {
    let temp = TempDir::new().expect("temp dir");
    write_invariant_fixture(temp.path());

    let initial = review_packet_json(temp.path(), &[]);
    let missing_before = initial["summary"]["missingReceipts"]
        .as_u64()
        .expect("missing before");
    assert!(missing_before > 0, "{initial}");
    let cargo_check_missing = initial["missingReceipts"]
        .as_array()
        .expect("missing receipts")
        .iter()
        .find(|receipt| receipt["receiptKind"] == "cargo-check")
        .expect("cargo-check missing receipt");
    let candidate_id = cargo_check_missing["invariantId"]
        .as_str()
        .expect("candidate id")
        .to_owned();

    let receipt_path = temp.path().join("receipt.json");
    let behavior_path = temp.path().join("behavior.json");
    let determinism_path = temp.path().join("determinism.json");
    let proof_path = temp.path().join("proof.json");
    let waiver_path = temp.path().join("waiver.json");
    fs::write(
        &receipt_path,
        serde_json::to_string(&cargo_check_receipt_json(&candidate_id)).expect("receipt json"),
    )
    .expect("write receipt");
    fs::write(
        &behavior_path,
        serde_json::to_string(&behavior_snapshot_json(&candidate_id)).expect("behavior json"),
    )
    .expect("write behavior");
    fs::write(
        &determinism_path,
        serde_json::to_string(&determinism_readiness_json()).expect("determinism json"),
    )
    .expect("write determinism");
    fs::write(
        &proof_path,
        serde_json::to_string(&proof_pilot_json()).expect("proof json"),
    )
    .expect("write proof");
    fs::write(
        &waiver_path,
        serde_json::to_string(&stale_waiver_json(&candidate_id)).expect("waiver json"),
    )
    .expect("write waiver");

    let packet = review_packet_json(
        temp.path(),
        &[
            "--receipt-json",
            receipt_path.to_str().expect("receipt path"),
            "--behavior-json",
            behavior_path.to_str().expect("behavior path"),
            "--determinism-json",
            determinism_path.to_str().expect("determinism path"),
            "--proof-json",
            proof_path.to_str().expect("proof path"),
            "--waiver-json",
            waiver_path.to_str().expect("waiver path"),
        ],
    );

    assert_eq!(packet["summary"]["changedBehavior"], 1);
    assert_eq!(packet["summary"]["staleWaivers"], 1);
    assert_eq!(packet["summary"]["determinismObservations"], 1);
    assert_eq!(packet["summary"]["proofClaims"], 1);
    assert!(
        packet["missingReceipts"]
            .as_array()
            .expect("missing receipts")
            .iter()
            .all(|receipt| {
                receipt["invariantId"] != candidate_id || receipt["receiptKind"] != "cargo-check"
            }),
        "{packet}"
    );
    let action_kinds = packet["reviewActions"]
        .as_array()
        .expect("review actions")
        .iter()
        .map(|action| action["kind"].as_str().expect("action kind"))
        .collect::<Vec<_>>();
    assert!(action_kinds.contains(&"inspect-behavior"), "{packet}");
    assert!(action_kinds.contains(&"refresh-waiver"), "{packet}");
    assert!(action_kinds.contains(&"address-determinism"), "{packet}");
}

#[test]
fn cli_review_packet_rejects_wrong_input_packet_schema_id() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir_all(temp.path().join("src")).expect("create source dir");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"review-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(temp.path().join("src/lib.rs"), "pub fn fixture() {}\n").expect("write source");
    let receipt_path = temp.path().join("bad-receipt.json");
    let mut receipt = cargo_check_receipt_json("agent-r027:src.lib.rs:1");
    receipt["schemaId"] = serde_json::json!("agent.semantic-protocols.semantic-behavior-snapshot");
    fs::write(
        &receipt_path,
        serde_json::to_string(&receipt).expect("receipt json"),
    )
    .expect("write receipt");

    let output = run_cli([
        "review".as_ref(),
        "packet".as_ref(),
        "--receipt-json".as_ref(),
        receipt_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("has schemaId agent.semantic-protocols.semantic-behavior-snapshot"),
        "{stderr}"
    );
    assert!(
        stderr.contains("expected agent.semantic-protocols.semantic-verification-receipt"),
        "{stderr}"
    );
}

#[test]
fn cli_review_packet_rejects_wrong_input_packet_protocol_id() {
    let temp = TempDir::new().expect("temp dir");
    let behavior_path = temp.path().join("bad-behavior.json");
    let mut behavior = behavior_snapshot_json("agent-r027:src.lib.rs:1");
    behavior["protocolId"] = serde_json::json!("agent.semantic-protocols.verification-receipt");
    fs::write(
        &behavior_path,
        serde_json::to_string(&behavior).expect("behavior json"),
    )
    .expect("write behavior");

    let output = run_cli([
        "review".as_ref(),
        "packet".as_ref(),
        "--behavior-json".as_ref(),
        behavior_path.as_os_str(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("has protocolId agent.semantic-protocols.verification-receipt"),
        "{stderr}"
    );
    assert!(
        stderr.contains("expected agent.semantic-protocols.behavior-snapshot"),
        "{stderr}"
    );
}

fn review_packet_json(root: &Path, extra_args: &[&str]) -> Value {
    let mut args: Vec<&OsStr> = vec![
        OsStr::new("review"),
        OsStr::new("packet"),
        OsStr::new("--json"),
    ];
    args.extend(extra_args.iter().map(OsStr::new));
    args.push(root.as_os_str());
    let output = run_cli(args);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    serde_json::from_str::<Value>(&stdout).expect("review packet json")
}

fn write_invariant_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        r#"
        [package]
        name = "review-packet-fixture"
        version = "0.1.0"
        edition = "2021"
        "#,
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r#"
        pub type UserId = String;

        pub struct Account {
            pub user_id: String,
            pub tenant_id: u64,
            pub profile_url: String,
        }

        pub struct Session {
            pub status: String,
        }

        pub fn load_user(user_id: String) -> (String, u64) {
            (user_id, 1)
        }
        "#,
    )
    .expect("write source");
}

fn cargo_check_receipt_json(candidate_id: &str) -> Value {
    serde_json::json!({
        "schemaId": "agent.semantic-protocols.semantic-verification-receipt",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.verification-receipt",
        "protocolVersion": "1",
        "receiptId": "rust.cargo-check.review-packet-fixture",
        "producer": {
            "languageId": "rust",
            "providerId": "rs-harness",
            "adapterId": "rust.cargo-check",
            "namespace": "agent.semantic-protocols.languages.rust.rs-harness"
        },
        "tool": "cargo-check",
        "status": "passed",
        "command": {
            "argv": ["cargo", "check", "--message-format=json"],
            "outputFormat": "cargo-json"
        },
        "exitCode": 0,
        "summary": "cargo check passed",
        "observations": [{"kind": "exit-status", "message": "exit 0"}],
        "candidateIds": [candidate_id]
    })
}

fn behavior_snapshot_json(candidate_id: &str) -> Value {
    serde_json::json!({
        "schemaId": "agent.semantic-protocols.semantic-behavior-snapshot",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.behavior-snapshot",
        "protocolVersion": "1",
        "snapshotId": "rust.behavior.review-packet-fixture",
        "producer": {
            "languageId": "rust",
            "providerId": "rs-harness",
            "namespace": "agent.semantic-protocols.languages.rust.rs-harness"
        },
        "subject": {
            "kind": "function",
            "path": "src/lib.rs",
            "symbol": "load_user",
            "command": ["cargo", "test", "load_user_snapshot"]
        },
        "status": "changed",
        "observations": [
            {"kind": "diff", "message": "observable output changed", "path": "src/lib.rs", "line": 12}
        ],
        "receiptIds": ["rust.expect-test.review-packet-fixture"],
        "candidateIds": [candidate_id]
    })
}

fn determinism_readiness_json() -> Value {
    serde_json::json!({
        "schemaId": "agent.semantic-protocols.semantic-determinism-readiness",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.determinism-readiness",
        "protocolVersion": "1",
        "readinessId": "rust.determinism-readiness.review-packet-fixture",
        "producer": {
            "languageId": "rust",
            "providerId": "rs-harness",
            "namespace": "agent.semantic-protocols.languages.rust.rs-harness"
        },
        "project": {"root": "."},
        "status": "needs-injection",
        "observations": [
            {
                "observationId": "clock:src-lib-rs:1",
                "category": "clock",
                "evidenceKind": "function-call",
                "severity": "warning",
                "summary": "direct clock access",
                "path": "src/lib.rs",
                "line": 1,
                "direct": true
            }
        ],
        "suggestions": [
            {
                "kind": "trait-injection",
                "category": "clock",
                "message": "inject clock dependency",
                "path": "src/lib.rs",
                "line": 1,
                "traitName": "Clock"
            }
        ]
    })
}

fn proof_pilot_json() -> Value {
    serde_json::json!({
        "schemaId": "agent.semantic-protocols.semantic-formal-proof-pilot",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.formal-proof-pilot",
        "protocolVersion": "1",
        "proofId": "rust.proof.review-packet-fixture",
        "producer": {
            "languageId": "rust",
            "providerId": "rs-harness",
            "namespace": "agent.semantic-protocols.languages.rust.rs-harness"
        },
        "target": {
            "kind": "dependency-graph-acyclicity",
            "name": "owner dependency graph cycle detection",
            "ruleIds": ["RUST-AGENT-OWNER-GRAPH-009"],
            "ownerPath": "src/rules/agent_policy/dependency_graph.rs",
            "symbol": "owner_dependency_cycle_indices"
        },
        "method": {
            "kind": "exhaustive-small-model",
            "tool": "rs-harness",
            "command": ["rs-harness", "proof", "pilot", "dependency-graph-acyclicity", "--json"]
        },
        "status": "proved-bounded",
        "claims": [
            {
                "claimId": "cycle-detection-iff-directed-cycle",
                "statement": "cycle detection agrees with independent checker",
                "status": "proved-bounded"
            }
        ],
        "checks": [
            {
                "checkId": "exhaustive-directed-graphs-up-to-3",
                "status": "proved-bounded",
                "summary": "checked all directed graphs up to 3 nodes",
                "modelsChecked": 72,
                "maxNodes": 3
            }
        ]
    })
}

fn stale_waiver_json(candidate_id: &str) -> Value {
    serde_json::json!({
        "waiverId": "waiver.review-packet-fixture.expect-test",
        "invariantId": candidate_id,
        "receiptKind": "expect-test",
        "status": "stale",
        "owner": "reviewer",
        "reason": "snapshot migration is pending",
        "expiresAt": "2026-05-01"
    })
}
