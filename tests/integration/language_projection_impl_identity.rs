use serde_json::Value;
use std::io::Write as _;
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
    let workspace = tempfile::tempdir().expect("fixture workspace");
    let source_dir = workspace.path().join("src");
    std::fs::create_dir_all(&source_dir).expect("fixture source directory");
    std::fs::write(
        workspace.path().join("Cargo.toml"),
        "[package]\nname = \"projection-identity-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("fixture manifest");
    std::fs::write(source_dir.join("lib.rs"), IDENTITY_SOURCE).expect("fixture source");

    let output = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .args([
            "projection",
            "src/lib.rs",
            "--workspace",
            workspace.path().to_str().expect("utf-8 fixture path"),
            "--json",
        ])
        .output()
        .expect("run projection");
    assert!(
        output.status.success(),
        "projection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let packet: Value = serde_json::from_slice(&output.stdout).expect("projection packet");
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
        .filter(|item| item["kind"] == "impl")
        .collect::<Vec<_>>();

    assert_eq!(
        parse_items.len(),
        3,
        "trait declaration plus two impl methods; executable={}; packet={packet}",
        env!("CARGO_BIN_EXE_rs-harness")
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

    for selector in selectors {
        let source = IDENTITY_SOURCE.as_bytes();
        let request = serde_json::json!({
            "schemaId": "agent.semantic-protocols.provider-native-exact-request",
            "schemaVersion": "1",
            "languageId": "rust",
            "providerId": "rs-harness",
            "structuralSelector": selector,
            "ownerPath": "src/lib.rs",
            "projectionKind": "source",
            "generationIdentityDigest": "a".repeat(64),
            "parserIdentityDigest": "b".repeat(64),
            "queryPackDigest": "c".repeat(64),
            "sourceDigest": blake3::hash(source).to_hex().to_string(),
            "sourceByteLength": source.len(),
            "sourceEncoding": "base64",
            "sourceBytesBase64": encode_base64(source),
            "transport": "stdin-json",
        });
        let mut child = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
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
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn exact selector parser");
        child
            .stdin
            .take()
            .expect("exact stdin")
            .write_all(
                serde_json::to_string(&request)
                    .expect("exact request JSON")
                    .as_bytes(),
            )
            .expect("write exact request");
        let exact = child.wait_with_output().expect("run exact selector parser");
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&exact.stdout),
            String::from_utf8_lossy(&exact.stderr)
        );
        assert!(
            exact.status.success(),
            "projection emitted a selector the exact consumer cannot materialize: {selector}; diagnostic={diagnostic}"
        );
        assert!(
            !exact.stdout.is_empty(),
            "exact projection must return source: {selector}"
        );
    }
}

#[test]
fn batch_projection_reads_framed_owner_bytes_without_workspace_scan() {
    let packet = run_projection_batch(IDENTITY_SOURCE.as_bytes(), IDENTITY_SOURCE.len());
    let owner = &packet["owners"][0];
    assert_eq!(owner["ownerPath"], "src/lib.rs");
    assert_eq!(owner["sourceLeafDigest"], "11".repeat(32));
    let parse_items = owner["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| item["name"] == "parse")
        .collect::<Vec<_>>();
    assert_eq!(parse_items.len(), 3);
    let impl_items = owner["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| item["kind"] == "impl")
        .collect::<Vec<_>>();
    assert_eq!(
        impl_items.len(),
        2,
        "batch generation must retain enclosing impls"
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
    let install = owner["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["name"] == "run_install_command")
        .expect("top-level callable item");
    assert_eq!(
        install["projections"][0]["projectionKind"],
        "callable-skeleton"
    );
    assert_eq!(
        install["projections"][0]["payload"]["schemaId"],
        "agent.semantic-protocols.callable-skeleton-projection"
    );
    for item in parse_items {
        assert_eq!(item["identity"]["languageId"], "rust");
        assert!(item["identity"]["scopes"].is_array());
        assert!(item.get("implOwner").is_none());
        assert!(item.get("traitOwner").is_none());
        let start = item["sourceByteStart"]
            .as_u64()
            .expect("batch sourceByteStart") as usize;
        let end = item["sourceByteEnd"].as_u64().expect("batch sourceByteEnd") as usize;
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

#[test]
fn batch_projection_rejects_a_truncated_owner_frame() {
    let header = projection_batch_header(IDENTITY_SOURCE.len() + 1);
    let header_bytes = serde_json::to_vec(&header).expect("batch header");
    let mut child = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .arg("projection-batch-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn batch projection");
    let mut stdin = child.stdin.take().expect("batch stdin");
    stdin
        .write_all(&(header_bytes.len() as u32).to_be_bytes())
        .expect("header length");
    stdin.write_all(&header_bytes).expect("header");
    stdin
        .write_all(IDENTITY_SOURCE.as_bytes())
        .expect("owner bytes");
    drop(stdin);
    let output = child.wait_with_output().expect("batch output");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("failed to read projection owner frame src/lib.rs")
    );
}

fn run_projection_batch(source: &[u8], declared_length: usize) -> Value {
    let header = projection_batch_header(declared_length);
    let header_bytes = serde_json::to_vec(&header).expect("batch header");
    let mut child = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .arg("projection-batch-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn batch projection");
    let mut stdin = child.stdin.take().expect("batch stdin");
    stdin
        .write_all(&(header_bytes.len() as u32).to_be_bytes())
        .expect("header length");
    stdin.write_all(&header_bytes).expect("header");
    stdin.write_all(source).expect("owner bytes");
    drop(stdin);
    let output = child.wait_with_output().expect("batch output");
    assert!(
        output.status.success(),
        "batch projection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("batch response")
}

fn projection_batch_header(byte_length: usize) -> Value {
    serde_json::json!({
        "schemaId": "asp.provider-language-projection-batch-request.v1",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "rs-harness",
        "workspaceIdentity": "workspace-fixture",
        "generationRootDigest": "00".repeat(32),
        "parserIdentityDigest": "22".repeat(32),
        "queryPackDigest": "33".repeat(32),
        "transport": "framed-stdin-v1",
        "owners": [{
            "ownerPath": "src/lib.rs",
            "sourceLeafDigest": "11".repeat(32),
            "byteLength": byte_length,
        }],
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
