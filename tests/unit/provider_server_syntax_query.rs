#[test]
fn native_capture_carries_an_executable_exact_selector() {
    let source = "pub fn run() {}\n";
    let payload = serde_json::json!({
        "schemaId": "agent.semantic-protocols.provider-syntax-query-request",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "asp-rust",
        "ownerPath": "src/lib.rs",
        "sourceContentDigest": blake3::hash(source.as_bytes()).to_hex().to_string(),
        "queryDigest": "1".repeat(64),
        "source": source,
        "plan": {
            "patterns": [{
                "index": 0,
                "captures": ["function.name"],
                "nodeTypes": ["function_item"],
                "fields": ["name"]
            }],
            "captures": ["function.name"],
            "nodeTypes": ["function_item"],
            "fields": ["name"],
            "predicates": []
        }
    });
    let encoded = super::handle_syntax_query_operation_value(&payload).expect("syntax query");
    let response: serde_json::Value = serde_json::from_slice(&encoded).expect("response");
    assert_eq!(
        response["captures"][0]["structuralSelector"],
        "rust://src/lib.rs#item/function/run"
    );
}
