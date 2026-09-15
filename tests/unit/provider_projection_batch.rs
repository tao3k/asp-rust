use super::handle_language_projection_batch_value;
use serde_json::{Value, json};

fn request(owners: Value) -> Value {
    json!({
        "schemaId": "agent.semantic-protocols.provider-language-projection-batch-request",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "asp-rust",
        "workspaceIdentity": "workspace-test",
        "generationRootDigest": "blake3-256:generation",
        "parserIdentityDigest": "blake3-256:parser",
        "queryPackDigest": "blake3-256:query-pack",
        "baseGenerationRootDigest": null,
        "owners": owners,
        "auxiliaryOwners": [],
    })
}

#[test]
fn projection_batch_rejects_invalid_auxiliary_owner_base64() {
    let mut request = request(json!([{
        "ownerPath": "src/lib.rs",
        "sourceLeafDigest": "blake3-256:primary",
        "sourceEncoding": "utf8",
        "sourceText": "pub fn primary() {}\n"
    }]));
    request["auxiliaryOwners"] = json!([{
        "ownerPath": "src/auxiliary.rs",
        "sourceLeafDigest": "blake3-256:auxiliary",
        "sourceEncoding": "base64",
        "sourceBytesBase64": "not-valid-base64***"
    }]);

    let error = handle_language_projection_batch_value(&request)
        .expect_err("invalid auxiliary bytes must fail the generation batch");
    assert!(
        error.contains("decode projection owner base64 source"),
        "unexpected auxiliary decoding failure: {error}"
    );
}

#[test]
fn projection_batch_isolates_one_syntax_unavailable_owner() {
    let response = handle_language_projection_batch_value(&request(json!([
        {
            "ownerPath": "src/ready.rs",
            "sourceLeafDigest": "blake3-256:ready",
            "sourceEncoding": "utf8",
            "sourceText": "pub fn ready() -> usize { 1 }\n"
        },
        {
            "ownerPath": "src/unavailable.rs",
            "sourceLeafDigest": "blake3-256:unavailable",
            "sourceEncoding": "utf8",
            "sourceText": "pub fn unavailable( {\n"
        }
    ])))
    .expect("projection batch remains transport-ready");
    let response: Value = serde_json::from_slice(&response).expect("response JSON");
    let owners = response["owners"].as_array().expect("owners");

    assert_eq!(owners[0]["projectionState"], "ready");
    assert!(owners[0]["diagnostic"].is_null());
    assert!(
        !owners[0]["items"]
            .as_array()
            .expect("ready items")
            .is_empty()
    );
    assert_eq!(owners[1]["projectionState"], "syntax-unavailable");
    assert_eq!(
        owners[1]["diagnostic"]["reasonKind"],
        "source-syntax-unavailable"
    );
    assert!(
        owners[1]["items"]
            .as_array()
            .expect("unavailable items")
            .is_empty()
    );
    assert!(
        owners[1]["relations"]
            .as_array()
            .expect("unavailable relations")
            .is_empty()
    );
}

#[test]
fn projection_batch_reuses_owner_parse_without_rebasing_nested_source_locations() {
    let source = "pub const PREFIX: usize = 1;\n\npub fn locate(value: usize) -> usize {\n    if value > 0 { value } else { PREFIX }\n}\n";
    let response = handle_language_projection_batch_value(&request(json!([{
        "ownerPath": "src/location.rs",
        "sourceLeafDigest": "blake3-256:location",
        "sourceEncoding": "utf8",
        "sourceText": source,
    }])))
    .expect("projection batch");
    let response: Value = serde_json::from_slice(&response).expect("response JSON");
    let callable = response["owners"][0]["items"]
        .as_array()
        .expect("projected items")
        .iter()
        .find(|item| item["name"] == "locate")
        .expect("locate callable");
    let branch = callable["projections"][0]["payload"]["nodes"]
        .as_array()
        .expect("skeleton nodes")
        .iter()
        .find(|node| node["kind"] == "branch")
        .expect("if branch");

    assert_eq!(
        branch["sourceLocatorHint"]["sourceByteStart"],
        source.find("if value").expect("if byte offset") as u64
    );
}

#[test]
fn representative_generation_projection_produces_the_complete_batch() {
    const OWNER_COUNT: usize = 32;
    const CALLABLES_PER_OWNER: usize = 32;
    let owners = (0..OWNER_COUNT)
        .map(|owner| {
            let mut source = String::new();
            for callable in 0..CALLABLES_PER_OWNER {
                source.push_str(&format!(
                    "pub fn owner_{owner:02}_callable_{callable:02}(value: usize) -> usize {{ value.wrapping_add({callable}) }}\n"
                ));
            }
            json!({
                "ownerPath": format!("src/owner_{owner:02}.rs"),
                "sourceLeafDigest": format!("blake3-256:owner-{owner:02}"),
                "sourceEncoding": "utf8",
                "sourceText": source,
            })
        })
        .collect::<Vec<_>>();
    let request = request(Value::Array(owners));
    let response = handle_language_projection_batch_value(&request)
        .expect("representative generation projection batch");
    let response: Value = serde_json::from_slice(&response).expect("projection response JSON");
    assert_eq!(
        response["owners"]
            .as_array()
            .expect("projected owners")
            .len(),
        OWNER_COUNT
    );
}
