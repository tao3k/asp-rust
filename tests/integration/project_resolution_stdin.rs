use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture parent");
    }
    std::fs::write(path, contents).expect("write fixture file");
}

fn run_project_resolution(root: &Path, candidates: &[&str]) -> std::process::Output {
    let request = json!({
        "schemaId": "agent.semantic-protocols.provider-project-resolution-request",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "asp-rust",
        "candidateBase": ".",
        "candidateGeneration": {
            "algorithm": "blake3-worktree-state-v1",
            "digest": format!("blake3:{}", "a".repeat(64)),
            "authorities": ["asp-workspace-generation"]
        },
        "collectionScope": { "kind": "complete-generation" },
        "candidatePaths": candidates,
        "policyExclusions": []
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .arg("project-resolution-stdin")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rs-harness");
    child
        .stdin
        .take()
        .expect("project-resolution stdin")
        .write_all(serde_json::to_string(&request).unwrap().as_bytes())
        .expect("write request");
    child.wait_with_output().expect("wait for rs-harness")
}

#[test]
fn typed_project_resolution_stdin_returns_package_graph_via_cli() {
    let fixture = tempfile::tempdir().expect("temporary workspace");
    write(
        fixture.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(fixture.path(), "src/lib.rs", "pub fn demo() {}\n");

    let output = run_project_resolution(fixture.path(), &["Cargo.toml", "src/lib.rs"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).expect("response JSON");
    assert_eq!(response["state"], "resolved");
    assert!(
        response.get("scope").is_some(),
        "resolved response must carry the provider-owned scope"
    );
    assert_eq!(
        response["scope"]["packageGraph"]["packages"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        response["scope"]["packageGraph"]["parserId"],
        "rust.cargo-toml"
    );
    assert_eq!(response["scope"]["metrics"]["fullWorkspaceReads"], 0);
    assert_eq!(response["scope"]["metrics"]["dbOpens"], 0);
}

#[test]
fn nested_project_resolution_consumes_asp_rebased_candidates() {
    let repository = tempfile::tempdir().expect("temporary repository");
    let project_root = repository.path().join("crates/leaf");
    write(
        repository.path(),
        "crates/leaf/Cargo.toml",
        "[package]\nname = \"leaf\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        repository.path(),
        "crates/leaf/src/lib.rs",
        "pub fn leaf() {}\n",
    );

    let admitted = run_project_resolution(&project_root, &["Cargo.toml", "src/lib.rs"]);
    assert!(
        admitted.status.success(),
        "{}",
        String::from_utf8_lossy(&admitted.stderr)
    );
}

#[test]
fn typed_project_resolution_stdin_fails_closed_without_root_entry() {
    let fixture = tempfile::tempdir().expect("temporary workspace");
    write(
        fixture.path(),
        "nested/Cargo.toml",
        "[package]\nname = \"nested\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );

    let output = run_project_resolution(fixture.path(), &["nested/Cargo.toml"]);

    assert_eq!(output.status.code(), Some(2));
    let response: Value = serde_json::from_slice(&output.stdout).expect("response JSON");
    assert_eq!(response["state"], "failed");
    assert!(
        response.get("scope").is_none(),
        "failed response must not carry a scope"
    );
    assert_eq!(response["failure"]["reasonKind"], "project-entry-missing");
    assert_eq!(
        response["failure"]["nextAction"],
        "refresh-project-resolution-candidates-or-select-project-entry"
    );
}
fn resolution_json(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "project resolution failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("typed project-resolution response")
}

#[test]
fn cargo_workspace_members_targets_and_path_dependencies_define_source_scopes() {
    let root = std::env::temp_dir().join(format!(
        "asp-cargo-project-resolution-{}",
        std::process::id()
    ));
    write(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/core\", \"crates/app\"]\nresolver = \"3\"\n",
    );
    write(
        &root,
        "crates/core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[lib]\npath = \"source/core.rs\"\n",
    );
    write(
        &root,
        "crates/core/source/core.rs",
        "pub fn core_value() -> u8 { 1 }\n",
    );
    write(
        &root,
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[[bin]]\nname = \"app\"\npath = \"cmd/main.rs\"\n[dependencies]\ncore = { path = \"../core\" }\n",
    );
    write(&root, "crates/app/cmd/main.rs", "fn main() {}\n");
    write(
        &root,
        "crates/not-a-candidate/Cargo.toml",
        "[package]\nname = \"not-a-candidate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &root,
        "crates/not-a-candidate/src/lib.rs",
        "pub fn must_not_enter_scope() {}\n",
    );
    let output = run_project_resolution(
        &root,
        &[
            "Cargo.toml",
            "crates/core/Cargo.toml",
            "crates/core/source/core.rs",
            "crates/app/Cargo.toml",
            "crates/app/cmd/main.rs",
        ],
    );
    let packet = resolution_json(&output);
    let scope = &packet["scope"];
    assert_eq!(
        scope["packageGraph"]["packages"].as_array().unwrap().len(),
        2
    );
    assert!(
        scope["packageGraph"]["packages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|package| package["name"] != "not-a-candidate")
    );
    let scope_roots: Vec<&str> = scope["sourceScopes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|scope| scope["roots"].as_array().unwrap())
        .map(|path| path.as_str().unwrap())
        .collect();
    assert!(scope_roots.contains(&"crates/core/source"));
    assert!(scope_roots.contains(&"crates/app/cmd"));
    let explicit_paths: Vec<&str> = scope["sourceScopes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|scope| scope["explicitPaths"].as_array().unwrap())
        .map(|path| path.as_str().unwrap())
        .collect();
    assert!(explicit_paths.contains(&"crates/core/source/core.rs"));
    assert!(explicit_paths.contains(&"crates/app/cmd/main.rs"));
    assert_eq!(scope["metrics"]["fullWorkspaceReads"], 0);
    assert_eq!(scope["metrics"]["dbOpens"], 0);
}

#[test]
fn cargo_manifest_delta_changes_resolution_without_root_scan() {
    let root =
        std::env::temp_dir().join(format!("asp-cargo-manifest-delta-{}", std::process::id()));
    write(
        &root,
        "Cargo.toml",
        "[package]\nname = \"delta\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&root, "src/lib.rs", "pub fn first() {}\n");
    write(&root, "generated/entry.rs", "pub fn second() {}\n");
    let candidates = ["Cargo.toml", "src/lib.rs", "generated/entry.rs"];
    let before = resolution_json(&run_project_resolution(&root, &candidates));

    write(
        &root,
        "Cargo.toml",
        "[package]\nname = \"delta\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[lib]\npath = \"generated/entry.rs\"\n",
    );
    let after = resolution_json(&run_project_resolution(&root, &candidates));
    assert_ne!(
        before["scope"]["packageGraph"],
        after["scope"]["packageGraph"]
    );
    assert_eq!(
        after["scope"]["sourceScopes"][0]["roots"],
        serde_json::json!(["generated"])
    );
    assert_eq!(
        after["scope"]["sourceScopes"][0]["explicitPaths"],
        serde_json::json!(["generated/entry.rs"])
    );
    assert_eq!(after["scope"]["metrics"]["fullWorkspaceReads"], 0);
    assert_eq!(after["scope"]["metrics"]["dbOpens"], 0);
}
