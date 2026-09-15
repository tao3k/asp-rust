//! File discovery for conventional Rust project layouts.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::AspRustScope;
use crate::parser::parse_cargo_manifest;
use crate::path::normalize_lexical_path;

/// Directory names ignored by default during recursive discovery.
pub const DEFAULT_IGNORED_DIR_NAMES: &[&str] = &[
    ".cache",
    ".direnv",
    ".git",
    ".idea",
    ".jj",
    ".run",
    ".vscode",
    "node_modules",
    "target",
];

/// Discover Rust source files under files or directories.
#[must_use]
pub fn discover_rust_files(
    paths: &[PathBuf],
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let mut files = BTreeSet::new();
    for path in paths {
        discover_path(
            path,
            ignored_dir_names,
            include_hidden_dir_names,
            &mut files,
            false,
        );
    }
    files.into_iter().collect()
}

/// Discover Rust files owned by one Cargo package without entering nested packages.
///
/// A package root can also be a workspace root. A package-scoped walk must not
/// silently absorb its workspace members; only the explicit workspace runner
/// is allowed to expand those owners.
#[must_use]
pub(crate) fn discover_rust_package_files(
    paths: &[PathBuf],
    package_root: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let excluded_package_roots =
        cargo_package_exclusion_roots(package_root, ignored_dir_names, include_hidden_dir_names);
    let mut files = BTreeSet::new();
    for path in paths {
        discover_path_with_package_boundary(
            path,
            &excluded_package_roots,
            ignored_dir_names,
            include_hidden_dir_names,
            &mut files,
            false,
        );
    }
    files.into_iter().collect()
}

/// Discover Cargo package roots under a project or workspace root.
#[must_use]
pub(crate) fn discover_cargo_package_roots(
    project_root: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let manifest_path = project_root.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Vec::new();
    }
    discover_package_roots_from_manifest(project_root, ignored_dir_names, include_hidden_dir_names)
}

/// Resolve manifest-declared package roots that are outside the anchored package.
///
/// This consumes parsed Cargo workspace/member/path-dependency facts. It does
/// not infer ownership from arbitrary nested `Cargo.toml` files.
pub(crate) fn cargo_package_exclusion_roots(
    package_root: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> BTreeSet<PathBuf> {
    let package_root = normalize_lexical_path(package_root);
    discover_cargo_package_roots(&package_root, ignored_dir_names, include_hidden_dir_names)
        .into_iter()
        .map(|root| normalize_lexical_path(&root))
        .filter(|root| root != &package_root)
        .collect()
}

fn discover_path(
    path: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
    files: &mut BTreeSet<PathBuf>,
    ignore_current: bool,
) {
    if ignore_current && should_ignore(path, ignored_dir_names, include_hidden_dir_names) {
        return;
    }
    if is_symlink_path(path) {
        return;
    }
    if path.is_file() {
        if is_rust_file(path) {
            files.insert(path.to_path_buf());
        }
        return;
    }
    if !path.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        discover_path(
            &entry.path(),
            ignored_dir_names,
            include_hidden_dir_names,
            files,
            true,
        );
    }
}

fn discover_path_with_package_boundary(
    path: &Path,
    excluded_package_roots: &BTreeSet<PathBuf>,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
    files: &mut BTreeSet<PathBuf>,
    ignore_current: bool,
) {
    if ignore_current && should_ignore(path, ignored_dir_names, include_hidden_dir_names) {
        return;
    }
    if is_symlink_path(path) {
        return;
    }
    if path.is_file() {
        if is_rust_file(path) {
            files.insert(path.to_path_buf());
        }
        return;
    }
    if !path.is_dir() {
        return;
    }
    if excluded_package_roots.contains(&normalize_lexical_path(path)) {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        discover_path_with_package_boundary(
            &entry.path(),
            excluded_package_roots,
            ignored_dir_names,
            include_hidden_dir_names,
            files,
            true,
        );
    }
}

pub(crate) fn is_symlink_path(path: &Path) -> bool {
    matches!(
        fs::symlink_metadata(path),
        Ok(metadata) if metadata.file_type().is_symlink()
    )
}

fn discover_cargo_manifests(
    path: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
    manifests: &mut BTreeSet<PathBuf>,
    ignore_current: bool,
) {
    if ignore_current && should_ignore(path, ignored_dir_names, include_hidden_dir_names) {
        return;
    }
    if path.is_file() {
        if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
            manifests.insert(path.to_path_buf());
        }
        return;
    }
    if !path.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        discover_cargo_manifests(
            &entry.path(),
            ignored_dir_names,
            include_hidden_dir_names,
            manifests,
            true,
        );
    }
}

fn should_ignore(
    path: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            should_ignore_dir_name(name, ignored_dir_names, include_hidden_dir_names)
        })
}

fn should_ignore_dir_name(
    name: &str,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> bool {
    ignored_dir_names.contains(name)
        || (name.starts_with('.')
            && name != "."
            && name != ".."
            && !include_hidden_dir_names.contains(name))
}

fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}

fn discover_package_roots_from_manifest(
    project_root: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let manifest = parse_cargo_manifest(project_root);
    if manifest.workspace_members.is_empty() {
        let mut roots = BTreeSet::from([project_root.to_path_buf()]);
        insert_path_dependency_roots(&[], &mut roots);
        return roots.into_iter().collect();
    }

    let excludes = manifest
        .workspace_excludes
        .iter()
        .map(|pattern| project_root.join(pattern))
        .collect::<Vec<_>>();
    let mut roots = BTreeSet::new();
    if manifest.has_package {
        roots.insert(project_root.to_path_buf());
    }
    for member_pattern in manifest.workspace_members {
        for member_root in expand_workspace_member_pattern(
            project_root,
            &member_pattern,
            ignored_dir_names,
            include_hidden_dir_names,
        ) {
            insert_package_root(
                project_root,
                ignored_dir_names,
                include_hidden_dir_names,
                &excludes,
                &mut roots,
                member_root,
            );
        }
    }
    insert_path_dependency_roots(&excludes, &mut roots);
    roots.into_iter().collect()
}

fn insert_path_dependency_roots(excludes: &[PathBuf], roots: &mut BTreeSet<PathBuf>) {
    let mut pending = roots.iter().cloned().collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    while let Some(package_root) = pending.pop() {
        if !visited.insert(package_root.clone()) {
            continue;
        }
        let manifest = parse_cargo_manifest(&package_root);
        for dependency_root in manifest.path_dependency_roots {
            if insert_explicit_path_dependency_root(excludes, roots, dependency_root.clone()) {
                pending.push(normalize_lexical_path(&dependency_root));
            }
        }
    }
}

fn insert_explicit_path_dependency_root(
    excludes: &[PathBuf],
    roots: &mut BTreeSet<PathBuf>,
    dependency_root: PathBuf,
) -> bool {
    let dependency_root = normalize_lexical_path(&dependency_root);
    if is_excluded_member(&dependency_root, excludes)
        || !dependency_root.join("Cargo.toml").is_file()
    {
        return false;
    }
    roots.insert(dependency_root)
}

fn insert_package_root(
    project_root: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
    excludes: &[PathBuf],
    roots: &mut BTreeSet<PathBuf>,
    package_root: PathBuf,
) -> bool {
    let package_root = normalize_lexical_path(&package_root);
    let project_root = normalize_lexical_path(project_root);
    if !package_root.starts_with(&project_root) {
        return false;
    }
    if is_excluded_member(&package_root, excludes) {
        return false;
    }
    let package_relative = package_root
        .strip_prefix(&project_root)
        .unwrap_or(package_root.as_path());
    if ignored_path_contains_ignored_segment(
        package_relative,
        ignored_dir_names,
        include_hidden_dir_names,
    ) {
        return false;
    }
    if !package_root.join("Cargo.toml").is_file() {
        return false;
    }
    roots.insert(package_root)
}

fn expand_workspace_member_pattern(
    project_root: &Path,
    pattern: &str,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    if !pattern.contains('*') {
        return vec![project_root.join(pattern)];
    }
    let search_root = fixed_prefix_root(project_root, pattern);
    let mut manifests = BTreeSet::new();
    discover_cargo_manifests(
        &search_root,
        ignored_dir_names,
        include_hidden_dir_names,
        &mut manifests,
        false,
    );
    manifests
        .into_iter()
        .filter_map(|manifest| manifest.parent().map(Path::to_path_buf))
        .filter(|root| {
            root.strip_prefix(project_root)
                .ok()
                .and_then(Path::to_str)
                .is_some_and(|relative| glob_pattern_matches(pattern, relative))
        })
        .collect()
}

fn fixed_prefix_root(project_root: &Path, pattern: &str) -> PathBuf {
    let before_star = pattern.split('*').next().unwrap_or_default();
    let prefix = before_star
        .rsplit_once('/')
        .map_or("", |(prefix, _)| prefix);
    if prefix.is_empty() {
        return project_root.to_path_buf();
    }
    project_root.join(prefix)
}

pub(crate) fn glob_pattern_matches(pattern: &str, value: &str) -> bool {
    let pattern = normalize_glob_path(pattern);
    let value = normalize_glob_path(value);
    let pattern_components = pattern.split('/').collect::<Vec<_>>();
    let value_components = value.split('/').collect::<Vec<_>>();
    pattern_components.len() == value_components.len()
        && pattern_components.iter().zip(value_components).all(
            |(pattern_component, value_component)| {
                glob_component_matches(pattern_component, value_component)
            },
        )
}

fn normalize_glob_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn glob_component_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    let mut remaining = value;
    let mut parts = pattern.split('*').peekable();
    let Some(first) = parts.next() else {
        return pattern == value;
    };
    if !remaining.starts_with(first) {
        return false;
    }
    remaining = &remaining[first.len()..];
    while let Some(part) = parts.next() {
        if part.is_empty() {
            continue;
        }
        let Some(index) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[index + part.len()..];
        if parts.peek().is_none() && !remaining.is_empty() {
            return false;
        }
    }
    pattern.ends_with('*') || remaining.is_empty()
}

fn is_excluded_member(member_root: &Path, excludes: &[PathBuf]) -> bool {
    excludes
        .iter()
        .any(|excluded| member_root == excluded || member_root.starts_with(excluded))
}

fn ignored_path_contains_ignored_segment(
    path: &Path,
    ignored_dir_names: &BTreeSet<String>,
    include_hidden_dir_names: &BTreeSet<String>,
) -> bool {
    path.iter()
        .filter_map(|component| component.to_str())
        .any(|component| {
            should_ignore_dir_name(component, ignored_dir_names, include_hidden_dir_names)
        })
}

/// Build a conventional project scope from a project root.
#[must_use]
pub fn asp_rust_scope(
    project_root: &Path,
    include_tests: bool,
    source_dir_names: &[String],
    test_dir_names: &[String],
) -> AspRustScope {
    let cargo_manifest = parse_cargo_manifest(project_root);
    let mut package_paths = ["build.rs", "examples", "benches"]
        .iter()
        .map(|name| project_root.join(name))
        .filter(|path| path.exists())
        .collect::<BTreeSet<_>>();
    package_paths.extend(
        cargo_manifest
            .example_targets
            .iter()
            .map(|target| &target.path)
            .chain(
                cargo_manifest
                    .bench_targets
                    .iter()
                    .map(|target| &target.path),
            )
            .filter(|path| path.exists())
            .cloned(),
    );

    let mut source_paths = configured_existing_paths(project_root, source_dir_names);
    source_paths.extend(existing_paths([project_root.join("src")]));
    source_paths.extend(existing_paths(target_parent_paths(
        &cargo_manifest.source_target_files,
    )));

    let test_paths = if include_tests {
        let mut paths = configured_existing_paths(project_root, test_dir_names);
        paths.extend(existing_paths([project_root.join("tests")]));
        paths.extend(existing_paths(target_parent_paths(
            &cargo_manifest.test_target_files,
        )));
        paths
    } else {
        BTreeSet::new()
    };
    AspRustScope {
        project_root: project_root.to_path_buf(),
        source_paths: source_paths.into_iter().collect(),
        test_paths: test_paths.into_iter().collect(),
        package_paths: package_paths.into_iter().collect(),
        fallback_paths: vec![project_root.to_path_buf()],
    }
}

fn configured_existing_paths(project_root: &Path, relative_paths: &[String]) -> BTreeSet<PathBuf> {
    existing_paths(relative_paths.iter().map(|name| project_root.join(name)))
}

fn existing_paths(paths: impl IntoIterator<Item = PathBuf>) -> BTreeSet<PathBuf> {
    paths.into_iter().filter(|path| path.exists()).collect()
}

fn target_parent_paths(target_files: &[PathBuf]) -> Vec<PathBuf> {
    target_files
        .iter()
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect()
}
