use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn repeated_namespace_policy_includes_file_stems() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "repeated-file-stem");
    fs::create_dir_all(root.join("src/domain")).expect("create source namespace");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\nmod domain;\n",
    )
    .expect("write branch");
    fs::write(
        root.join("src/domain/domain.rs"),
        "//! Repeated domain namespace.\nfn local() {}\n",
    )
    .expect("write repeated namespace module");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-SOURCE-NAMESPACE-003");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/domain/domain"));
    assert!(
        findings[0]
            .location
            .path
            .as_ref()
            .is_some_and(|path| path.ends_with("src/domain/domain.rs"))
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn branch_module_without_intent_doc_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "branch-without-intent");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain/mod.rs"), "mod parse;\nmod render;\n").expect("write domain");
    fs::write(root.join("src/domain/parse.rs"), "//! Parse leaf.\n").expect("write parse");
    fs::write(root.join("src/domain/render.rs"), "//! Render leaf.\n").expect("write render");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DOCS-BRANCH-008");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("2 resolved child edges"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn branch_intent_counts_resolved_reasoning_tree_edges() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "branch-resolved-children");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain/mod.rs"), "mod parse;\nmod missing;\n").expect("write domain");
    fs::write(root.join("src/domain/parse.rs"), "//! Parse leaf.\n").expect("write parse");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DOCS-BRANCH-008");
    assert!(findings.is_empty(), "{:?}", report.findings);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn generic_source_module_paths_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "generic-source-path");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod helpers;\n").expect("write lib");
    fs::write(root.join("src/helpers.rs"), "//! Helper bucket.\n").expect("write helpers");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-SOURCE-PATH-007");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`helpers`"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn generic_public_module_names_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "generic-public-module");
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha");
    fs::create_dir_all(root.join("src/beta")).expect("create beta");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n/// Shared utility bucket.\npub mod utils;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/utils.rs"), "//! Utility bucket.\n").expect("write utils");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-SOURCE-MODULE-006");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("generic public module `utils`")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_doc_policy_ignores_comment_text_that_mentions_docs() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-doc-comment-text");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owner;\n").expect("write lib");
    fs::write(
        root.join("src/owner.rs"),
        "//! Owner module.\n// /// Pretend doc text.\npub struct MissingDoc;\n",
    )
    .expect("write owner");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DOCS-PUBLIC-002");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`MissingDoc`"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn module_intent_policy_uses_native_inner_doc_attributes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "module-intent-native-doc");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owner;\n").expect("write lib");
    fs::write(
        root.join("src/owner.rs"),
        "// ! Pretend module doc text.\n/// Owner handle.\npub struct Owner;\n",
    )
    .expect("write owner");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DOCS-MODULE-001");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/owner.rs"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn repeated_namespace_policy_covers_default_test_roots() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "repeated-test-root");
    fs::create_dir_all(root.join("tests/unit/unit")).expect("create repeated test namespace");
    fs::write(
        root.join("tests/unit_test.rs"),
        "asp_rust::asp_rust_gate!();\n#[path = \"unit/unit.rs\"]\nmod unit;\n",
    )
    .expect("write test root");
    fs::write(
        root.join("tests/unit/unit.rs"),
        "//! Unit nested root.\nmod helper;\n",
    )
    .expect("write nested test root");
    fs::write(
        root.join("tests/unit/unit/helper.rs"),
        "fn helper_fixture() {}\n",
    )
    .expect("write repeated test helper");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-SOURCE-NAMESPACE-003");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("tests/unit/unit"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn duplicated_public_names_are_reported_as_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "duplicated-public-names");
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha");
    fs::create_dir_all(root.join("src/beta")).expect("create beta");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha/mod.rs"),
        "//! Alpha owner.\nmod types;\npub use types::Handle;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta/mod.rs"),
        "//! Beta owner.\nmod types;\npub use types::Handle;\n",
    )
    .expect("write beta");
    fs::write(
        root.join("src/alpha/types.rs"),
        "//! Alpha types.\n/// Alpha handle.\npub struct Handle;\n",
    )
    .expect("write alpha types");
    fs::write(
        root.join("src/beta/types.rs"),
        "//! Beta types.\n/// Beta handle.\npub struct Handle;\n",
    )
    .expect("write beta types");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert_eq!(
        findings_for_rule(&report, "RUST-AGENT-API-NAME-004").len(),
        2
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn generic_source_module_paths_with_boundary_doc_are_not_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "generic-source-path-boundary-doc");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod helpers;\n").expect("write lib");
    fs::write(
        root.join("src/helpers.rs"),
        "//! Compatibility path boundary.\n",
    )
    .expect("write helpers");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-SOURCE-PATH-007");
    assert_eq!(findings.len(), 0, "{:?}", report.findings);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn duplicated_public_names_with_boundary_doc_are_not_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "duplicated-public-names-boundary-doc");
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha");
    fs::create_dir_all(root.join("src/beta")).expect("create beta");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha/mod.rs"),
        "//! Alpha owner.\nmod types;\n/// Namespace boundary.\npub use types::Handle;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta/mod.rs"),
        "//! Beta owner.\nmod types;\n/// Namespace boundary.\npub use types::Handle;\n",
    )
    .expect("write beta");
    fs::write(
        root.join("src/alpha/types.rs"),
        "//! Alpha types.\n/// Namespace boundary.\npub struct Handle;\n",
    )
    .expect("write alpha types");
    fs::write(
        root.join("src/beta/types.rs"),
        "//! Beta types.\n/// Namespace boundary.\npub struct Handle;\n",
    )
    .expect("write beta types");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-API-NAME-004");
    assert_eq!(findings.len(), 0, "{:?}", report.findings);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn branch_module_with_intent_doc_is_not_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "branch-with-intent");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain module owns parsing and rendering orchestration.\nmod parse;\nmod render;\n",
    )
    .expect("write domain");
    fs::write(root.join("src/domain/parse.rs"), "//! Parse leaf.\n").expect("write parse");
    fs::write(root.join("src/domain/render.rs"), "//! Render leaf.\n").expect("write render");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DOCS-BRANCH-008");
    assert_eq!(findings.len(), 0, "{:?}", report.findings);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_doc_policy_with_real_doc_suppresses_finding() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-doc-real-doc");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owner;\n").expect("write lib");
    fs::write(
        root.join("src/owner.rs"),
        "//! Owner module.\n/// Concrete public DTO.\npub struct HasDoc;\n",
    )
    .expect("write owner");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DOCS-PUBLIC-002");
    assert_eq!(findings.len(), 0, "{:?}", report.findings);
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_primitive_identifier_params_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "primitive-identifier-param");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Loads a user.\n\
         pub fn load_user(user_id: String, count: usize) {}\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-API-TYPE-012");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`load_user`"));
    assert!(findings[0].summary.contains("`user_id`"));
    assert!(findings[0].summary.contains("`String`"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_typed_identifier_params_are_not_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "typed-identifier-param");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User identifier.\n\
         pub struct UserId(String);\n\
         /// Loads a user.\n\
         pub fn load_user(user_id: UserId) {}\n\
         #[cfg(test)]\n\
         pub fn fixture_user(user_id: String) {}\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-TYPE-012").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
