use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use super::support::{findings_for_rule, write_manifest};

#[test]
fn sibling_file_dir_owner_policy_rejects_split_owner_entrypoint() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "sibling-file-dir-owner");
    fs::create_dir_all(root.join("src/search/cache")).expect("create search cache");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod search;\n").expect("write lib");
    fs::write(
        root.join("src/search.rs"),
        "//! Search owner.\nmod cache;\npub fn search() {}\n",
    )
    .expect("write search file");
    fs::write(root.join("src/search/cache.rs"), "//! Cache owner.\n").expect("write cache file");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-MOD-R011");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("src/search.rs"));
    assert!(findings[0].summary.contains("src/search/"));
    assert_eq!(
        findings[0].label,
        "move the owner interface to mod.rs under the directory"
    );
}
