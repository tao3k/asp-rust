//! Derives the deterministic Cargo workspace Build DAG used by downstream policy execution.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::AspRustConfig;

/// Stable schema id for a dependency-graph-derived workspace Build DAG.
pub const ASP_RUST_WORKSPACE_BUILD_DAG_SCHEMA_ID: &str = "asp-rust.workspace-build-dag";

/// Stable workspace Build DAG schema version.
pub const ASP_RUST_WORKSPACE_BUILD_DAG_SCHEMA_VERSION: &str = "1";

type PackageFactsByRoot = BTreeMap<PathBuf, crate::parser::CargoPackageGraphFacts>;
type PackageRootsByName = BTreeMap<String, PathBuf>;
type WorkspacePackageCatalog = (PackageFactsByRoot, PackageRootsByName);

/// Deterministic dependency-first package execution DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AspRustWorkspaceBuildDag {
    /// Stable schema id.
    pub schema_id: String,
    /// Stable schema version.
    pub schema_version: String,
    /// Workspace root that owns the graph.
    pub workspace_root: PathBuf,
    /// Caller-selected roots whose local dependency closure is admitted.
    pub selected_packages: Vec<String>,
    /// Dependency-first packages, each present exactly once.
    pub packages: Vec<AspRustWorkspaceBuildDagPackage>,
}

/// One package atom in a dependency-graph-derived Build DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AspRustWorkspaceBuildDagPackage {
    /// Cargo package name.
    pub package_name: String,
    /// Normalized Cargo package root.
    pub package_root: PathBuf,
    /// Direct local dependency package names.
    pub local_dependencies: Vec<String>,
    /// Dependency-first execution index.
    pub execution_index: usize,
}

/// Derive the deterministic dependency-first Build DAG for selected Cargo packages.
pub fn asp_rust_workspace_build_dag(
    workspace_root: &Path,
    workspace_config: &AspRustConfig,
    selected_packages: impl IntoIterator<Item = impl Into<String>>,
) -> Result<AspRustWorkspaceBuildDag, String> {
    let workspace_root = crate::path::normalize_lexical_path(workspace_root);
    let (facts_by_root, root_by_name) =
        discover_dependency_graph_packages(&workspace_root, workspace_config)?;
    let dependencies_by_name = dependency_names_by_package(&facts_by_root, &root_by_name)?;
    let selected = normalize_selected_packages(selected_packages, &root_by_name)?;
    let order = dependency_first_package_order(&selected, &dependencies_by_name)?;
    let packages =
        materialize_dependency_graph_packages(order, &root_by_name, &dependencies_by_name);

    Ok(AspRustWorkspaceBuildDag {
        schema_id: ASP_RUST_WORKSPACE_BUILD_DAG_SCHEMA_ID.to_string(),
        schema_version: ASP_RUST_WORKSPACE_BUILD_DAG_SCHEMA_VERSION.to_string(),
        workspace_root,
        selected_packages: selected,
        packages,
    })
}

fn discover_dependency_graph_packages(
    workspace_root: &Path,
    workspace_config: &AspRustConfig,
) -> Result<WorkspacePackageCatalog, String> {
    let package_roots = crate::discovery::discover_cargo_package_roots(
        workspace_root,
        &workspace_config.ignored_dir_names,
        &workspace_config.include_hidden_dir_names,
    );
    if package_roots.is_empty() {
        return Err(format!(
            "Cargo dependency graph contains no packages: {}",
            workspace_root.display()
        ));
    }
    let mut facts_by_root = BTreeMap::new();
    let mut root_by_name = BTreeMap::new();
    for root in package_roots {
        let root = crate::path::normalize_lexical_path(&root);
        let facts = crate::parser::parse_required_cargo_package_graph_facts(&root, workspace_root)?;
        insert_unique_dependency_graph_package(&mut root_by_name, &root, &facts)?;
        facts_by_root.insert(root, facts);
    }
    Ok((facts_by_root, root_by_name))
}

fn insert_unique_dependency_graph_package(
    root_by_name: &mut BTreeMap<String, PathBuf>,
    root: &Path,
    facts: &crate::parser::CargoPackageGraphFacts,
) -> Result<(), String> {
    let Some(previous) = root_by_name.insert(facts.package_name.clone(), root.to_path_buf()) else {
        return Ok(());
    };
    Err(format!(
        "Cargo dependency graph has duplicate local package name `{}` at {} and {}",
        facts.package_name,
        previous.display(),
        root.display()
    ))
}

fn dependency_names_by_package(
    facts_by_root: &BTreeMap<PathBuf, crate::parser::CargoPackageGraphFacts>,
    root_by_name: &BTreeMap<String, PathBuf>,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    facts_by_root
        .iter()
        .map(|(root, facts)| {
            let dependencies = local_dependency_names(facts, facts_by_root)?;
            debug_assert_eq!(root_by_name.get(&facts.package_name), Some(root));
            Ok((facts.package_name.clone(), dependencies))
        })
        .collect()
}

fn local_dependency_names(
    facts: &crate::parser::CargoPackageGraphFacts,
    facts_by_root: &BTreeMap<PathBuf, crate::parser::CargoPackageGraphFacts>,
) -> Result<Vec<String>, String> {
    let mut dependencies = facts
        .local_dependency_roots
        .iter()
        .map(|dependency_root| {
            let dependency_root = crate::path::normalize_lexical_path(dependency_root);
            facts_by_root
                .get(&dependency_root)
                .map(|dependency| dependency.package_name.clone())
                .ok_or_else(|| {
                    format!(
                        "Cargo dependency graph package `{}` references unadmitted local dependency {}",
                        facts.package_name,
                        dependency_root.display()
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    dependencies.sort();
    dependencies.dedup();
    Ok(dependencies)
}

fn normalize_selected_packages(
    selected_packages: impl IntoIterator<Item = impl Into<String>>,
    root_by_name: &BTreeMap<String, PathBuf>,
) -> Result<Vec<String>, String> {
    let mut selected = selected_packages
        .into_iter()
        .map(Into::into)
        .collect::<Vec<String>>();
    if selected.is_empty() {
        selected.extend(root_by_name.keys().cloned());
    }
    selected.sort();
    selected.dedup();
    if let Some(package) = selected
        .iter()
        .find(|package| !root_by_name.contains_key(*package))
    {
        return Err(format!(
            "Cargo dependency graph selection references unknown package `{package}`"
        ));
    }
    Ok(selected)
}

fn dependency_first_package_order(
    selected: &[String],
    dependencies_by_name: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for package in selected {
        visit_dependency_graph_package(
            package,
            dependencies_by_name,
            &mut visiting,
            &mut visited,
            &mut order,
        )?;
    }
    Ok(order)
}

fn materialize_dependency_graph_packages(
    order: Vec<String>,
    root_by_name: &BTreeMap<String, PathBuf>,
    dependencies_by_name: &BTreeMap<String, Vec<String>>,
) -> Vec<AspRustWorkspaceBuildDagPackage> {
    order
        .into_iter()
        .enumerate()
        .map(
            |(execution_index, package_name)| AspRustWorkspaceBuildDagPackage {
                package_root: root_by_name
                    .get(&package_name)
                    .expect("Build DAG package root")
                    .clone(),
                local_dependencies: dependencies_by_name
                    .get(&package_name)
                    .expect("Build DAG package dependencies")
                    .clone(),
                package_name,
                execution_index,
            },
        )
        .collect()
}

fn visit_dependency_graph_package(
    package: &str,
    dependencies_by_name: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) -> Result<(), String> {
    if visited.contains(package) {
        return Ok(());
    }
    if !visiting.insert(package.to_string()) {
        return Err(format!(
            "Cargo local dependency graph contains a cycle through `{package}`"
        ));
    }
    let dependencies = dependencies_by_name
        .get(package)
        .ok_or_else(|| format!("Cargo dependency graph package is missing: `{package}`"))?;
    for dependency in dependencies {
        visit_dependency_graph_package(dependency, dependencies_by_name, visiting, visited, order)?;
    }
    visiting.remove(package);
    visited.insert(package.to_string());
    order.push(package.to_string());
    Ok(())
}
