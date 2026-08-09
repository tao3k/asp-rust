use std::fs;

use serde_json::{Value, json};

use super::{
    assert_receipt_verification_contains, replace_item_packet, run_ast_patch_with_packet,
    write_replace_item_fixture,
};
use crate::cli::support::run_cli_with_stdin;

#[test]
fn cli_ast_patch_dry_run_returns_provider_unsupported_operation_receipt() {
    let packet = json!({
        "target": { "ownerPath": "src/lib.rs", "locator": "src/lib.rs#fn:demo", "read": "src/lib.rs:1:4" },
        "operation": { "op": "remove_statement", "snippet": "return;" }
    })
    .to_string();

    let output = run_cli_with_stdin(["ast-patch", "dry-run", "--packet", "-", "."], &packet);
    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");

    assert_eq!(receipt["status"], "failed");
    assert_eq!(receipt["capability"], "provider-ast-dry-run");
    assert_eq!(receipt["failureKind"], "unsupported-operation");
    assert_eq!(
        receipt["supportedOperations"],
        json!(["replace_item", "split_owner_items"])
    );
    assert_eq!(receipt["operation"], "remove_statement");
    assert_eq!(receipt["target"]["read"], "src/lib.rs:1:4");

    let next = receipt["next"].as_str().unwrap();
    assert!(next.contains("replace_item"));
    assert!(next.contains("split_owner_items"));
    assert!(next.contains("asp ast-patch template --language rust"));
    assert!(next.contains("--owner src/lib.rs"));
    assert!(next.contains("--op split_owner_items"));
    assert!(next.contains("asp rust ast-patch dry-run"));
    assert!(next.contains("asp rust ast-patch apply"));
    assert!(next.contains("asp rust check --changed"));
}

#[test]
fn cli_ast_patch_dry_run_verifies_replace_item_without_mutating_file() {
    let root = write_replace_item_fixture();
    let packet = replace_item_packet("fn demo() -> usize { 2 }");
    let source_path = root.path().join("src/lib.rs");
    let original = fs::read_to_string(&source_path).expect("source before drift");
    fs::write(
        &source_path,
        format!("// inserted before stale locator\n{original}"),
    )
    .expect("write drifted source");
    let before = fs::read_to_string(&source_path).expect("read before dry-run");

    let output = run_ast_patch_with_packet("dry-run", root.path(), &packet);

    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");
    assert_eq!(receipt["status"], "verified");
    assert_eq!(receipt["mode"], "dry-run");
    assert_eq!(receipt["capability"], "provider-ast-dry-run");
    assert_eq!(receipt["mutationAvailable"], false);
    assert_eq!(
        receipt["supportedOperations"],
        json!(["replace_item", "split_owner_items"])
    );
    assert_eq!(receipt["mechanicalEditPlan"]["kind"], "provider-dry-run");
    assert_eq!(
        receipt["mechanicalEditPlan"]["requiresCodexApplyPatch"],
        false
    );
    assert_eq!(receipt["target"]["read"], "src/lib.rs:2:4");
    assert_eq!(
        receipt["mechanicalEditPlan"]["targetRead"],
        "src/lib.rs:2:4"
    );
    assert_receipt_verification_contains(&receipt, "target-locator-stale");
    assert_receipt_verification_contains(&receipt, "target-re-resolved");
    assert_receipt_verification_contains(&receipt, "target-item-parsed");
    assert_receipt_verification_contains(&receipt, "file-reparsed");
    assert_receipt_verification_contains(&receipt, "formatter-output-reparsed");

    let after = fs::read_to_string(source_path).expect("read after dry-run");
    assert_eq!(before, after);
}

#[test]
fn cli_ast_patch_apply_replaces_item_and_formats_file() {
    let root = write_replace_item_fixture();
    let packet = replace_item_packet("pub fn demo() -> usize { 2 }");
    let output = run_ast_patch_with_packet("apply", root.path(), &packet);

    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "successful ast-patch apply should not print a receipt: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let source = fs::read_to_string(root.path().join("src/lib.rs")).expect("source");
    assert!(source.contains("pub fn demo() -> usize {\n    2\n}"));
    assert!(!source.contains("usize { 1 }"));
}
