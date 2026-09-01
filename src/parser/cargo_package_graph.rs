//! Parses Cargo workspace membership and local dependency edges for Build DAG derivation.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use cargo_toml::{Dependency, DepsSet, Manifest};
use globset::{Glob, GlobSet, GlobSetBuilder};

pub(crate) struct CargoPackageGraphFacts {
    pub(crate) package_name: String,
    pub(crate) local_dependency_roots: Vec<PathBuf>,
}

pub(crate) fn find_required_cargo_workspace_root(start: &Path) -> Result<PathBuf, String> {
    for candidate in start.ancestors() {
        let manifest_path = candidate.join("Cargo.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let manifest = Manifest::from_path(&manifest_path).map_err(|error| {
            format!(
                "parse candidate Cargo workspace {}: {error}",
                manifest_path.display()
            )
        })?;
        if manifest.workspace.is_some() {
            return Ok(crate::path::normalize_lexical_path(candidate));
        }
    }
    Err(format!(
        "no Cargo workspace manifest owns {}",
        start.display()
    ))
}

pub(crate) fn retain_cargo_workspace_member_roots(
    workspace_root: &Path,
    discovered_roots: Vec<PathBuf>,
) -> Result<Vec<PathBuf>, String> {
    let manifest_path = workspace_root.join("Cargo.toml");
    let manifest = Manifest::from_path(&manifest_path).map_err(|error| {
        format!(
            "parse Cargo workspace membership {}: {error}",
            manifest_path.display()
        )
    })?;
    let workspace = manifest.workspace.as_ref().ok_or_else(|| {
        format!(
            "Cargo Build DAG root is not a workspace: {}",
            manifest_path.display()
        )
    })?;
    let members = compile_workspace_patterns(&workspace.members, "member")?;
    let excludes = compile_workspace_patterns(&workspace.exclude, "exclude")?;
    let normalized_workspace_root = crate::path::normalize_lexical_path(workspace_root);
    let mut retained = discovered_roots
        .into_iter()
        .map(|root| crate::path::normalize_lexical_path(&root))
        .filter(|root| {
            if root == &normalized_workspace_root {
                return manifest.package.is_some();
            }
            let Ok(relative) = root.strip_prefix(&normalized_workspace_root) else {
                return false;
            };
            members.is_match(relative) && !excludes.is_match(relative)
        })
        .collect::<Vec<_>>();
    retained.sort();
    retained.dedup();
    if retained.is_empty() {
        return Err(format!(
            "Cargo workspace contains no admitted package members: {}",
            manifest_path.display()
        ));
    }
    Ok(retained)
}

fn compile_workspace_patterns(patterns: &[String], kind: &str) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(
            Glob::new(pattern)
                .map_err(|error| format!("invalid Cargo workspace {kind} `{pattern}`: {error}"))?,
        );
    }
    builder
        .build()
        .map_err(|error| format!("compile Cargo workspace {kind} patterns: {error}"))
}

pub(crate) fn parse_required_cargo_package_graph_facts(
    package_root: &Path,
    workspace_root: &Path,
) -> Result<CargoPackageGraphFacts, String> {
    let manifest_path = package_root.join("Cargo.toml");
    let manifest = Manifest::from_path(&manifest_path).map_err(|error| {
        format!(
            "parse Cargo dependency graph package {}: {error}",
            manifest_path.display()
        )
    })?;
    let package_name = manifest
        .package
        .as_ref()
        .map(|package| package.name.clone())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "Cargo graph package has no package.name: {}",
                manifest_path.display()
            )
        })?;
    let workspace_manifest =
        Manifest::from_path(workspace_root.join("Cargo.toml")).map_err(|error| {
            format!(
                "parse Cargo dependency graph workspace {}: {error}",
                workspace_root.join("Cargo.toml").display()
            )
        })?;
    let workspace_dependencies = workspace_manifest
        .workspace
        .as_ref()
        .map(|workspace| &workspace.dependencies);
    let mut roots = BTreeSet::new();
    collect_graph_dependency_roots(
        package_root,
        workspace_root,
        &manifest.dependencies,
        workspace_dependencies,
        &mut roots,
    );
    collect_graph_dependency_roots(
        package_root,
        workspace_root,
        &manifest.dev_dependencies,
        workspace_dependencies,
        &mut roots,
    );
    collect_graph_dependency_roots(
        package_root,
        workspace_root,
        &manifest.build_dependencies,
        workspace_dependencies,
        &mut roots,
    );
    for target in manifest.target.values() {
        collect_graph_dependency_roots(
            package_root,
            workspace_root,
            &target.dependencies,
            workspace_dependencies,
            &mut roots,
        );
        collect_graph_dependency_roots(
            package_root,
            workspace_root,
            &target.dev_dependencies,
            workspace_dependencies,
            &mut roots,
        );
        collect_graph_dependency_roots(
            package_root,
            workspace_root,
            &target.build_dependencies,
            workspace_dependencies,
            &mut roots,
        );
    }
    Ok(CargoPackageGraphFacts {
        package_name,
        local_dependency_roots: roots.into_iter().collect(),
    })
}

fn collect_graph_dependency_roots(
    package_root: &Path,
    workspace_root: &Path,
    dependencies: &DepsSet,
    workspace_dependencies: Option<&DepsSet>,
    roots: &mut BTreeSet<PathBuf>,
) {
    for (dependency_name, dependency) in dependencies {
        let resolved = match dependency {
            Dependency::Inherited(inherited) if inherited.workspace => workspace_dependencies
                .and_then(|dependencies| dependencies.get(dependency_name))
                .and_then(|dependency| dependency_path_root(workspace_root, dependency)),
            _ => dependency_path_root(package_root, dependency),
        };
        if let Some(root) = resolved {
            roots.insert(crate::path::normalize_lexical_path(&root));
        }
    }
}

fn dependency_path_root(base_root: &Path, dependency: &Dependency) -> Option<PathBuf> {
    let Dependency::Detailed(detail) = dependency else {
        return None;
    };
    detail.path.as_ref().map(|path| base_root.join(path))
}
