use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn complete_owner_request_publishes_every_item_without_a_query_via_cli() {
    let source = b"pub fn alpha() {}\npub struct Beta;\n";
    let packet = invoke(&request(source));
    assert_eq!(
        packet["requestedProjectionMode"],
        serde_json::Value::String("complete-owner".to_owned())
    );
    assert_eq!(
        packet["projectionCompleteness"],
        serde_json::Value::String("complete-owner".to_owned())
    );
    let names = packet["projections"]
        .as_array()
        .expect("projections")
        .iter()
        .filter_map(|projection| projection["canonicalItemSelector"]["symbol"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["alpha", "Beta"]);
}

#[test]
fn complete_owner_request_includes_private_and_async_items() {
    let source = b"async fn install() {}\nfn reconcile() {}\n";
    let packet = invoke(&request(source));
    let names = packet["projections"]
        .as_array()
        .expect("projections")
        .iter()
        .filter_map(|projection| projection["canonicalItemSelector"]["symbol"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["install", "reconcile"]);
}

#[test]
fn complete_owner_request_preserves_impl_and_trait_scopes_in_the_canonical_packet() {
    let source = br#"
trait Render { fn render(&self); }
struct Alpha;
struct Beta;
impl Render for Alpha { fn render(&self) {} }
impl Render for Beta { fn render(&self) {} }
"#;
    let packet = invoke(&request(source));
    let methods = packet["projections"]
        .as_array()
        .expect("projections")
        .iter()
        .map(|projection| &projection["canonicalItemSelector"])
        .filter(|selector| selector["kind"] == "method" && selector["symbol"] == "render")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2, "{packet}");
    let selectors = methods
        .iter()
        .map(|selector| selector["structuralSelector"].as_str().expect("selector"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(selectors.len(), 2, "{packet}");
    for selector in methods {
        assert_eq!(selector["schemaId"], "asp.canonical-item-selector.v1");
        assert_eq!(selector["schemaVersion"], "1");
        assert_eq!(selector["languageId"], "rust");
        let scopes = selector["scopes"].as_array().expect("canonical scopes");
        assert_eq!(scopes.len(), 2, "{selector}");
        assert_eq!(scopes[0]["relation"], "implementation-owner");
        assert_eq!(scopes[0]["kind"], "type");
        assert!(matches!(
            scopes[0]["symbol"].as_str(),
            Some("Alpha" | "Beta")
        ));
        assert_eq!(scopes[1]["relation"], "trait-owner");
        assert_eq!(scopes[1]["kind"], "trait");
        assert_eq!(scopes[1]["symbol"], "Render");
    }
}

#[test]
fn legacy_query_request_fails_closed() {
    let source = b"pub fn alpha() {}\n";
    let mut legacy = request(source);
    legacy
        .as_object_mut()
        .expect("request")
        .remove("projectionMode");
    legacy["query"] = serde_json::json!({"text": "alpha", "itemMode": "items"});

    let mut child = command();
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(
            serde_json::to_string(&legacy)
                .expect("request JSON")
                .as_bytes(),
        )
        .expect("write request");
    let output = child.wait_with_output().expect("wait for rs-harness");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown field"),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn invoke(request: &serde_json::Value) -> serde_json::Value {
    let mut child = command();
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(
            serde_json::to_string(request)
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
    serde_json::from_slice(&output.stdout).expect("owner-search response")
}

fn command() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .args(["owner-search-stdin", "--asp-provider-id", "rs-harness"])
        .env("AST_STATE_HOME", "/path/that/must/not/be-opened")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rs-harness")
}

fn request(source: &[u8]) -> serde_json::Value {
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
