use std::fs;
use std::path::Path;

use asp_rust::{render_asp_rust, run_asp_rust_for_scope};
use tempfile::TempDir;

#[test]
fn mod_r001_interface_mod_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r001-interface");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain interface.\nmod leaf { fn helper() {} }\n",
    )
    .expect("write mod");

    assert_mod_snapshot(root, "RUST-MOD-R001", "rust_mod_r001_mod_rs_interface");
}

#[test]
fn mod_r002_source_bloat_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r002-bloat");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod pile;\n").expect("write lib");
    fs::write(root.join("src/pile.rs"), private_implementation_pile()).expect("write pile");

    assert_mod_snapshot(root, "RUST-MOD-R002", "rust_mod_r002_source_bloat");
}

#[test]
fn mod_r003_deep_relative_import_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r003-deep-relative");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\npub use super::{super::MissingOwner, sibling::Thing};\n",
    )
    .expect("write domain");

    assert_mod_snapshot(root, "RUST-MOD-R003", "rust_mod_r003_deep_relative_import");
}

#[test]
fn mod_r004_lib_facade_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r004-lib-facade");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod owned;\nmacro_rules! local_macro { () => {} }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "//! Owned module.\n").expect("write owned");

    assert_mod_snapshot(root, "RUST-MOD-R004", "rust_mod_r004_lib_facade");
}

#[test]
fn mod_r005_binary_entrypoint_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r005-binary-entrypoint");
    fs::create_dir_all(root.join("src/bin")).expect("create bin");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(
        root.join("src/bin/tool.rs"),
        "//! Tool entrypoint.\nstruct CliOptions;\nfn main() {}\n",
    )
    .expect("write bin");

    assert_mod_snapshot(root, "RUST-MOD-R005", "rust_mod_r005_binary_entrypoint");
}

#[test]
fn mod_r006_build_script_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r006-build-script");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(
        root.join("build.rs"),
        "use std::path::Path;\nfn helper() {}\nfn main() {}\n",
    )
    .expect("write build");

    assert_mod_snapshot(root, "RUST-MOD-R006", "rust_mod_r006_build_script");
}

#[test]
fn mod_r007_module_source_shadow_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r007-source-shadow");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain.rs"), "//! Domain owner.\n").expect("write file form");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain directory owner.\n",
    )
    .expect("write mod form");

    assert_mod_snapshot(root, "RUST-MOD-R007", "rust_mod_r007_module_source_shadow");
}

#[test]
fn mod_r008_inline_source_module_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r008-inline-source");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain owner.\nmod leaf { fn helper() {} }\n",
    )
    .expect("write domain");

    assert_mod_snapshot(root, "RUST-MOD-R008", "rust_mod_r008_inline_source_module");
}

#[test]
fn mod_r009_orphan_source_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r009-orphan-source");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(root.join("src/forgotten.rs"), "//! Forgotten owner.\n").expect("write orphan");

    assert_mod_snapshot(root, "RUST-MOD-R009", "rust_mod_r009_orphan_source");
}

#[test]
fn mod_r010_glob_import_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r010-glob-import");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\nuse super::*;\nfn local() {}\n",
    )
    .expect("write domain");

    assert_mod_snapshot(root, "RUST-MOD-R010", "rust_mod_r010_glob_import");
}

#[test]
fn mod_r010_absolute_owner_glob_import_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "mod-r010-absolute-owner-glob-import");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain module.\nuse crate::gateway::studio::studio_repo_sync_api_tests::*;\n",
    )
    .expect("write domain");

    assert_mod_snapshot(
        root,
        "RUST-MOD-R010",
        "rust_mod_r010_absolute_owner_glob_import",
    );
}

fn assert_mod_snapshot(root: &Path, rule_id: &str, snapshot_name: &'static str) {
    let mut report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");
    report.findings.retain(|finding| finding.rule_id == rule_id);
    assert_eq!(
        report.findings.len(),
        1,
        "expected one {rule_id} finding, got {:?}",
        report.findings
    );
    let rendered = normalize_temp_root(&render_asp_rust(&report), root);
    insta::assert_snapshot!(snapshot_name, rendered);
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}

fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

fn private_implementation_pile() -> String {
    let mut source = String::from("//! Private implementation pile.\n");
    for index in 0..45 {
        source.push_str(&format!(
            "fn helper_{index}() -> usize {{\n  let mut total = {index};\n  total += 1;\n  total += 2;\n  total += 3;\n  total += 4;\n  total += 5;\n  total += 6;\n  total += 7;\n  total += 8;\n  total += 9;\n  total += 10;\n  total += 11;\n  total += 12;\n  total += 13;\n  total\n}}\n"
        ));
    }
    source
}
