use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn public_struct_primitive_semantic_fields_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-struct-primitive-fields");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User profile crossing a public boundary.\n\
         pub struct UserProfile {\n\
         \tpub user_id: String,\n\
         \tpub session_token: String,\n\
         \tpub timeout_ms: u64,\n\
         \tpub include_inactive: bool,\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DATA-FIELD-020");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`UserProfile`"));
    assert!(findings[0].summary.contains("user_id: String"));
    assert!(findings[0].summary.contains("timeout_ms: u64"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_struct_typed_fields_clear_primitive_data_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-struct-typed-fields");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User identifier.\n\
         pub struct UserId(String);\n\
         /// Session token.\n\
         pub struct SessionToken(String);\n\
         /// Timeout in milliseconds.\n\
         pub struct TimeoutMs(u64);\n\
         /// User profile crossing a public boundary.\n\
         pub struct UserProfile {\n\
         \tpub user_id: UserId,\n\
         \tpub session_token: SessionToken,\n\
         \tpub timeout_ms: TimeoutMs,\n\
         }\n\
         #[cfg(test)]\n\
         pub struct FixtureProfile { pub user_id: String, pub token: String, pub timeout_ms: u64 }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-FIELD-020").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_enum_variant_primitive_payload_fields_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-enum-primitive-payload");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Domain event emitted by the API.\n\
         pub enum DomainEvent {\n\
         \t/// User data was loaded.\n\
         \tUserLoaded { user_id: String, request_id: String, include_inactive: bool },\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DATA-ENUM-PAYLOAD-021");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`DomainEvent`"));
    assert!(findings[0].summary.contains("`UserLoaded`"));
    assert!(findings[0].summary.contains("user_id: String"));
    assert!(findings[0].summary.contains("request_id: String"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_enum_variant_typed_payload_clears_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-enum-typed-payload");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User identifier.\n\
         pub struct UserId(String);\n\
         /// Request identifier.\n\
         pub struct RequestId(String);\n\
         /// Domain event emitted by the API.\n\
         pub enum DomainEvent {\n\
         \t/// User data was loaded.\n\
         \tUserLoaded { user_id: UserId, request_id: RequestId },\n\
         \t#[cfg(test)]\n\
         \tFixture { user_id: String, request_id: String },\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-ENUM-PAYLOAD-021").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_enum_tuple_variant_primitive_payload_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-enum-tuple-primitive-payload");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Domain event emitted by the API.\n\
         pub enum DomainEvent {\n\
         \t/// User data was loaded.\n\
         \tUserLoaded(String, usize, bool),\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DATA-ENUM-TUPLE-024");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`DomainEvent`"));
    assert!(findings[0].summary.contains("tuple variant `UserLoaded`"));
    assert!(findings[0].summary.contains("#1: String"));
    assert!(findings[0].summary.contains("#3: bool"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_enum_named_or_typed_tuple_payload_clears_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-enum-typed-tuple-payload");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User identifier.\n\
         pub struct UserId(String);\n\
         /// User count.\n\
         pub struct UserCount(usize);\n\
         /// Domain event emitted by the API.\n\
         pub enum DomainEvent {\n\
         \t/// User data was loaded.\n\
         \tUserLoaded(UserId, UserCount),\n\
         \t#[cfg(test)]\n\
         \tFixture(String, bool),\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-ENUM-TUPLE-024").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_generic_data_type_bounds_are_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-generic-data-bounds");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Generic cache entry.\n\
         pub struct CacheEntry<T: Clone + std::fmt::Debug>\n\
         where\n\
         \tT: Default,\n\
         {\n\
         \tpub value: T,\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DATA-BOUNDS-022");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`CacheEntry`"));
    assert!(findings[0].summary.contains("T: Clone"));
    assert!(findings[0].summary.contains("T: Debug"));
    assert!(findings[0].summary.contains("T: Default"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn generic_method_bounds_clear_public_data_type_bound_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "generic-method-bounds");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Generic cache entry.\n\
         pub struct CacheEntry<T> {\n\
         \tpub value: T,\n\
         }\n\
         impl<T> CacheEntry<T>\n\
         where\n\
         \tT: Clone + std::fmt::Debug,\n\
         {\n\
         \t/// Clones the value for diagnostics.\n\
         \tpub fn clone_for_debug(&self) -> T { self.value.clone() }\n\
         }\n\
         #[cfg(test)]\n\
         pub struct Fixture<T: Clone> { pub value: T }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-BOUNDS-022").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
