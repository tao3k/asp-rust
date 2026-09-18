use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn public_multiple_flag_params_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-flag-params");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Loads users.\n\
         pub fn load_users(include_inactive: bool, allow_cache: Option<bool>, limit: usize) {}\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-API-FLAGS-018");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`load_users`"));
    assert!(findings[0].summary.contains("include_inactive: bool"));
    assert!(findings[0].summary.contains("allow_cache: Option<bool>"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_multiple_flag_params_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-flag-params");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Flag mode boundary: this bridge mirrors an external query surface.\n\
         pub fn load_users(include_inactive: bool, allow_cache: Option<bool>, limit: usize) {}\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-FLAGS-018").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn single_public_flag_param_is_not_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "single-public-flag-param");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Loads users.\n\
         pub fn load_users(include_inactive: bool, limit: usize) {}\n\
         #[cfg(test)]\n\
         pub fn fixture_users(include_inactive: bool, allow_cache: bool) {}\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-FLAGS-018").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_many_constructor_params_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-many-constructor-params");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Client handle.\n\
         pub struct Client;\n\
         impl Client {\n\
         \t/// Creates a client.\n\
         \tpub fn new(endpoint: String, token: String, retries: usize, timeout_ms: u64, batch_size: usize) -> Self { Self }\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-API-PARAMETERS-019");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`new`"));
    assert!(findings[0].summary.contains("5 positional parameters"));
    assert!(findings[0].summary.contains("endpoint, token"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_many_constructor_params_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-many-constructor-params");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Client handle.\n\
         pub struct Client;\n\
         impl Client {\n\
         \t/// Positional boundary: constructor preserves a generated compatibility bridge.\n\
         \tpub fn new(endpoint: String, token: String, retries: usize, timeout_ms: u64, batch_size: usize) -> Self { Self }\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-PARAMETERS-019").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_config_object_clears_many_param_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-config-object");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Client configuration.\n\
         pub struct ClientConfig;\n\
         /// Client handle.\n\
         pub struct Client;\n\
         impl Client {\n\
         \t/// Creates a client.\n\
         \tpub fn new(config: ClientConfig) -> Self { Self }\n\
         \t#[cfg(test)]\n\
         \tpub fn fixture(endpoint: String, token: String, retries: usize, timeout_ms: u64, batch_size: usize) -> Self { Self }\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-PARAMETERS-019").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
