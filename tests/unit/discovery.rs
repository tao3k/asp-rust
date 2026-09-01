use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::discovery::{
    asp_rust_scope, cargo_package_exclusion_roots, discover_cargo_package_roots,
    discover_rust_files, glob_pattern_matches,
};
#[cfg(feature = "provider-server")]
use crate::parser::{cargo_package_root_for_path, cargo_project_root_for_path};
use crate::runner::asp_rust_config_for_project;

#[test]
fn workspace_member_glob_matches_forward_slash_relative_paths() {
    assert!(glob_pattern_matches("crates/*", "crates/member"));
    assert!(!glob_pattern_matches("crates/*", "crates/member/nested"));
}

#[test]
fn workspace_member_glob_matches_windows_relative_paths() {
    assert!(glob_pattern_matches("crates/*", r"crates\member"));
    assert!(!glob_pattern_matches("crates/*", r"crates\member\nested"));
}

#[test]
fn project_resolution_is_anchored_to_cargo_manifest_targets() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"cargo-anchored\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"custom/lib.rs\"\n\n[[bin]]\nname = \"worker\"\npath = \"tools/worker.rs\"\n\n[[test]]\nname = \"contract\"\npath = \"contracts/api.rs\"\n\n[[example]]\nname = \"demo\"\npath = \"demo/example.rs\"\n\n[[bench]]\nname = \"throughput\"\npath = \"perf/throughput.rs\"\nharness = false\n",
    )
    .expect("write manifest");
    write_file(root, "src/lib.rs");
    write_file(root, "tests/smoke.rs");
    write_file(root, "custom/lib.rs");
    write_file(root, "tools/worker.rs");
    write_file(root, "contracts/api.rs");
    write_file(root, "demo/example.rs");
    write_file(root, "perf/throughput.rs");

    let scope = asp_rust_scope(root, true, &[], &[]);
    let source_paths = path_set(root, &scope.source_paths);
    let test_paths = path_set(root, &scope.test_paths);
    let package_paths = path_set(root, &scope.package_paths);

    assert!(source_paths.contains("src"));
    assert!(source_paths.contains("custom"));
    assert!(source_paths.contains("tools"));
    assert!(test_paths.contains("tests"));
    assert!(test_paths.contains("contracts"));
    assert!(package_paths.contains("demo/example.rs"));
    assert!(package_paths.contains("perf/throughput.rs"));
}

#[cfg(feature = "provider-server")]
#[test]
fn explicit_package_root_does_not_promote_to_parent_cargo_workspace() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path();
    fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"member\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    let package = workspace.join("member");
    fs::create_dir_all(package.join("src")).expect("create package");
    fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write package manifest");
    write_file(&package, "src/lib.rs");

    assert_eq!(
        cargo_package_root_for_path(&package).expect("explicit package root"),
        package.canonicalize().expect("canonical package")
    );
    assert_eq!(
        cargo_project_root_for_path(&package).expect("workspace project root"),
        workspace.canonicalize().expect("canonical workspace")
    );
}

#[test]
fn workspace_path_dependencies_are_discovered_as_package_roots() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/hook\"]\n\n[workspace.dependencies]\nasp-rust = { path = \"languages/asp-rust\", default-features = false }\n",
    )
    .expect("write root manifest");
    fs::create_dir_all(root.join("crates/hook")).expect("create hook crate");
    fs::write(
        root.join("crates/hook/Cargo.toml"),
        "[package]\nname = \"hook\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nasp-rust = { workspace = true }\n",
    )
    .expect("write hook manifest");
    fs::create_dir_all(root.join("languages/asp-rust")).expect("create harness crate");
    fs::write(
        root.join("languages/asp-rust/Cargo.toml"),
        "[package]\nname = \"asp-rust\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write harness manifest");
    write_file(root, "crates/hook/src/lib.rs");
    write_file(root, "languages/asp-rust/src/lib.rs");

    let package_roots = discover_cargo_package_roots(root, &BTreeSet::new(), &BTreeSet::new());
    let package_roots = path_set(root, &package_roots);

    assert!(package_roots.contains("crates/hook"));
    assert!(package_roots.contains("languages/asp-rust"));
}

#[test]
fn package_exclusions_come_from_manifest_graph_not_nested_manifest_guessing() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='root'\nversion='0.1.0'\n\
         [workspace]\nmembers=['declared']\n\
         [dependencies]\nowned-path={ path='owned-path' }\n",
    )
    .expect("write root manifest");
    for nested in ["declared", "owned-path", "unregistered"] {
        fs::create_dir_all(root.join(nested)).expect("create nested package");
        fs::write(
            root.join(nested).join("Cargo.toml"),
            format!("[package]\nname='{nested}'\nversion='0.1.0'\n"),
        )
        .expect("write nested manifest");
    }

    let exclusions = cargo_package_exclusion_roots(root, &BTreeSet::new(), &BTreeSet::new());

    assert!(exclusions.contains(&root.join("declared")));
    assert!(exclusions.contains(&root.join("owned-path")));
    assert!(
        !exclusions.contains(&root.join("unregistered")),
        "an arbitrary nested Cargo.toml must not invent a package-graph edge"
    );
}

#[test]
fn nested_manifests_without_an_anchor_manifest_do_not_create_a_package_graph() {
    let temp = TempDir::new().expect("temp dir");
    let nested = temp.path().join("nested");
    fs::create_dir_all(&nested).expect("create nested package");
    fs::write(
        nested.join("Cargo.toml"),
        "[package]\nname='nested'\nversion='0.1.0'\n",
    )
    .expect("write nested manifest");

    let roots = discover_cargo_package_roots(temp.path(), &BTreeSet::new(), &BTreeSet::new());

    assert!(
        roots.is_empty(),
        "a recursive filesystem walk invented package owners: {roots:?}"
    );
}

#[test]
fn hidden_directories_are_ignored_by_default() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_file(root, "src/lib.rs");
    write_file(root, ".devenv/generated.rs");

    let files = discover_rust_files(&[root.to_path_buf()], &BTreeSet::new(), &BTreeSet::new());
    let files = path_set(root, &files);

    assert!(files.contains("src/lib.rs"));
    assert!(!files.contains(".devenv/generated.rs"));
}

#[test]
fn asp_toml_can_include_a_hidden_directory() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("asp.toml"),
        "[discovery]\nincludeHiddenDirNames = [\".agent-fixtures\"]\n",
    )
    .expect("write asp config");
    write_file(root, "src/lib.rs");
    write_file(root, ".agent-fixtures/generated.rs");

    let config = asp_rust_config_for_project(root);
    let files = discover_rust_files(
        &[root.to_path_buf()],
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    );
    let files = path_set(root, &files);

    assert!(files.contains("src/lib.rs"));
    assert!(files.contains(".agent-fixtures/generated.rs"));
}

#[cfg(unix)]
#[test]
fn symlinked_rust_file_is_not_discovered() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path().join("project");
    let target = temp.path().join("external.rs");
    write_file(&root, "src/lib.rs");
    fs::write(&target, "pub fn external() {}\n").expect("write symlink target");
    std::os::unix::fs::symlink(&target, root.join("src/external.rs"))
        .expect("create Rust file symlink");

    let files = discover_rust_files(
        std::slice::from_ref(&root),
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    let files = path_set(&root, &files);

    assert!(files.contains("src/lib.rs"));
    assert!(!files.contains("src/external.rs"));
}

#[cfg(unix)]
#[test]
fn symlinked_directory_is_not_discovered() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path().join("project");
    let target = temp.path().join("external");
    write_file(&root, "src/lib.rs");
    write_file(&target, "generated.rs");
    std::os::unix::fs::symlink(&target, root.join("linked"))
        .expect("create Rust directory symlink");

    let files = discover_rust_files(
        std::slice::from_ref(&root),
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    let files = path_set(&root, &files);

    assert!(files.contains("src/lib.rs"));
    assert!(!files.contains("linked/generated.rs"));
}

fn write_file(root: &Path, relative_path: &str) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, "//! Fixture.\n").expect("write file");
}

fn path_set(root: &Path, paths: &[PathBuf]) -> BTreeSet<String> {
    paths
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .expect("project relative path")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}
