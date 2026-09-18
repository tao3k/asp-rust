use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use super::support::{has_rule, write_manifest};

#[test]
fn crate_facade_policy_accepts_cfg_contract_macros() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cfg-contract-facade");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nextern crate alloc;\n#[cfg(all(feature = \"fs\", target_family = \"wasm\"))]\ncompile_error!(\"fs is not supported on wasm\");\ncfg_feature! {\n    pub mod optional;\n}\ncfg_macros! {\n    pub use owned::Thing;\n    cfg_inner! {\n        pub(crate) use owned::Other;\n    }\n}\nmod owned;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "//! Owned module.\n").expect("write owned module");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R004"), "{:?}", report.findings);
}

#[test]
fn crate_facade_policy_rejects_macro_wrapped_implementation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "macro-wrapped-implementation");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod owned;\ncfg_bad! {\n    pub fn leaked() {}\n}\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "//! Owned module.\n").expect("write owned module");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(has_rule(&report, "RUST-MOD-R004"), "{:?}", report.findings);
}

#[test]
fn crate_facade_policy_still_rejects_macro_rules_implementation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "macro-rules-facade");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod owned;\nmacro_rules! declare_mod { () => { mod generated; } }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "//! Owned module.\n").expect("write owned module");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(has_rule(&report, "RUST-MOD-R004"), "{:?}", report.findings);
}
