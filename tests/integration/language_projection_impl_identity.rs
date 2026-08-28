use serde_json::Value;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::process::Command;
use std::process::Stdio;

const IDENTITY_SOURCE: &str = r#"
pub trait Parse {
    fn parse(&self);
}

pub struct CliOptions;

pub fn run_install_command() {}

impl CliOptions {
    pub fn parse(&self) {}
}

impl Parse for CliOptions {
    fn parse(&self) {}
}
"#;

#[test]
fn projection_preserves_canonical_impl_and_trait_method_identity() {
    let packet = project_through_runtime(IDENTITY_SOURCE);
    let parse_items = packet["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| item["name"] == "parse")
        .collect::<Vec<_>>();
    let impl_items = packet["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| item["kind"] == "implementation")
        .collect::<Vec<_>>();

    assert_eq!(
        parse_items.len(),
        3,
        "trait declaration plus two impl methods; executable={}; packet={packet}",
        env!("CARGO_BIN_EXE_asp-rust")
    );
    assert_eq!(
        impl_items.len(),
        2,
        "both enclosing impls must be projected"
    );
    assert!(impl_items.iter().any(|item| {
        identity_scopes(item) == vec![("implementation-owner", "type", "CliOptions")]
    }));
    assert!(impl_items.iter().any(|item| {
        identity_scopes(item)
            == vec![
                ("implementation-owner", "type", "CliOptions"),
                ("trait-owner", "trait", "Parse"),
            ]
    }));
    assert!(parse_items.iter().any(|item| {
        item["kind"] == "method"
            && identity_scopes(item) == vec![("implementation-owner", "type", "CliOptions")]
    }));
    assert!(parse_items.iter().any(|item| {
        item["kind"] == "method"
            && identity_scopes(item)
                == vec![
                    ("implementation-owner", "type", "CliOptions"),
                    ("trait-owner", "trait", "Parse"),
                ]
    }));
    assert!(parse_items.iter().any(|item| {
        item["kind"] == "trait-function"
            && identity_scopes(item) == vec![("trait-owner", "trait", "Parse")]
    }));

    let selectors = parse_items
        .iter()
        .copied()
        .chain(impl_items.iter().copied())
        .filter_map(|item| item["selector"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(selectors.len(), 5, "canonical selectors must be unique");

    for item in &parse_items {
        assert_eq!(item["identity"]["languageId"], "rust");
        assert!(item.get("implOwner").is_none());
        assert!(item.get("traitOwner").is_none());
        let start = item["sourceByteStart"]
            .as_u64()
            .expect("projection sourceByteStart") as usize;
        let end = item["sourceByteEnd"]
            .as_u64()
            .expect("projection sourceByteEnd") as usize;
        assert!(start < end && end <= IDENTITY_SOURCE.len());
        assert!(IDENTITY_SOURCE[start..end].contains("fn parse"));
    }
}

fn identity_scopes(item: &Value) -> Vec<(&str, &str, &str)> {
    item["identity"]["scopes"]
        .as_array()
        .expect("canonical identity scopes")
        .iter()
        .map(|scope| {
            (
                scope["relation"].as_str().expect("scope relation"),
                scope["kind"].as_str().expect("scope kind"),
                scope["symbol"].as_str().expect("scope symbol"),
            )
        })
        .collect()
}

fn project_through_runtime(source_text: &str) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .arg("serve")
        .env("ASP_PROVIDER_ARTIFACT_DIGEST", "a".repeat(64))
        .env("ASP_PROVIDER_REGISTRATION_DIGEST", "b".repeat(64))
        .env("ASP_PROVIDER_RUNTIME_CONTRACT_DIGEST", "c".repeat(64))
        .env("ASP_PROVIDER_ID", "asp-rust")
        .env("ASP_PROVIDER_LANGUAGE_ID", "rust")
        .env("ASP_CLIENT_SERVER_HOST", "127.0.0.1:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn Rust provider server");
    let mut stdout = BufReader::new(child.stdout.take().expect("provider bootstrap stdout"));
    let mut bootstrap_line = String::new();
    stdout
        .read_line(&mut bootstrap_line)
        .expect("read provider bootstrap");
    let bootstrap: Value = serde_json::from_str(&bootstrap_line).expect("provider bootstrap JSON");
    assert_eq!(bootstrap["schemaVersion"], "1");
    assert_eq!(bootstrap["transport"], "http-json");
    assert_eq!(bootstrap["state"], "ready");
    let authority = bootstrap["endpoint"]
        .as_str()
        .expect("provider endpoint")
        .strip_prefix("http://")
        .and_then(|endpoint| endpoint.strip_suffix('/'))
        .expect("HTTP provider authority");

    let request = serde_json::json!({
        "schemaId": "agent.semantic-protocols.provider-runtime-request-frame",
        "schemaVersion": "1",
        "requestId": "projection-identity",
        "operation": "projection-batch",
        "payload": {
            "schemaId": "agent.semantic-protocols.provider-language-projection-batch-request",
            "schemaVersion": "1",
            "languageId": "rust",
            "providerId": "asp-rust",
            "workspaceIdentity": "projection-identity-fixture",
            "generationRootDigest": "d".repeat(64),
            "parserIdentityDigest": "e".repeat(64),
            "queryPackDigest": "f".repeat(64),
            "owners": [{
                "ownerPath": "src/lib.rs",
                "sourceLeafDigest": blake3::hash(source_text.as_bytes()).to_hex().to_string(),
                "sourceEncoding": "utf8",
                "sourceText": source_text,
            }],
        },
    });
    let request_body = serde_json::to_string(&request).expect("runtime request JSON");
    let response = http_request(
        authority,
        &format!(
            "POST /v1/provider-runtime HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            request_body.len(),
            request_body
        ),
    )
    .expect("projection runtime request");
    let response: Value = serde_json::from_str(http_body(&response).expect("runtime body"))
        .expect("runtime response JSON");
    assert_eq!(response["schemaVersion"], "1");
    assert_eq!(response["requestId"], "projection-identity");
    assert_eq!(
        response["outcome"], "ready",
        "provider error response: {response}"
    );
    let packet = response["payload"]["owners"][0].clone();

    http_request(
        authority,
        "POST /shutdown HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    )
    .expect("provider shutdown");
    let output = child
        .wait_with_output()
        .expect("wait for provider shutdown");
    assert!(
        output.status.success(),
        "provider server failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    packet
}

fn http_request(authority: &str, request: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(authority)
        .map_err(|error| format!("connect to provider server {authority}: {error}"))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .map_err(|error| format!("set provider read timeout: {error}"))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write provider request: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("read provider response: {error}"))?;
    if !response.starts_with("HTTP/1.1 200") {
        return Err(format!("provider request failed: {response}"));
    }
    Ok(response)
}

fn http_body(response: &str) -> Result<&str, String> {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .ok_or_else(|| "provider response does not contain an HTTP body".to_string())
}
