use super::{
    CandidateGeneration, ProjectResolutionCollectionScope, ProjectResolutionError,
    ProjectResolutionInput, resolve_cargo_project_resolution,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn candidates(paths: &[&str]) -> ProjectResolutionInput {
    ProjectResolutionInput {
        candidate_generation: CandidateGeneration {
            algorithm: "blake3-path-set-v1".to_string(),
            digest: format!("blake3:{}", "a".repeat(64)),
            authorities: vec!["git-index".to_string()],
        },
        collection_scope: ProjectResolutionCollectionScope::CompleteGeneration,
        candidate_paths: paths.iter().map(PathBuf::from).collect(),
        policy_exclusions: Vec::new(),
    }
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rust-project-resolution-{name}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent");
        }
        std::fs::write(path, contents).expect("write fixture file");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn resolves_workspace_packages_targets_and_internal_dependencies() {
    let fixture = Fixture::new("workspace");
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"2\"\n",
    );
    fixture.write("Cargo.lock", "version = 4\n");
    fixture.write(
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nb = { path = \"../b\" }\n",
    );
    fixture.write("crates/a/src/lib.rs", "pub fn a() {}\n");
    fixture.write("crates/a/src/nested.rs", "pub fn nested() {}\n");
    fixture.write(
        "crates/b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    fixture.write("crates/b/src/lib.rs", "pub fn b() {}\n");
    let candidates = candidates(&[
        "Cargo.toml",
        "Cargo.lock",
        "crates/a/Cargo.toml",
        "crates/a/src/lib.rs",
        "crates/a/src/nested.rs",
        "crates/b/Cargo.toml",
        "crates/b/src/lib.rs",
    ]);

    let scope = resolve_cargo_project_resolution(&fixture.root, &candidates)
        .expect("resolve Cargo workspace ProjectResolution");

    assert_eq!(scope.package_graph.packages.len(), 2);
    assert_eq!(scope.package_graph.manifests.len(), 3);
    assert_eq!(scope.package_graph.lockfiles.len(), 1);
    assert_eq!(scope.source_scopes.len(), 2);
    assert!(
        scope
            .source_scopes
            .iter()
            .any(|scope| { scope.roots.contains(&PathBuf::from("crates/a/src")) })
    );
    assert!(
        scope
            .package_graph
            .internal_dependency_edges
            .iter()
            .any(|dependency| { dependency.from_package_id != dependency.to_package_id })
    );
    assert_eq!(scope.metrics.full_workspace_reads, 0);
    assert_eq!(scope.metrics.db_opens, 0);
}

#[test]
fn identical_explicit_and_default_targets_publish_one_source_scope() {
    let fixture = Fixture::new("explicit-default-target");
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"sample\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    );
    fixture.write("src/lib.rs", "pub fn sample() {}\n");

    let scope =
        resolve_cargo_project_resolution(&fixture.root, &candidates(&["Cargo.toml", "src/lib.rs"]))
            .expect("resolve one canonical Cargo target");

    assert_eq!(scope.package_graph.packages.len(), 1);
    assert_eq!(scope.package_graph.packages[0].targets.len(), 1);
    assert_eq!(scope.source_scopes.len(), 1);
}

#[test]
fn same_named_test_entrypoints_have_distinct_target_and_scope_identities() {
    let fixture = Fixture::new("same-named-test-entrypoints");
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"sample\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    fixture.write("src/lib.rs", "pub fn sample() {}\n");
    fixture.write(
        "tests/integration/main.rs",
        "#[test]\nfn integration() {}\n",
    );
    fixture.write("tests/unit/main.rs", "#[test]\nfn unit() {}\n");

    let scope = resolve_cargo_project_resolution(
        &fixture.root,
        &candidates(&[
            "Cargo.toml",
            "src/lib.rs",
            "tests/integration/main.rs",
            "tests/unit/main.rs",
        ]),
    )
    .expect("resolve distinct same-named Cargo test targets");

    let package = &scope.package_graph.packages[0];
    assert_eq!(package.targets.len(), 3);
    let target_ids = package
        .targets
        .iter()
        .map(|target| target.target_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let scope_ids = scope
        .source_scopes
        .iter()
        .map(|source_scope| source_scope.scope_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(target_ids.len(), package.targets.len());
    assert_eq!(scope_ids.len(), scope.source_scopes.len());
}

#[test]
fn fails_closed_when_root_project_entry_is_not_a_candidate() {
    let fixture = Fixture::new("missing-entry");
    fixture.write(
        "nested/Cargo.toml",
        "[package]\nname = \"nested\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    let candidates = candidates(&["nested/Cargo.toml"]);

    let error = resolve_cargo_project_resolution(&fixture.root, &candidates)
        .expect_err("root entry must be explicit");

    assert!(matches!(
        error,
        ProjectResolutionError::ProjectEntryMissing { .. }
    ));
    assert!(error.to_string().contains("project-entry-missing"));
}
