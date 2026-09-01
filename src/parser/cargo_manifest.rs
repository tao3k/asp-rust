//! Cargo manifest facts owned by the parser layer.

use std::collections::BTreeSet;
#[cfg(feature = "provider-server")]
use std::collections::HashSet;
#[cfg(any(feature = "provider-server", feature = "search"))]
use std::fs;
#[cfg(feature = "provider-server")]
use std::io;
use std::path::{Path, PathBuf};

use cargo_toml::{Dependency, DepsSet, Manifest, Product};

const ASP_RUST_PACKAGE_NAMES: &[&str] = &["asp-rust", "asp-rust-build-support"];

#[derive(Debug, Clone, Default)]
pub(crate) struct CargoManifestFacts {
    pub(crate) has_package: bool,
    #[cfg(any(feature = "provider-server", feature = "search"))]
    pub(crate) package_name: Option<String>,
    pub(crate) package_edition: Option<String>,
    pub(crate) workspace_members: Vec<String>,
    pub(crate) workspace_excludes: Vec<String>,
    pub(crate) path_dependency_roots: Vec<PathBuf>,
    #[cfg(feature = "provider-server")]
    pub(crate) package_targets: Vec<CargoPackageTargetFacts>,
    pub(crate) source_target_files: Vec<PathBuf>,
    pub(crate) example_targets: Vec<CargoExampleTargetFacts>,
    pub(crate) test_target_files: Vec<PathBuf>,
    pub(crate) bench_targets: Vec<CargoBenchTargetFacts>,
    pub(crate) references_harness: bool,
    pub(crate) references_harness_build_dependency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoExampleTargetFacts {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) required_features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoBenchTargetFacts {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) harness: bool,
    pub(crate) required_features: Vec<String>,
}

#[cfg(feature = "provider-server")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoPackageTargetFacts {
    pub(crate) name: String,
    pub(crate) kind: &'static str,
    pub(crate) path: PathBuf,
}

pub(crate) fn parse_cargo_manifest(project_root: &Path) -> CargoManifestFacts {
    let Some(manifest) = read_manifest(project_root) else {
        return CargoManifestFacts::default();
    };
    cargo_manifest_facts(project_root, &manifest)
}

pub(crate) fn parse_required_cargo_manifest(
    project_root: &Path,
) -> Result<CargoManifestFacts, String> {
    let manifest_path = project_root.join("Cargo.toml");
    let manifest = Manifest::from_path(&manifest_path).map_err(|error| {
        format!(
            "parse Cargo package graph anchor {}: {error}",
            manifest_path.display()
        )
    })?;
    Ok(cargo_manifest_facts(project_root, &manifest))
}

#[cfg(feature = "provider-server")]
pub(crate) fn parse_cargo_project_facts(
    project_root: &Path,
    candidate_paths: &BTreeSet<PathBuf>,
    workspace_manifest: Option<(&Manifest, &Path)>,
) -> (
    CargoManifestFacts,
    Vec<super::cargo_dependency_facts::CargoDependencyFacts>,
    Option<Manifest>,
) {
    let Some(mut manifest) = read_candidate_manifest(project_root) else {
        return (CargoManifestFacts::default(), Vec::new(), None);
    };
    if manifest
        .complete_from_abstract_filesystem::<cargo_toml::Value, _>(
            CandidateFilesystem { candidate_paths },
            workspace_manifest,
        )
        .is_err()
    {
        return (CargoManifestFacts::default(), Vec::new(), None);
    }
    let dependencies =
        super::cargo_dependency_facts::cargo_dependency_facts_from_manifest(&manifest);
    (
        cargo_manifest_facts(project_root, &manifest),
        dependencies,
        Some(manifest),
    )
}

#[cfg(feature = "provider-server")]
struct CandidateFilesystem<'a> {
    candidate_paths: &'a BTreeSet<PathBuf>,
}

#[cfg(feature = "provider-server")]
impl cargo_toml::AbstractFilesystem for CandidateFilesystem<'_> {
    fn file_names_in(&self, relative_directory: &str) -> io::Result<HashSet<Box<str>>> {
        let directory = Path::new(relative_directory);
        Ok(self
            .candidate_paths
            .iter()
            .filter_map(|path| path.strip_prefix(directory).ok())
            .filter_map(|suffix| suffix.components().next())
            .filter_map(|component| component.as_os_str().to_str())
            .map(|name| name.to_owned().into_boxed_str())
            .collect())
    }
}

#[cfg(feature = "provider-server")]
fn read_candidate_manifest(project_root: &Path) -> Option<Manifest> {
    fs::read(project_root.join("Cargo.toml"))
        .ok()
        .and_then(|bytes| Manifest::from_slice(&bytes).ok())
}

fn cargo_manifest_facts(project_root: &Path, manifest: &Manifest) -> CargoManifestFacts {
    let references_harness = manifest_references_harness(manifest);
    let references_harness_build_dependency =
        manifest_references_harness_build_dependency(manifest);
    let package_name = manifest
        .package
        .as_ref()
        .map(|package| package.name.clone());
    let package_edition = manifest
        .package
        .as_ref()
        .map(|package| package.edition().to_string());
    let has_package = package_name
        .as_deref()
        .is_some_and(|name| !name.trim().is_empty());
    let (workspace_members, workspace_excludes) = manifest
        .workspace
        .as_ref()
        .map(|workspace| (workspace.members.clone(), workspace.exclude.clone()))
        .unwrap_or_default();
    #[cfg(feature = "provider-server")]
    let package_targets = manifest_package_targets(project_root, manifest);
    let source_target_files = manifest_source_target_files(project_root, manifest);
    let example_targets = manifest_example_targets(project_root, &manifest.example);
    let test_target_files = manifest_test_target_files(project_root, &manifest.test);
    let bench_targets = manifest_bench_targets(project_root, &manifest.bench);
    let path_dependency_roots = manifest_path_dependency_roots(project_root, manifest);
    CargoManifestFacts {
        has_package,
        #[cfg(any(feature = "provider-server", feature = "search"))]
        package_name,
        package_edition,
        workspace_members,
        workspace_excludes,
        path_dependency_roots,
        #[cfg(feature = "provider-server")]
        package_targets,
        source_target_files,
        example_targets,
        test_target_files,
        bench_targets,
        references_harness,
        references_harness_build_dependency,
    }
}

#[cfg(feature = "search")]
pub(crate) fn parse_cargo_workspace_member_roots(project_root: &Path) -> Vec<PathBuf> {
    let Some(manifest) = read_manifest(project_root) else {
        return Vec::new();
    };
    let Some(workspace) = manifest.workspace.as_ref() else {
        return Vec::new();
    };
    let mut roots = BTreeSet::new();
    for member in &workspace.members {
        expand_workspace_member_pattern(project_root, member, &mut roots);
    }
    roots.retain(|root| {
        root.join("Cargo.toml").is_file()
            && root.strip_prefix(project_root).ok().is_none_or(|relative| {
                let relative = relative.to_string_lossy().replace('\\', "/");
                !workspace
                    .exclude
                    .iter()
                    .any(|pattern| workspace_member_pattern_matches(pattern, &relative))
            })
    });
    roots.into_iter().collect()
}

#[cfg(feature = "provider-server")]
pub(crate) fn cargo_workspace_member_roots_from_candidates(
    project_root: &Path,
    facts: &CargoManifestFacts,
    candidate_paths: &BTreeSet<PathBuf>,
) -> Vec<PathBuf> {
    candidate_paths
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "Cargo.toml"))
        .filter_map(|manifest| manifest.parent())
        .filter(|relative| !relative.as_os_str().is_empty())
        .filter(|relative| {
            let relative = relative.to_string_lossy().replace('\\', "/");
            facts
                .workspace_members
                .iter()
                .any(|pattern| workspace_member_pattern_matches(pattern, &relative))
                && !facts
                    .workspace_excludes
                    .iter()
                    .any(|pattern| workspace_member_pattern_matches(pattern, &relative))
        })
        .map(|relative| project_root.join(relative))
        .collect()
}

#[cfg(all(test, feature = "provider-server"))]
pub(crate) fn cargo_project_root_for_path(path: &Path) -> Result<PathBuf, String> {
    cargo_package_root_for_path(path)
        .map(|manifest_dir| cargo_project_root_for_manifest_dir(&manifest_dir))
}

#[cfg(all(test, feature = "provider-server"))]
pub(crate) fn cargo_package_root_for_path(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        format!(
            "failed to resolve Rust project path {}: {error}",
            path.display()
        )
    })?;
    let mut current = if canonical.is_file() {
        canonical
            .parent()
            .ok_or_else(|| format!("failed to resolve parent for {}", canonical.display()))?
            .to_path_buf()
    } else {
        canonical
    };
    loop {
        if current.join("Cargo.toml").is_file() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }
    Err("failed to find Rust project root: Cargo.toml not found".to_string())
}

#[cfg(all(test, feature = "provider-server"))]
fn cargo_project_root_for_manifest_dir(manifest_dir: &Path) -> PathBuf {
    let manifest = read_manifest(manifest_dir);
    if manifest
        .as_ref()
        .is_some_and(|manifest| manifest.workspace.is_some())
    {
        return manifest_dir.to_path_buf();
    }
    if let Some(workspace_root) = manifest
        .as_ref()
        .and_then(|manifest| manifest.package.as_ref())
        .and_then(|package| package.workspace.as_deref())
        .map(|workspace| manifest_dir.join(workspace))
    {
        return fs::canonicalize(&workspace_root).unwrap_or(workspace_root);
    }
    cargo_parent_workspace_root(manifest_dir).unwrap_or_else(|| manifest_dir.to_path_buf())
}

#[cfg(all(test, feature = "provider-server"))]
fn cargo_parent_workspace_root(manifest_dir: &Path) -> Option<PathBuf> {
    let mut current = manifest_dir.parent();
    while let Some(candidate) = current {
        if read_manifest(candidate).as_ref().is_some_and(|manifest| {
            workspace_contains_manifest_dir(candidate, manifest, manifest_dir)
        }) {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }
    None
}

#[cfg(all(test, feature = "provider-server"))]
fn workspace_contains_manifest_dir(
    workspace_root: &Path,
    manifest: &Manifest,
    manifest_dir: &Path,
) -> bool {
    let Some(workspace) = manifest.workspace.as_ref() else {
        return false;
    };
    let Ok(relative) = manifest_dir.strip_prefix(workspace_root) else {
        return false;
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    if workspace
        .exclude
        .iter()
        .any(|pattern| workspace_member_pattern_matches(pattern, &relative))
    {
        return false;
    }
    workspace
        .members
        .iter()
        .any(|pattern| workspace_member_pattern_matches(pattern, &relative))
}

#[cfg(any(feature = "provider-server", feature = "search"))]
pub(crate) fn workspace_member_pattern_matches(pattern: &str, relative: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == relative;
    }
    let pattern_components = pattern.replace('\\', "/");
    let pattern_components = pattern_components.split('/').collect::<Vec<_>>();
    let relative_components = relative.split('/').collect::<Vec<_>>();
    pattern_components.len() == relative_components.len()
        && pattern_components.iter().zip(relative_components).all(
            |(pattern_component, relative_component)| {
                workspace_member_component_matches(pattern_component, relative_component)
            },
        )
}

#[cfg(any(feature = "provider-server", feature = "search"))]
fn workspace_member_component_matches(pattern: &str, value: &str) -> bool {
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

#[cfg(feature = "search")]
fn expand_workspace_member_pattern(
    project_root: &Path,
    pattern: &str,
    roots: &mut BTreeSet<PathBuf>,
) {
    let normalized = pattern.replace('\\', "/");
    let components = normalized
        .split('/')
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>();
    if components.is_empty() {
        return;
    }
    expand_workspace_member_components(project_root, &components, roots);
}

#[cfg(feature = "search")]
fn expand_workspace_member_components(
    current: &Path,
    components: &[&str],
    roots: &mut BTreeSet<PathBuf>,
) {
    let Some((component, remaining)) = components.split_first() else {
        roots.insert(current.to_path_buf());
        return;
    };
    if !component.contains('*') {
        expand_workspace_member_components(&current.join(component), remaining, roots);
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if workspace_member_component_matches(component, name) {
            expand_workspace_member_components(&path, remaining, roots);
        }
    }
}

fn manifest_example_targets(
    project_root: &Path,
    example_targets: &[Product],
) -> Vec<CargoExampleTargetFacts> {
    example_targets
        .iter()
        .filter_map(|target| {
            let name = target.name.as_deref()?.trim();
            if name.is_empty() {
                return None;
            }
            let path = completed_product_path(project_root, target, "example", None)?;
            let mut required_features = target.required_features.clone();
            required_features.sort();
            required_features.dedup();
            Some(CargoExampleTargetFacts {
                name: name.to_string(),
                path,
                required_features,
            })
        })
        .collect()
}

fn manifest_path_dependency_roots(project_root: &Path, manifest: &Manifest) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    if let Some(workspace) = &manifest.workspace {
        collect_path_dependency_roots(project_root, &workspace.dependencies, &mut roots);
    }
    collect_path_dependency_roots(project_root, &manifest.dependencies, &mut roots);
    collect_path_dependency_roots(project_root, &manifest.dev_dependencies, &mut roots);
    collect_path_dependency_roots(project_root, &manifest.build_dependencies, &mut roots);
    for target in manifest.target.values() {
        collect_path_dependency_roots(project_root, &target.dependencies, &mut roots);
        collect_path_dependency_roots(project_root, &target.dev_dependencies, &mut roots);
        collect_path_dependency_roots(project_root, &target.build_dependencies, &mut roots);
    }
    roots.into_iter().collect()
}

fn collect_path_dependency_roots(
    project_root: &Path,
    dependencies: &DepsSet,
    roots: &mut BTreeSet<PathBuf>,
) {
    for dependency in dependencies.values() {
        if let Some(root) = dependency_path_root(project_root, dependency) {
            roots.insert(root);
        }
    }
}

fn dependency_path_root(project_root: &Path, dependency: &Dependency) -> Option<PathBuf> {
    dependency
        .detail()
        .and_then(|detail| detail.path.as_deref())
        .map(|path| resolve_dependency_path(project_root, path))
}

fn resolve_dependency_path(base: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn read_manifest(project_root: &Path) -> Option<Manifest> {
    let manifest_path = project_root.join("Cargo.toml");
    Manifest::from_path(&manifest_path).ok()
}

#[cfg(any(feature = "search", test))]
pub(super) fn read_manifest_for_cfg(project_root: &Path) -> Option<Manifest> {
    read_manifest(project_root)
}

fn manifest_source_target_files(project_root: &Path, manifest: &Manifest) -> Vec<PathBuf> {
    let package_name = manifest
        .package
        .as_ref()
        .map(|package| package.name.as_str());
    let mut target_files = Vec::new();
    if let Some(library_target) = &manifest.lib {
        target_files.extend(manifest_product_target_files(
            project_root,
            std::slice::from_ref(library_target),
            "library",
            package_name,
        ));
    }
    target_files.extend(manifest_product_target_files(
        project_root,
        &manifest.bin,
        "binary",
        package_name,
    ));
    target_files
}

#[cfg(feature = "provider-server")]
fn manifest_package_targets(
    project_root: &Path,
    manifest: &Manifest,
) -> Vec<CargoPackageTargetFacts> {
    let package_name = manifest
        .package
        .as_ref()
        .map(|package| package.name.as_str());
    let mut targets = Vec::new();
    if let Some(target) = manifest.lib.as_ref()
        && let Some(target) = package_target_fact(project_root, target, "library", package_name)
    {
        targets.push(target);
    }
    for (products, kind) in [
        (manifest.bin.as_slice(), "binary"),
        (manifest.test.as_slice(), "test"),
        (manifest.example.as_slice(), "example"),
        (manifest.bench.as_slice(), "bench"),
    ] {
        targets.extend(
            products
                .iter()
                .filter_map(|target| package_target_fact(project_root, target, kind, package_name)),
        );
    }
    targets.sort();
    targets.dedup();
    targets
}

#[cfg(feature = "provider-server")]
fn package_target_fact(
    project_root: &Path,
    target: &Product,
    kind: &'static str,
    package_name: Option<&str>,
) -> Option<CargoPackageTargetFacts> {
    let path = completed_product_path(project_root, target, kind, package_name)?;
    let name = target
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| cargo_default_target_name(project_root, &path, kind, package_name))?;
    Some(CargoPackageTargetFacts { name, kind, path })
}

#[cfg(feature = "provider-server")]
fn cargo_default_target_name(
    project_root: &Path,
    path: &Path,
    kind: &str,
    package_name: Option<&str>,
) -> Option<String> {
    let relative = path.strip_prefix(project_root).unwrap_or(path);
    if matches!(kind, "library" | "binary")
        && (relative == Path::new("src/lib.rs") || relative == Path::new("src/main.rs"))
    {
        return package_name.map(|name| name.replace('-', "_"));
    }
    if kind == "binary"
        && relative.file_name().is_some_and(|name| name == "main.rs")
        && relative.parent().and_then(Path::file_name) != Some(std::ffi::OsStr::new("src"))
    {
        return relative
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned);
    }
    relative
        .file_stem()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

fn manifest_test_target_files(project_root: &Path, test_targets: &[Product]) -> Vec<PathBuf> {
    manifest_product_target_files(project_root, test_targets, "test", None)
}

fn manifest_product_target_files(
    project_root: &Path,
    targets: &[Product],
    kind: &str,
    package_name: Option<&str>,
) -> Vec<PathBuf> {
    targets
        .iter()
        .filter_map(|target| completed_product_path(project_root, target, kind, package_name))
        .collect()
}

fn completed_product_path(
    project_root: &Path,
    target: &Product,
    kind: &str,
    package_name: Option<&str>,
) -> Option<PathBuf> {
    target
        .path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(|path| project_root.join(path))
        .or_else(|| implicit_product_path(project_root, target, kind, package_name))
}

fn implicit_product_path(
    project_root: &Path,
    target: &Product,
    kind: &str,
    package_name: Option<&str>,
) -> Option<PathBuf> {
    let package_target_name = package_name.map(|name| name.replace('-', "_"));
    if kind == "library" {
        return project_root
            .join("src/lib.rs")
            .is_file()
            .then(|| project_root.join("src/lib.rs"));
    }
    let name = target
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .or_else(|| {
            (kind == "binary")
                .then_some(package_target_name.as_deref())
                .flatten()
        })?;
    let mut candidates = Vec::new();
    match kind {
        "binary" => {
            if package_target_name.as_deref() == Some(name) {
                candidates.push(PathBuf::from("src/main.rs"));
            }
            candidates.push(PathBuf::from(format!("src/bin/{name}.rs")));
            candidates.push(PathBuf::from(format!("src/bin/{name}/main.rs")));
        }
        "test" | "example" | "bench" => {
            let directory = match kind {
                "test" => "tests",
                "example" => "examples",
                "bench" => "benches",
                _ => unreachable!(),
            };
            candidates.push(PathBuf::from(format!("{directory}/{name}.rs")));
            candidates.push(PathBuf::from(format!("{directory}/{name}/main.rs")));
        }
        _ => return None,
    }
    candidates
        .into_iter()
        .map(|path| project_root.join(path))
        .find(|path| path.is_file())
}

fn manifest_bench_targets(
    project_root: &Path,
    bench_targets: &[Product],
) -> Vec<CargoBenchTargetFacts> {
    bench_targets
        .iter()
        .filter_map(|target| {
            let name = target.name.as_deref()?.trim();
            if name.is_empty() {
                return None;
            }
            let path = completed_product_path(project_root, target, "bench", None)?;
            let mut required_features = target.required_features.clone();
            required_features.sort();
            required_features.dedup();
            Some(CargoBenchTargetFacts {
                name: name.to_string(),
                path,
                harness: target.harness,
                required_features,
            })
        })
        .collect()
}

fn manifest_references_harness(manifest: &Manifest) -> bool {
    dependency_table_references_harness(&manifest.dependencies)
        || dependency_table_references_harness(&manifest.dev_dependencies)
        || dependency_table_references_harness(&manifest.build_dependencies)
        || manifest.target.values().any(|target| {
            dependency_table_references_harness(&target.dependencies)
                || dependency_table_references_harness(&target.dev_dependencies)
                || dependency_table_references_harness(&target.build_dependencies)
        })
}

fn manifest_references_harness_build_dependency(manifest: &Manifest) -> bool {
    dependency_table_references_harness(&manifest.build_dependencies)
        || manifest
            .target
            .values()
            .any(|target| dependency_table_references_harness(&target.build_dependencies))
}

fn dependency_table_references_harness(dependencies: &DepsSet) -> bool {
    dependencies
        .iter()
        .any(|(name, value)| dependency_references_harness(name, value))
}

fn dependency_references_harness(name: &str, value: &Dependency) -> bool {
    dependency_name_is_harness(name) || value.package().is_some_and(dependency_name_is_harness)
}

fn dependency_name_is_harness(name: &str) -> bool {
    ASP_RUST_PACKAGE_NAMES.contains(&name)
}
