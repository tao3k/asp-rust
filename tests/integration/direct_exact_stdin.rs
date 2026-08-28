use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn exact_selector_reads_one_stdin_owner_without_state_or_snapshot_via_cli() {
    let source = b"pub struct QueryOptions {\n    pub code: bool,\n}\n";
    let source_digest = blake3::hash(source).to_hex().to_string();
    let request = serde_json::json!({
        "schemaId": "agent.semantic-protocols.provider-native-exact-request",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "rs-harness",
        "structuralSelector": "rust://src/cli/query_options.rs#item/struct/QueryOptions",
        "ownerPath": "src/cli/query_options.rs",
        "projectionKind": "source",
        "generationIdentityDigest": "a".repeat(64),
        "parserIdentityDigest": "b".repeat(64),
        "queryPackDigest": "c".repeat(64),
        "sourceDigest": source_digest,
        "sourceByteLength": source.len(),
        "sourceEncoding": "base64",
        "sourceBytesBase64": encode_base64(source),
        "transport": "stdin-json"
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .args([
            "query",
            "--selector",
            "rust://src/cli/query_options.rs#item/struct/QueryOptions",
            "--projection",
            "source",
            "--json",
            "--asp-provider-id",
            "rs-harness",
            "--asp-exact-request-stdin",
        ])
        .env("AST_STATE_HOME", "/path/that/must/not/be/opened")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rs-harness");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(
            serde_json::to_string(&request)
                .expect("request JSON")
                .as_bytes(),
        )
        .expect("write request");
    let output = child.wait_with_output().expect("wait for rs-harness");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("provider-native packet");
    assert_eq!(
        packet["schemaId"],
        "agent.semantic-protocols.provider-native-exact-projection"
    );
    assert_eq!(packet["schemaVersion"], "1");
    assert_eq!(packet["projectionMode"], "source");
    assert_eq!(
        packet["requestedStructuralSelector"],
        request["structuralSelector"]
    );
    assert_eq!(
        packet["structuralSelector"],
        "rust://src/cli/query_options.rs#item/struct/QueryOptions"
    );
    assert_eq!(packet["sourceContentDigest"], source_digest);
    assert_eq!(
        packet["projectionText"],
        "pub struct QueryOptions {\n    pub code: bool,\n}"
    );
    assert_eq!(packet["sourceByteStart"], 0);
    assert_eq!(packet["sourceByteEnd"], source.len() - 1);
}

#[test]
fn callable_skeleton_child_selector_round_trips_to_source() {
    let source =
        b"pub fn selected(value: u64) -> u64 {\n    if value > 1 { value } else { 0 }\n}\n";
    let source_digest = blake3::hash(source).to_hex().to_string();
    let root_selector = "rust://src/lib.rs#item/function/selected";
    let request = |structural_selector: &str, projection_kind: &str| {
        serde_json::json!({
            "schemaId": "agent.semantic-protocols.provider-native-exact-request",
            "schemaVersion": "1",
            "languageId": "rust",
            "providerId": "rs-harness",
            "structuralSelector": structural_selector,
            "ownerPath": "src/lib.rs",
            "projectionKind": projection_kind,
            "generationIdentityDigest": "a".repeat(64),
            "parserIdentityDigest": "b".repeat(64),
            "queryPackDigest": "c".repeat(64),
            "sourceDigest": source_digest.clone(),
            "sourceByteLength": source.len(),
            "sourceEncoding": "base64",
            "sourceBytesBase64": encode_base64(source),
            "transport": "stdin-json"
        })
    };
    let invoke = |selector: &str, projection_kind: &str| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_asp-rust"))
            .args([
                "query",
                "--selector",
                selector,
                "--projection",
                projection_kind,
                "--json",
                "--asp-provider-id",
                "rs-harness",
                "--asp-exact-request-stdin",
            ])
            .env("AST_STATE_HOME", "/path/that/must/not/be/opened")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn rs-harness");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(
                serde_json::to_string(&request(selector, projection_kind))
                    .expect("request JSON")
                    .as_bytes(),
            )
            .expect("write request");
        let output = child.wait_with_output().expect("wait for rs-harness");
        assert!(
            output.status.success(),
            "stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("provider-native packet")
    };

    let skeleton = invoke(root_selector, "callable-skeleton");
    for envelope_field in [
        "schemaId",
        "schemaVersion",
        "projectionKind",
        "languageId",
        "providerId",
    ] {
        assert!(
            skeleton["projectionPayload"].get(envelope_field).is_none(),
            "callable-skeleton payload retained envelope field {envelope_field}: {}",
            skeleton["projectionPayload"]
        );
    }
    let branch_selector = skeleton["projectionPayload"]["nodes"]
        .as_array()
        .expect("skeleton nodes")
        .iter()
        .find(|node| node["kind"] == "branch")
        .and_then(|node| node["selector"].as_str())
        .expect("branch selector")
        .to_string();
    let branch = invoke(&branch_selector, "source");
    assert_eq!(branch["requestedStructuralSelector"], branch_selector);
    assert_eq!(branch["structuralSelector"], branch_selector);
    assert_eq!(
        branch["projectionText"],
        "if value > 1 { value } else { 0 }"
    );
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(TABLE[(first >> 2) as usize] as char);
        encoded.push(TABLE[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(third & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

fn run_exact_failure(source: &[u8], selector: &str) -> std::process::Output {
    let owner_path = selector
        .strip_prefix("rust://")
        .and_then(|value| value.split_once('#').map(|(owner, _)| owner))
        .expect("fixture selector owner");
    let request = serde_json::json!({
        "schemaId": "agent.semantic-protocols.provider-native-exact-request",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "rs-harness",
        "structuralSelector": selector,
        "ownerPath": owner_path,
        "projectionKind": "source",
        "generationIdentityDigest": "a".repeat(64),
        "parserIdentityDigest": "b".repeat(64),
        "queryPackDigest": "c".repeat(64),
        "sourceDigest": blake3::hash(source).to_hex().to_string(),
        "sourceByteLength": source.len(),
        "sourceEncoding": "base64",
        "sourceBytesBase64": encode_base64(source),
        "transport": "stdin-json"
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .args([
            "query",
            "--selector",
            selector,
            "--projection",
            "source",
            "--json",
            "--asp-provider-id",
            "rs-harness",
            "--asp-exact-request-stdin",
        ])
        .env("AST_STATE_HOME", "/path/that/must/not/be/opened")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rs-harness");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(
            serde_json::to_string(&request)
                .expect("request JSON")
                .as_bytes(),
        )
        .expect("write request");
    child.wait_with_output().expect("wait for rs-harness")
}

#[test]
fn exact_selector_reports_kind_mismatch_only_for_the_same_owner_identity() {
    let output = run_exact_failure(
        b"pub struct QueryOptions;\n",
        "rust://src/cli/query_options.rs#item/function/QueryOptions",
    );

    assert!(
        output.status.success(),
        "provider transport failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("typed mismatch packet");
    assert_eq!(packet["resolutionState"], "kind-mismatch");
    assert_eq!(packet["reasonKind"], "owner-item-kind-mismatch");
    assert_ne!(packet["reasonKind"], "snapshot-item-kind-mismatch");
    assert_eq!(packet["actualKinds"], serde_json::json!(["struct"]));
}

#[test]
fn exact_method_without_impl_owner_fails_as_identity_incomplete() {
    let output = run_exact_failure(
        b"struct QueryOptions;\nimpl QueryOptions { fn parse() {} }\n",
        "rust://src/cli/query_options.rs#item/method/parse",
    );

    assert!(
        output.status.success(),
        "provider transport failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let packet: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("typed incomplete identity packet");
    assert_eq!(packet["resolutionState"], "identity-incomplete");
    assert_eq!(packet["reasonKind"], "canonical-item-scope-required");
    assert!(
        packet["candidates"]
            .as_array()
            .is_some_and(|candidates| candidates.iter().any(|candidate| {
                candidate
                    == "rust://src/cli/query_options.rs#item/method/parse/scope/implementation-owner/type/QueryOptions"
            })),
        "{packet}"
    );
}
