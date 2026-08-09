use std::fs;

use serde_json::{Value, json};

use super::{
    assert_receipt_verification_contains, run_ast_patch_with_packet, split_owner_items_packet,
    split_owner_items_packet_with_max_edits, write_split_owner_fixture,
};

#[test]
fn cli_ast_patch_apply_splits_owner_item_without_agent_hunk() {
    let root = write_split_owner_fixture();
    let packet = split_owner_items_packet();
    let output = run_ast_patch_with_packet("apply", root.path(), &packet);

    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "successful provider-native split should not print a source hunk: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let owner = fs::read_to_string(root.path().join("src/lib.rs")).expect("owner source");
    let destination =
        fs::read_to_string(root.path().join("src/split.rs")).expect("destination source");

    assert!(owner.contains("//! crate docs\n\nmod split;\n"));
    assert!(owner.contains("pub fn keep() -> usize"));
    assert!(!owner.contains("pub fn moved"));
    assert!(destination.contains("pub fn moved<T>(value: T) -> usize"));
    assert!(destination.contains("let text = \"{\";"));
    assert!(destination.contains("format!(\"{text:?}\")"));
}

#[test]
fn cli_ast_patch_dry_run_plans_split_without_mutating_files() {
    let root = write_split_owner_fixture();
    let packet = split_owner_items_packet();
    let source_path = root.path().join("src/lib.rs");
    let before = fs::read_to_string(&source_path).expect("source before dry-run");

    let output = run_ast_patch_with_packet("dry-run", root.path(), &packet);

    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");
    assert_eq!(receipt["status"], "verified");
    assert_eq!(receipt["operation"], "split_owner_items");
    assert_eq!(receipt["mutationSource"], "provider-native");
    assert_eq!(receipt["snippetRequired"], false);
    assert_eq!(receipt["codeInPrompt"], false);
    assert_eq!(
        receipt["mechanicalEditPlan"]["changedPaths"],
        json!(["src/lib.rs", "src/split.rs"])
    );
    assert_eq!(receipt["mechanicalEditPlan"]["estimatedEdits"], 2);
    assert_eq!(receipt["mechanicalEditPlan"]["maxEdits"], 2);
    assert_eq!(
        receipt["mechanicalEditPlan"]["requiresCodexApplyPatch"],
        false
    );
    assert!(
        receipt["mechanicalEditPlan"]["promptBytesAvoided"]
            .as_u64()
            .is_some_and(|value| value > 0)
    );
    assert_receipt_verification_contains(&receipt, "provider-native-operation");
    assert_receipt_verification_contains(&receipt, "formatter-output-reparsed");

    let after = fs::read_to_string(&source_path).expect("source after dry-run");
    assert_eq!(before, after);
    assert!(!root.path().join("src/split.rs").exists());
}

#[test]
fn cli_ast_patch_dry_run_rejects_split_when_max_edits_too_low() {
    let root = write_split_owner_fixture();
    let packet = split_owner_items_packet_with_max_edits(1);

    let output = run_ast_patch_with_packet("dry-run", root.path(), &packet);

    assert!(output.status.success(), "{output:?}");
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt");
    assert_eq!(receipt["status"], "failed");
    assert_eq!(receipt["failureKind"], "target-range-invalid");
    assert!(
        receipt["failures"][0]
            .as_str()
            .is_some_and(|failure| failure.contains("estimated 2 structural edits"))
    );
    assert!(!root.path().join("src/split.rs").exists());
}
