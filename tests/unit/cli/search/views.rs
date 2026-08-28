mod rfc_line_protocol;
use std::fs;
use std::process::Command;

use tempfile::TempDir;

use crate::cli::support::{
    configure_shared_asp_renderer, normalize_temp_root, run_cli, run_search, run_search_with_stdin,
    write_manifest, write_search_fixture,
};

#[test]
fn cli_search_prime_seeds_omit_owner_only_frontier() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "owner-only-prime");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "pub fn load() {}\n").expect("write lib");

    let prime = run_search(root, &["prime", "--view", "seeds"]);

    assert!(
        prime.starts_with("[search-prime] root=. alg=budgeted-prime-frontier-v1"),
        "{prime}"
    );
    assert!(prime.contains("aliases: owner:{O=owner}"), "{prime}");
    assert!(prime.contains("O=owner:path(src/lib.rs)!owner"), "{prime}");
    assert!(
        prime.contains("entries=owner-tests(O=>covering-tests+test-entrypoints+fixtures)"),
        "{prime}"
    );
    assert!(
        !prime.contains("aliases: graph:{G=search,O=owner}"),
        "{prime}"
    );
    assert!(!prime.contains("G>{"), "{prime}");
    assert!(!prime.contains("frontier=O.owner"), "{prime}");
}

#[test]
fn cli_search_from_workspace_member_uses_workspace_root() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/member\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    fs::create_dir_all(root.join("crates/member/src")).expect("create member src");
    fs::write(
        root.join("crates/member/Cargo.toml"),
        "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write member manifest");
    fs::write(
        root.join("crates/member/src/lib.rs"),
        "pub fn member() {}\n",
    )
    .expect("write member source");

    let mut command = Command::new(env!("CARGO_BIN_EXE_asp-rust"));
    configure_shared_asp_renderer(&mut command);
    let output = command
        .current_dir(root.join("crates/member"))
        .args(["search", "workspace"])
        .output()
        .expect("run search workspace");
    assert!(output.status.success(), "{output:?}");
    let rendered = normalize_temp_root(&String::from_utf8(output.stdout).expect("stdout"), root);

    assert!(
        rendered.starts_with("[search-workspace] root=. pkg=1"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "|package crates/member root=crates/member manifest=crates/member/Cargo.toml"
        ),
        "{rendered}"
    );
}

#[test]
fn cli_search_does_not_use_parent_workspace_when_member_is_excluded() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let nested = root.join("languages/orgize");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"languages/*\"]\nexclude = [\"languages/orgize\"]\nresolver = \"2\"\n",
    )
    .expect("write parent workspace manifest");
    fs::create_dir_all(nested.join("src")).expect("create nested src");
    fs::write(
        nested.join("Cargo.toml"),
        "[package]\nname = \"orgize\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write nested manifest");
    fs::write(nested.join("src/lib.rs"), "pub fn orgize() {}\n").expect("write nested source");

    let mut command = Command::new(env!("CARGO_BIN_EXE_asp-rust"));
    configure_shared_asp_renderer(&mut command);
    let output = command
        .current_dir(&nested)
        .args(["search", "workspace"])
        .output()
        .expect("run search workspace");
    assert!(output.status.success(), "{output:?}");
    let rendered = normalize_temp_root(&String::from_utf8(output.stdout).expect("stdout"), &nested);

    assert!(
        rendered.starts_with("[search-workspace] root=. pkg=1"),
        "{rendered}"
    );
    assert!(
        rendered.contains("|package . root=. manifest=Cargo.toml"),
        "{rendered}"
    );
    assert!(!rendered.contains("languages/orgize"), "{rendered}");
}

#[test]
fn cli_search_owner_with_dot_project_root_is_not_duplicated() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let mut command = Command::new(env!("CARGO_BIN_EXE_asp-rust"));
    configure_shared_asp_renderer(&mut command);
    let output = command
        .current_dir(root)
        .args([
            "search",
            "owner",
            "src/lib.rs",
            "items",
            "--query",
            "fixture",
            ".",
        ])
        .output()
        .expect("run search owner");
    assert!(output.status.success(), "{output:?}");
    let rendered = normalize_temp_root(&String::from_utf8(output.stdout).expect("stdout"), root);

    assert_eq!(rendered.matches("[search-owner]").count(), 1, "{rendered}");
}

#[test]
fn cli_search_ingest_accepts_workspace_relative_scope_argument() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path().join("fixture");
    fs::create_dir_all(&root).expect("create fixture root");
    write_search_fixture(&root);

    let mut command = Command::new(env!("CARGO_BIN_EXE_asp-rust"));
    configure_shared_asp_renderer(&mut command);
    let output = command
        .current_dir(temp.path())
        .args([
            "search",
            "ingest",
            "items",
            "tests",
            "--view",
            "seeds",
            "--workspace",
        ])
        .arg(&root)
        .arg(".")
        .output()
        .expect("run search ingest");

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert!(stdout.starts_with("[search-ingest]"), "{stdout}");
    assert!(stdout.contains(" scope=."), "{stdout}");
}
