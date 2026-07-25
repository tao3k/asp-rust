use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::cli::support::{run_cli, write_source_snapshot_envelope};
mod code;

#[test]
fn cli_query_hook_selector_follows_workspace_path_dependency_roots() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/hook\"]\n\n[workspace.dependencies]\nrust-lang-project-harness = { path = \"languages/rust-lang-project-harness\", default-features = false }\n",
    )
    .expect("write root manifest");
    fs::create_dir_all(root.join("crates/hook")).expect("create hook crate");
    fs::write(
        root.join("crates/hook/Cargo.toml"),
        "[package]\nname = \"hook\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nrust-lang-project-harness = { workspace = true }\n",
    )
    .expect("write hook manifest");
    fs::create_dir_all(root.join("languages/rust-lang-project-harness"))
        .expect("create harness crate");
    fs::write(
        root.join("languages/rust-lang-project-harness/Cargo.toml"),
        "[package]\nname = \"rust-lang-project-harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write harness manifest");
    fs::create_dir_all(root.join("crates/hook/src")).expect("create hook src");
    fs::write(root.join("crates/hook/src/lib.rs"), "pub fn hook() {}\n")
        .expect("write hook source");
    fs::create_dir_all(root.join("languages/rust-lang-project-harness/src"))
        .expect("create harness src");
    let harness_source = "pub fn harness() {}\n";
    fs::write(
        root.join("languages/rust-lang-project-harness/src/lib.rs"),
        harness_source,
    )
    .expect("write harness source");
    let envelope = write_source_snapshot_envelope(
        root,
        "rs-harness-test",
        &[(
            "languages/rust-lang-project-harness/src/lib.rs",
            harness_source,
        )],
    );
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "rust://languages/rust-lang-project-harness/src/lib.rs#item/function/harness".as_ref(),
        "--source-snapshot-envelope".as_ref(),
        envelope.as_os_str(),
        "--workspace".as_ref(),
        root.as_os_str(),
        "--code".as_ref(),
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("query code is UTF-8")
            .trim(),
        "pub fn harness() {}"
    );
}

#[test]
fn cli_query_code_output_uses_canonical_package_relative_selector() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let owner = "tests/unit/cli/support.rs";
    let source = fs::read_to_string(root.join(owner)).expect("read support source");
    let temp = TempDir::new().expect("snapshot temp dir");
    let envelope =
        write_source_snapshot_envelope(temp.path(), "rs-harness-test", &[(owner, &source)]);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "rust://tests/unit/cli/support.rs#item/function/run_search".as_ref(),
        "--source-snapshot-envelope".as_ref(),
        envelope.as_os_str(),
        "--code".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("fn run_search"), "{stdout}");
    assert!(stdout.contains("command_args.push"), "{stdout}");
    assert!(stdout.contains("command_args.extend"), "{stdout}");
    assert!(stdout.contains("normalize_temp_root"), "{stdout}");
}
