use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use tempfile::tempdir;

fn run_project_resolution(request: &Value, current_dir: &std::path::Path) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rs-harness"))
        .arg("project-resolution-stdin")
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn provider");
    child
        .stdin
        .take()
        .expect("provider stdin")
        .write_all(request.to_string().as_bytes())
        .expect("write typed request");
    let output = child.wait_with_output().expect("wait for provider");
    assert!(
        output.status.success(),
        "provider failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("typed response")
}

fn request(candidate_paths: &[&str]) -> Value {
    json!({
        "schemaId": "agent.semantic-protocols.provider-project-resolution-request",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": "rs-harness",
        "workspace": ".",
        "candidateBase": ".",
        "candidateGeneration": {
            "algorithm": "blake3-merkle-v1",
            "digest": "blake3-256:0000000000000000000000000000000000000000000000000000000000000000",
            "authorities": candidate_paths
        },
        "collectionScope": { "kind": "complete-generation" },
        "candidatePaths": candidate_paths,
        "policyExclusions": []
    })
}

fn source_scopes(response: &Value) -> &[Value] {
    assert!(
        response.get("resolution").is_none(),
        "legacy project-resolution response field must not be published"
    );
    response["scope"]["sourceScopes"]
        .as_array()
        .expect("sourceScopes array")
}

fn assert_single_digit_millisecond_cold_path(response: &Value) {
    let elapsed_micros = response["scope"]["metrics"]["elapsedMicros"]
        .as_u64()
        .expect("provider elapsedMicros");
    assert!(
        elapsed_micros < 10_000,
        "cold manifest-to-scope projection exceeded the 10ms budget: {elapsed_micros}us"
    );
}

#[test]
fn project_resolution_derives_default_source_scope_from_cargo_manifest() {
    let fixture = tempdir().expect("project fixture");
    fs::create_dir(fixture.path().join("src")).expect("source directory");
    fs::write(
        fixture.path().join("Cargo.toml"),
        "[package]\nname = \"scope-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("fixture manifest");
    fs::write(fixture.path().join("src/lib.rs"), "pub fn fixture() {}\n").expect("fixture library");
    let manifest = cargo_toml::Manifest::from_path(fixture.path().join("Cargo.toml"))
        .expect("cargo_toml completed manifest");
    assert!(
        manifest.lib.is_some(),
        "cargo_toml must materialize Cargo's implicit library target"
    );
    let response = run_project_resolution(&request(&["Cargo.toml", "src/lib.rs"]), fixture.path());
    assert_eq!(response["state"], "resolved");
    let scopes = source_scopes(&response);
    assert_eq!(
        scopes.len(),
        1,
        "one Cargo library target scope; response={response}"
    );
    assert_eq!(scopes[0]["roots"], json!(["src"]));
    assert_eq!(scopes[0]["extensions"], json!([".rs"]));
    assert_eq!(scopes[0]["classifications"], json!(["library"]));
    assert_single_digit_millisecond_cold_path(&response);
}

#[test]
fn implicit_target_scope_is_owned_by_the_candidate_snapshot() {
    let fixture = tempdir().expect("project fixture");
    fs::create_dir(fixture.path().join("src")).expect("source directory");
    fs::write(
        fixture.path().join("Cargo.toml"),
        "[package]\nname = \"candidate-independent\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("fixture manifest");
    fs::write(fixture.path().join("src/main.rs"), "fn main() {}\n").expect("fixture binary");

    let manifest_only = run_project_resolution(&request(&["Cargo.toml"]), fixture.path());
    let with_source =
        run_project_resolution(&request(&["Cargo.toml", "src/main.rs"]), fixture.path());
    assert!(source_scopes(&manifest_only).is_empty());
    assert_eq!(source_scopes(&with_source).len(), 1);
    assert_single_digit_millisecond_cold_path(&manifest_only);
    assert_single_digit_millisecond_cold_path(&with_source);
}

#[test]
fn workspace_member_scopes_bootstrap_from_manifests_only() {
    let fixture = tempdir().expect("workspace fixture");
    for member in ["library", "binary"] {
        fs::create_dir_all(fixture.path().join(member).join("src"))
            .expect("member source directory");
    }
    fs::write(
        fixture.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"library\", \"binary\"]\nresolver = \"3\"\n",
    )
    .expect("workspace manifest");
    fs::write(
        fixture.path().join("library/Cargo.toml"),
        "[package]\nname = \"scope-library\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("library manifest");
    fs::write(
        fixture.path().join("binary/Cargo.toml"),
        "[package]\nname = \"scope-binary\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("binary manifest");
    fs::write(
        fixture.path().join("library/src/lib.rs"),
        "pub fn library() {}\n",
    )
    .expect("library source");
    fs::write(fixture.path().join("binary/src/main.rs"), "fn main() {}\n").expect("binary source");

    let response = run_project_resolution(
        &request(&[
            "Cargo.toml",
            "library/Cargo.toml",
            "library/src/lib.rs",
            "binary/Cargo.toml",
            "binary/src/main.rs",
        ]),
        fixture.path(),
    );
    assert_eq!(response["state"], "resolved");
    let scopes = source_scopes(&response);
    assert_eq!(scopes.len(), 2, "one scope per workspace member target");
    let mut roots = scopes
        .iter()
        .map(|scope| scope["roots"].clone())
        .collect::<Vec<_>>();
    roots.sort_by_key(Value::to_string);
    assert_eq!(roots, vec![json!(["binary/src"]), json!(["library/src"])]);
    assert_single_digit_millisecond_cold_path(&response);
}

#[test]
fn workspace_member_inherits_package_fields_from_the_root_manifest() {
    let fixture = tempdir().expect("workspace fixture");
    fs::create_dir_all(fixture.path().join("member/src")).expect("member source directory");
    fs::write(
        fixture.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"member\"]\nresolver = \"3\"\n\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("workspace manifest");
    fs::write(
        fixture.path().join("member/Cargo.toml"),
        "[package]\nname = \"inherited-member\"\nversion.workspace = true\nedition.workspace = true\n",
    )
    .expect("member manifest");
    fs::write(
        fixture.path().join("member/src/lib.rs"),
        "pub fn inherited() {}\n",
    )
    .expect("member source");

    let response = run_project_resolution(
        &request(&["Cargo.toml", "member/Cargo.toml", "member/src/lib.rs"]),
        fixture.path(),
    );
    assert_eq!(response["state"], "resolved");
    assert_eq!(source_scopes(&response).len(), 1);
    assert_eq!(source_scopes(&response)[0]["roots"], json!(["member/src"]));
    assert_single_digit_millisecond_cold_path(&response);
}

#[test]
fn workspace_path_dependencies_are_automatic_members_of_the_source_scope() {
    let fixture = tempdir().expect("workspace fixture");
    for package in ["app", "identity", "leaf"] {
        fs::create_dir_all(fixture.path().join(package).join("src"))
            .expect("package source directory");
    }
    fs::write(
        fixture.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\nresolver = \"3\"\n\n[workspace.dependencies]\nidentity = { path = \"identity\" }\n",
    )
    .expect("workspace manifest");
    fs::write(
        fixture.path().join("app/Cargo.toml"),
        "[package]\nname = \"scope-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nidentity.workspace = true\n",
    )
    .expect("app manifest");
    fs::write(
        fixture.path().join("identity/Cargo.toml"),
        "[package]\nname = \"identity\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nleaf = { path = \"../leaf\" }\n",
    )
    .expect("identity manifest");
    fs::write(
        fixture.path().join("leaf/Cargo.toml"),
        "[package]\nname = \"leaf\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("leaf manifest");
    for package in ["app", "identity", "leaf"] {
        fs::write(
            fixture.path().join(package).join("src/lib.rs"),
            format!("pub fn {package}() {{}}\n"),
        )
        .expect("package source");
    }

    let response = run_project_resolution(
        &request(&[
            "Cargo.toml",
            "app/Cargo.toml",
            "app/src/lib.rs",
            "identity/Cargo.toml",
            "identity/src/lib.rs",
            "leaf/Cargo.toml",
            "leaf/src/lib.rs",
        ]),
        fixture.path(),
    );
    assert_eq!(response["state"], "resolved");
    let mut roots = source_scopes(&response)
        .iter()
        .map(|scope| scope["roots"].clone())
        .collect::<Vec<_>>();
    roots.sort_by_key(Value::to_string);
    assert_eq!(
        roots,
        vec![
            json!(["app/src"]),
            json!(["identity/src"]),
            json!(["leaf/src"]),
        ],
        "Cargo's in-workspace path-dependency closure must be a single parser-owned scope"
    );
    assert_single_digit_millisecond_cold_path(&response);
}
