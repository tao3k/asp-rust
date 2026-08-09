mod failures;
mod replace;
mod split_owner;
use std::ffi::OsString;
use std::fs;
use std::io::Write;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::support::{run_cli_with_stdin, write_clean_source, write_manifest};

pub(super) fn write_replace_item_fixture() -> TempDir {
    let root = TempDir::new().expect("tempdir");
    write_manifest(root.path(), "ast-patch-fixture");
    write_clean_source(root.path());
    fs::write(
        root.path().join("src/lib.rs"),
        "pub fn demo() -> usize {\n    1\n}\n",
    )
    .expect("write lib");
    root
}

pub(super) fn write_split_owner_fixture() -> TempDir {
    let root = TempDir::new().expect("tempdir");
    write_manifest(root.path(), "ast-patch-split-fixture");
    write_clean_source(root.path());
    fs::write(
        root.path().join("src/lib.rs"),
        r#"//! crate docs

pub fn keep() -> usize {
    0
}

pub fn moved<T>(value: T) -> usize
where
    T: std::fmt::Debug,
{
    let text = "{";
    let _ = format!("{text:?}");
    let _ = value;
    1
}
"#,
    )
    .expect("write lib");
    root
}

pub(super) fn replace_item_packet(snippet: &str) -> String {
    json!({
        "target": {
            "ownerPath": "src/lib.rs",
            "locator": "src/lib.rs#fn:demo",
            "read": "src/lib.rs:1:3",
            "itemName": "demo",
            "itemKind": "fn"
        },
        "operation": {
            "op": "replace_item",
            "snippet": snippet,
            "expectedSnippet": "pub fn demo",
            "maxEdits": 1
        }
    })
    .to_string()
}

pub(super) fn split_owner_items_packet() -> String {
    split_owner_items_packet_with_max_edits(2)
}

pub(super) fn split_owner_items_packet_with_max_edits(max_edits: usize) -> String {
    json!({
        "target": {
            "ownerPath": "src/lib.rs",
            "locator": "src/lib.rs#fn:moved",
            "read": "src/lib.rs:7:15",
            "itemName": "moved",
            "itemKind": "fn"
        },
        "operation": {
            "op": "split_owner_items",
            "mutationSource": "provider-native",
            "snippetRequired": false,
            "codeInPrompt": false,
            "mechanicalKind": "owner-items",
            "expectedSnippet": "let text = \"{\";",
            "maxEdits": max_edits,
            "fields": {
                "destinationPath": "src/split.rs",
                "moduleName": "split"
            }
        }
    })
    .to_string()
}

pub(super) fn run_ast_patch_with_packet(
    mode: &str,
    root: &std::path::Path,
    packet: &str,
) -> std::process::Output {
    run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from(mode),
            OsString::from("--packet"),
            OsString::from("-"),
            root.as_os_str().to_os_string(),
        ],
        packet,
    )
}

pub(super) fn run_ast_patch_with_packet_and_path(
    mode: &str,
    root: &std::path::Path,
    packet: &str,
    path_env: &str,
) -> std::process::Output {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .args([
            OsString::from("ast-patch"),
            OsString::from(mode),
            OsString::from("--packet"),
            OsString::from("-"),
            root.as_os_str().to_os_string(),
        ])
        .env("PATH", path_env)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn cli");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(packet.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait cli")
}

pub(super) fn assert_receipt_verification_contains(receipt: &Value, expected: &str) {
    let verification = receipt["verification"].as_array().expect("verification");
    assert!(
        verification.iter().any(|value| value == expected),
        "missing verification {expected}: {verification:?}"
    );
}
