use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn documented_public_struct_primitive_semantic_fields_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-struct-primitive-fields");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Raw DTO boundary: serialized profiles keep primitive transport fields.\n\
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

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-FIELD-020").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_enum_variant_primitive_payload_fields_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-enum-primitive-payload");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Domain event emitted by the API.\n\
         pub enum DomainEvent {\n\
         \t/// Primitive payload boundary: generated events mirror external payload fields.\n\
         \tUserLoaded { user_id: String, request_id: String, include_inactive: bool },\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-ENUM-PAYLOAD-021").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_enum_tuple_variant_primitive_payload_clears_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-enum-tuple-primitive-payload");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Domain event emitted by the API.\n\
         pub enum DomainEvent {\n\
         \t/// Tuple payload boundary: generated events mirror external payload tuples.\n\
         \tUserLoaded(String, usize, bool),\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-ENUM-TUPLE-024").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_generic_data_type_bounds_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-generic-data-bounds");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Generic bound boundary: public data derives require these trait bounds.\n\
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

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-BOUNDS-022").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
