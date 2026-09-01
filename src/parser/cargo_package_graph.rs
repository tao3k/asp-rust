use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use cargo_toml::{Dependency, DepsSet, Manifest};

pub(crate) struct CargoPackageGraphFacts {
    pub(crate) package_name: String,
    pub(crate) local_dependency_roots: Vec<PathBuf>,
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
