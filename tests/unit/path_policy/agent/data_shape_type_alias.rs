use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn public_semantic_primitive_type_alias_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-type-alias-primitive");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User identifier.\n\
         pub type UserId = String;\n\
         /// Cache toggle.\n\
         pub type CacheEnabled = bool;\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-API-TYPE-ALIAS-027");
    assert_eq!(findings.len(), 2, "{:?}", report.findings);
    assert!(findings[0].summary.contains("`UserId`"));
    assert!(findings[0].summary.contains("String"));
    assert!(findings[1].summary.contains("`CacheEnabled`"));
    assert!(findings[1].summary.contains("bool"));
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_semantic_primitive_type_alias_clears_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-type-alias-primitive");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Primitive alias boundary: generated API keeps raw user identifiers.\n\
         pub type UserId = String;\n\
         /// Primitive alias boundary: generated API keeps raw cache toggles.\n\
         pub type CacheEnabled = bool;\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-TYPE-ALIAS-027").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_newtype_and_domain_alias_clear_primitive_type_alias_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "public-type-alias-newtype");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// User identifier.\n\
         pub struct UserId(String);\n\
         /// Domain-scoped user identifier.\n\
         pub type DomainUserId = UserId;\n\
         #[cfg(test)]\n\
         pub type FixtureId = String;\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-TYPE-ALIAS-027").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
