use std::fs;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::{
    assert_receipt_verification_contains, replace_item_packet, run_ast_patch_with_packet,
    run_ast_patch_with_packet_and_path, write_replace_item_fixture,
};
use crate::cli::support::write_manifest;

#[test]
fn cli_ast_patch_apply_does_not_write_when_rustfmt_fails() {
    let root = write_replace_item_fixture();
    let packet = replace_item_packet("pub fn demo() -> usize { 2 }");
    let source_path = root.path().join("src/lib.rs");
    let before = fs::read_to_string(&source_path).expect("read before");

    let output = run_ast_patch_with_packet_and_path("apply", root.path(), &packet, "");
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt JSON");

    assert_eq!(receipt["status"], "failed");
    assert_eq!(receipt["failureKind"], "formatter-failed");
    assert_receipt_verification_contains(&receipt, "file-reparsed");

    let after = fs::read_to_string(source_path).expect("read after");
    assert_eq!(before, after);
}

#[test]
fn cli_ast_patch_apply_rejects_append_to_existing_module_file() {
    let root = TempDir::new().expect("tempdir");
    write_manifest(root.path(), "ast-patch-append-existing");
    fs::create_dir_all(root.path().join("src")).expect("mkdir src");
    fs::write(root.path().join("src/mod.rs"), "").expect("write mod");
    let packet = json!({
        "target": { "ownerPath": "src/mod.rs", "locator": "src/mod.rs#module-root", "read": "src/mod.rs:1:1" },
        "operation": { "op": "append_to_block", "snippet": "pub mod read;\n", "maxEdits": 1 }
    })
    .to_string();

    let output = run_ast_patch_with_packet("apply", root.path(), &packet);
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");

    assert_eq!(receipt["status"], "failed", "{receipt}");
    assert_eq!(receipt["failureKind"], "unsupported-operation");
    assert_eq!(receipt["operation"], "append_to_block");
    assert_eq!(
        receipt["supportedOperations"],
        json!(["replace_item", "split_owner_items"])
    );
    assert!(receipt["mechanicalEditPlan"].is_null());
    let source = fs::read_to_string(root.path().join("src/mod.rs")).expect("source");
    assert_eq!(source, "");
}

#[test]
fn cli_ast_patch_apply_rejects_append_to_missing_file() {
    let root = TempDir::new().expect("tempdir");
    write_manifest(root.path(), "ast-patch-append-missing-file");
    fs::create_dir_all(root.path().join("src")).expect("mkdir src");
    let packet = json!({
        "target": { "ownerPath": "src/generated.rs", "locator": "src/generated.rs#module-root", "read": "src/generated.rs:1:1" },
        "operation": { "op": "append_to_block", "snippet": "pub fn generated_marker() -> usize { 1 }\n", "maxEdits": 1 }
    })
    .to_string();

    let output = run_ast_patch_with_packet("apply", root.path(), &packet);
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");

    assert_eq!(receipt["status"], "failed", "{receipt}");
    assert_eq!(receipt["failureKind"], "unsupported-operation");
    assert_eq!(receipt["operation"], "append_to_block");
    assert_eq!(
        receipt["supportedOperations"],
        json!(["replace_item", "split_owner_items"])
    );
    assert!(receipt["mechanicalEditPlan"].is_null());
    assert!(!root.path().join("src/generated.rs").exists());
}
