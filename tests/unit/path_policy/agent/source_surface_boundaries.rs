use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn documented_generic_source_module_paths_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-generic-source-path");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod helpers;\n").expect("write lib");
    fs::write(
        root.join("src/helpers.rs"),
        "//! Compatibility path boundary: this module preserves a generated bridge path.\n",
    )
    .expect("write helpers");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-SOURCE-PATH-007").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_name_conflicts_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-public-name-conflicts");
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha");
    fs::create_dir_all(root.join("src/beta")).expect("create beta");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha/mod.rs"),
        "//! Alpha owner.\nmod types;\n/// Namespace boundary: this handle is scoped by the alpha module.\npub use types::Handle;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta/mod.rs"),
        "//! Beta owner.\nmod types;\n/// Namespace boundary: this handle is scoped by the beta module.\npub use types::Handle;\n",
    )
    .expect("write beta");
    fs::write(
        root.join("src/alpha/types.rs"),
        "//! Alpha types.\n/// Namespace boundary: this handle is scoped by the alpha module.\npub struct Handle;\n",
    )
    .expect("write alpha types");
    fs::write(
        root.join("src/beta/types.rs"),
        "//! Beta types.\n/// Namespace boundary: this handle is scoped by the beta module.\npub struct Handle;\n",
    )
    .expect("write beta types");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-NAME-004").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn documented_public_primitive_identifier_params_clear_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "documented-primitive-identifier-param");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Primitive boundary: this generated bridge keeps raw external identifiers.\n\
         pub fn load_user(user_id: String, count: usize) {}\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-API-TYPE-012").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}
