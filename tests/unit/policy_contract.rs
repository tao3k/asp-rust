use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use asp_rust::{
    RustDiagnosticSeverity, default_asp_rust_config, render_asp_rust, rust_agent_policy_rules,
};

#[path = "policy_contract/parser.rs"]
mod parser;
#[path = "policy_contract/reasoning_tree.rs"]
mod reasoning_tree;

#[test]
fn default_policy_blocks_only_warning_and_error() {
    let config = default_asp_rust_config();

    assert_eq!(
        config.blocking_severities,
        BTreeSet::from([
            RustDiagnosticSeverity::Warning,
            RustDiagnosticSeverity::Error,
        ])
    );
}

#[test]
fn agent_policy_rules_are_non_blocking_advice() {
    for rule in rust_agent_policy_rules() {
        assert_eq!(
            rule.severity,
            RustDiagnosticSeverity::Info,
            "{}",
            rule.rule_id
        );
    }
}

#[test]
fn crate_is_clean_under_its_own_project_harness() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut config = asp_rust::default_asp_rust_config();
    config.ignored_dir_names.insert("scenarios".to_string());
    let report = asp_rust::run_asp_rust_with_config_for_scope(
        &root,
        &config,
        asp_rust::AspRustRunScope::Package,
    )
    .expect("run self harness");
    let rendered = render_asp_rust(&report);

    assert!(report.is_clean(), "{rendered}");
    assert_eq!(rendered, "[ok] rust\n");
}

#[test]
fn external_test_target_owns_the_full_self_apply_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = fs::read_to_string(root.join("src/lib.rs")).expect("read src/lib.rs");
    let self_policy = fs::read_to_string(root.join("tests/unit/self_policy.rs"))
        .expect("read tests/unit/self_policy.rs");
    let unit_test =
        fs::read_to_string(root.join("tests/unit_test.rs")).expect("read tests/unit_test.rs");

    assert!(!lib_rs.contains("asp_rust_source_gate!"));
    assert!(self_policy.contains("assert_asp_rust_cargo_test_clean_with_config"));
    assert!(unit_test.contains("#[path = \"unit/self_policy.rs\"]"));
}

#[test]
fn crate_facade_keeps_macro_implementation_out_of_lib_rs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = fs::read_to_string(root.join("src/lib.rs")).expect("read src/lib.rs");
    let macros_rs = fs::read_to_string(root.join("src/macros.rs")).expect("read src/macros.rs");

    assert!(!lib_rs.contains("macro_rules!"));
    assert!(macros_rs.contains("macro_rules! asp_rust_gate"));
    assert!(macros_rs.contains("macro_rules! asp_rust_cargo_test_gate"));
}

#[test]
fn root_test_target_relies_on_source_backed_project_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let unit_test =
        fs::read_to_string(root.join("tests/unit_test.rs")).expect("read tests/unit_test.rs");

    assert!(!unit_test.contains("asp_rust_gate!()"));
    assert!(unit_test.contains("#[path = \"unit/policy_contract.rs\"]"));
}
