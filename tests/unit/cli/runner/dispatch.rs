use super::resolve_search_workspace_root;
use std::path::Path;

#[test]
fn relative_repo_root_resolves_from_provider_cwd() {
    let fixture = tempfile::TempDir::new().expect("create workspace-root fixture");
    let workspace = fixture.path().join("repository");
    std::fs::create_dir_all(&workspace).expect("create repository");

    let resolved =
        resolve_search_workspace_root(fixture.path(), Path::new("repository")).expect("resolve");

    assert_eq!(
        resolved,
        std::fs::canonicalize(workspace).expect("canonical repository")
    );
}

#[test]
fn provider_cwd_suffix_resolves_to_cwd() {
    let fixture = tempfile::TempDir::new().expect("create workspace-root fixture");
    let provider_cwd = fixture.path().join("crates/example");
    std::fs::create_dir_all(&provider_cwd).expect("create provider cwd");

    let resolved = resolve_search_workspace_root(&provider_cwd, Path::new("crates/example"))
        .expect("resolve provider cwd suffix");

    assert_eq!(
        resolved,
        std::fs::canonicalize(&provider_cwd).expect("canonical provider cwd")
    );
}

#[test]
fn absolute_workspace_root_stays_absolute() {
    let fixture = tempfile::TempDir::new().expect("create workspace-root fixture");
    let workspace = fixture.path().join("absolute-workspace");
    std::fs::create_dir_all(&workspace).expect("create absolute workspace");

    let resolved =
        resolve_search_workspace_root(fixture.path(), &workspace).expect("resolve absolute root");

    assert!(resolved.is_absolute());
    assert_eq!(
        resolved,
        std::fs::canonicalize(workspace).expect("canonical absolute workspace")
    );
}

#[test]
fn workspace_identity_is_never_concatenated_twice() {
    let fixture = tempfile::TempDir::new().expect("create workspace-root fixture");
    let provider_cwd = fixture.path().join("crates/example");
    std::fs::create_dir_all(&provider_cwd).expect("create provider cwd");
    let duplicated = provider_cwd.join("crates/example");

    let resolved = resolve_search_workspace_root(&provider_cwd, Path::new("crates/example"))
        .expect("resolve nonduplicated workspace identity");

    assert_ne!(resolved, duplicated);
    assert_eq!(
        resolved,
        std::fs::canonicalize(provider_cwd).expect("canonical provider cwd")
    );
}
