use super::handle_owner_search_request;

#[test]
fn single_owner_request_parses_exactly_one_owner() {
    let source = b"pub fn alpha() {}\npub struct Beta;\n";
    let request = request_json(source);
    let response =
        handle_owner_search_request(request.as_bytes(), "rs-harness").expect("owner response");
    assert_eq!(response.parsed_owner_count, 1);
    assert_eq!(response.projection_completeness, "complete-owner");
    assert_eq!(response.requested_owner_path, "src/lib.rs");
    assert_eq!(response.requested_projection_mode, "complete-owner");
    assert_eq!(response.projections.len(), 2);
    assert_eq!(
        response.projections[0]
            .canonical_item_selector
            .identity()
            .symbol
            .as_str(),
        "alpha"
    );
    assert_eq!(
        response.projections[0]
            .canonical_item_selector
            .structural_selector,
        "rust://src/lib.rs#item/function/alpha"
    );
}

#[test]
fn owner_request_rejects_digest_echo_drift() {
    let source = b"pub fn alpha() {}\n";
    let mut value: serde_json::Value =
        serde_json::from_str(&request_json(source)).expect("request JSON");
    value["sourceFingerprint"]["contentDigest"] = serde_json::Value::String("0".repeat(64));
    let error = handle_owner_search_request(
        serde_json::to_string(&value)
            .expect("encode request")
            .as_bytes(),
        "rs-harness",
    )
    .expect_err("reject digest drift");
    assert!(error.contains("digest mismatch"), "error={error}");
}

#[test]
fn legacy_query_field_is_rejected() {
    let source = b"pub fn alpha() {}\n";
    let mut value: serde_json::Value =
        serde_json::from_str(&request_json(source)).expect("request JSON");
    value
        .as_object_mut()
        .expect("request object")
        .remove("projectionMode");
    value["query"] = serde_json::json!({"text": "alpha", "itemMode": "items"});
    let error = handle_owner_search_request(
        serde_json::to_string(&value)
            .expect("encode request")
            .as_bytes(),
        "rs-harness",
    )
    .expect_err("legacy query must fail closed");
    assert!(error.contains("unknown field"), "error={error}");
}

fn request_json(source: &[u8]) -> String {
    serde_json::json!({
        "schemaId": "agent.semantic-protocols.provider-native-owner-search-request",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "rs-harness",
        "workspaceIdentity": "workspace-1",
        "providerWorkspaceIdentityDigest": "a".repeat(64),
        "ownerPath": "src/lib.rs",
        "sourceFingerprint": {
            "fileIdentity": "file-1",
            "sizeBytes": source.len(),
            "modifiedUnixNanos": 1,
            "changeTimeUnixNanos": 2,
            "contentDigest": blake3::hash(source).to_hex().to_string()
        },
        "sourceEncoding": "base64",
        "sourceBytesBase64": encode_base64(source),
        "projectionMode": "complete-owner",
        "transport": "stdin-json"
    })
    .to_string()
}

fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(ALPHABET[(first >> 2) as usize] as char);
        encoded.push(ALPHABET[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(third & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    encoded
}
