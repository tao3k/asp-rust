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

struct WorkspacePackageCatalog {
    facts_by_root: PackageFactsByRoot,
    root_by_name: PackageRootsByName,
    discovered_package_root_count: usize,
    parsed_manifest_count: usize,
}

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

/// Observable parser work performed while deriving one workspace Build DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AspRustWorkspaceBuildDagMetrics {
    /// Cargo package roots discovered before workspace membership filtering.
    pub discovered_package_root_count: usize,
    /// Workspace package manifests admitted and parsed exactly once.
    pub admitted_package_count: usize,
    /// Total manifest parses: one workspace manifest plus every admitted package manifest.
    pub parsed_manifest_count: usize,
    /// Unique local dependency edges retained in the Build DAG.
    pub local_dependency_edge_count: usize,
}

/// One Build DAG together with the parser-work facts used by scenarios and benchmarks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AspRustWorkspaceBuildDagDerivation {
    /// Deterministic dependency-first Build DAG.
    pub build_dag: AspRustWorkspaceBuildDag,
    /// Exact parser-work counters for this derivation.
    pub metrics: AspRustWorkspaceBuildDagMetrics,
}

/// Derive the deterministic dependency-first Build DAG for one Cargo workspace instance.
pub fn asp_rust_workspace_build_dag(
    workspace_root: &Path,
    workspace_config: &AspRustConfig,
) -> Result<AspRustWorkspaceBuildDag, String> {
    asp_rust_workspace_build_dag_with_metrics(workspace_root, workspace_config)
        .map(|derivation| derivation.build_dag)
}

/// Derive a workspace Build DAG and expose exact parser-work counters.
pub fn asp_rust_workspace_build_dag_with_metrics(
    workspace_root: &Path,
    workspace_config: &AspRustConfig,
) -> Result<AspRustWorkspaceBuildDagDerivation, String> {
    let workspace_root = crate::path::normalize_lexical_path(workspace_root);
    let catalog = discover_dependency_graph_packages(&workspace_root, workspace_config)?;
    let dependencies_by_name =
        dependency_names_by_package(&catalog.facts_by_root, &catalog.root_by_name)?;
    let workspace_packages = catalog.root_by_name.keys().cloned().collect::<Vec<_>>();
    let order = dependency_first_package_order(&workspace_packages, &dependencies_by_name)?;
    let packages =
        materialize_dependency_graph_packages(order, &catalog.root_by_name, &dependencies_by_name);
    let metrics = AspRustWorkspaceBuildDagMetrics {
        discovered_package_root_count: catalog.discovered_package_root_count,
        admitted_package_count: packages.len(),
        parsed_manifest_count: catalog.parsed_manifest_count,
        local_dependency_edge_count: dependencies_by_name.values().map(Vec::len).sum(),
    };

    Ok(AspRustWorkspaceBuildDagDerivation {
        build_dag: AspRustWorkspaceBuildDag {
            schema_id: ASP_RUST_WORKSPACE_BUILD_DAG_SCHEMA_ID.to_string(),
            schema_version: ASP_RUST_WORKSPACE_BUILD_DAG_SCHEMA_VERSION.to_string(),
            workspace_root,
            packages,
        },
        metrics,
    })
}

/// Derive the Build DAG for the Cargo workspace owning `CARGO_MANIFEST_DIR`.
pub fn asp_rust_workspace_build_dag_from_env(
    workspace_config: &AspRustConfig,
) -> Result<AspRustWorkspaceBuildDag, String> {
    asp_rust_workspace_build_dag_from_env_with_metrics(workspace_config)
        .map(|derivation| derivation.build_dag)
}

/// Derive the Build DAG and parser-work metrics for the workspace owning `CARGO_MANIFEST_DIR`.
pub fn asp_rust_workspace_build_dag_from_env_with_metrics(
    workspace_config: &AspRustConfig,
) -> Result<AspRustWorkspaceBuildDagDerivation, String> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            "CARGO_MANIFEST_DIR is required for ASP Rust workspace policy".to_string()
        })?;
    let workspace_root = crate::parser::find_required_cargo_workspace_root(&manifest_dir)?;
    asp_rust_workspace_build_dag_with_metrics(&workspace_root, workspace_config)
}

fn discover_dependency_graph_packages(
    workspace_root: &Path,
    workspace_config: &AspRustConfig,
) -> Result<WorkspacePackageCatalog, String> {
    let discovered_roots = crate::discovery::discover_cargo_package_roots(
        workspace_root,
        &workspace_config.ignored_dir_names,
        &workspace_config.include_hidden_dir_names,
    );
    if discovered_roots.is_empty() {
        return Err(format!(
            "Cargo dependency graph contains no packages: {}",
            workspace_root.display()
        ));
    }
    let discovered_package_root_count = discovered_roots.len();
    let package_roots =
        crate::parser::retain_cargo_workspace_member_roots(workspace_root, discovered_roots)?;
    let graph_facts = crate::parser::parse_required_cargo_workspace_package_graph_facts(
        &package_roots,
        workspace_root,
    )?;
    let mut facts_by_root = BTreeMap::new();
    let mut root_by_name = BTreeMap::new();
    for (root, facts) in graph_facts.packages {
        let root = crate::path::normalize_lexical_path(&root);
        insert_unique_dependency_graph_package(&mut root_by_name, &root, &facts)?;
        facts_by_root.insert(root, facts);
    }
    Ok(WorkspacePackageCatalog {
        facts_by_root,
        root_by_name,
        discovered_package_root_count,
        parsed_manifest_count: graph_facts.parsed_manifest_count,
    })
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
            let dependencies = local_dependency_names(facts, facts_by_root);
            debug_assert_eq!(root_by_name.get(&facts.package_name), Some(root));
            Ok((facts.package_name.clone(), dependencies))
        })
        .collect()
}

fn local_dependency_names(
    facts: &crate::parser::CargoPackageGraphFacts,
    facts_by_root: &BTreeMap<PathBuf, crate::parser::CargoPackageGraphFacts>,
) -> Vec<String> {
    let mut dependencies = facts
        .local_dependency_roots
        .iter()
        .filter_map(|dependency_root| {
            let dependency_root = crate::path::normalize_lexical_path(dependency_root);
            facts_by_root
                .get(&dependency_root)
                .map(|dependency| dependency.package_name.clone())
        })
        .collect::<Vec<_>>();
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

fn dependency_first_package_order(
    workspace_packages: &[String],
    dependencies_by_name: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for package in workspace_packages {
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
