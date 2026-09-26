//! Cargo dependency facts owned by the parser layer.

use std::path::Path;

use cargo_toml::{Dependency, DepsSet, Manifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CargoDependencyKind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoDependencyFacts {
    pub(crate) dependency_key: String,
    pub(crate) import_name: String,
    pub(crate) package_name: String,
    pub(crate) version_req: Option<String>,
    pub(crate) kind: CargoDependencyKind,
    pub(crate) target: Option<String>,
    pub(crate) optional: bool,
    pub(crate) features: Vec<String>,
}

pub(crate) fn parse_cargo_dependency_facts(project_root: &Path) -> Vec<CargoDependencyFacts> {
    let Some(manifest) = read_manifest(project_root) else {
        return Vec::new();
    };
    cargo_dependency_facts_from_manifest(&manifest)
}

pub(super) fn cargo_dependency_facts_from_manifest(
    manifest: &Manifest,
) -> Vec<CargoDependencyFacts> {
    let mut dependencies = manifest_dependency_facts(manifest);
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

fn read_manifest(project_root: &Path) -> Option<Manifest> {
    let manifest_path = project_root.join("Cargo.toml");
    Manifest::from_path(&manifest_path).ok()
}

fn manifest_dependency_facts(manifest: &Manifest) -> Vec<CargoDependencyFacts> {
    let mut dependencies = Vec::new();
    dependencies.extend(dependency_table_facts(
        CargoDependencyKind::Normal,
        None,
        &manifest.dependencies,
    ));
    dependencies.extend(dependency_table_facts(
        CargoDependencyKind::Dev,
        None,
        &manifest.dev_dependencies,
    ));
    dependencies.extend(dependency_table_facts(
        CargoDependencyKind::Build,
        None,
        &manifest.build_dependencies,
    ));
    for (target_name, target) in &manifest.target {
        dependencies.extend(dependency_table_facts(
            CargoDependencyKind::Normal,
            Some(target_name),
            &target.dependencies,
        ));
        dependencies.extend(dependency_table_facts(
            CargoDependencyKind::Dev,
            Some(target_name),
            &target.dev_dependencies,
        ));
        dependencies.extend(dependency_table_facts(
            CargoDependencyKind::Build,
            Some(target_name),
            &target.build_dependencies,
        ));
    }
    dependencies
}

fn dependency_table_facts(
    kind: CargoDependencyKind,
    target: Option<&str>,
    dependencies: &DepsSet,
) -> Vec<CargoDependencyFacts> {
    dependencies
        .iter()
        .map(|(name, dependency)| dependency_fact(name, dependency, kind, target))
        .collect()
}

fn dependency_fact(
    name: &str,
    dependency: &Dependency,
    kind: CargoDependencyKind,
    target: Option<&str>,
) -> CargoDependencyFacts {
    let mut features = dependency.req_features().to_vec();
    features.sort();
    features.dedup();
    let package_name = dependency.package().unwrap_or(name).to_string();
    let version_req = dependency.try_req().ok().map(|req| req.to_string());
    CargoDependencyFacts {
        dependency_key: name.to_string(),
        import_name: name.replace('-', "_"),
        package_name,
        version_req,
        kind,
        target: target.map(compact_cfg_expression),
        optional: dependency.optional(),
        features,
    }
}

fn compact_cfg_expression(expression: &str) -> String {
    expression
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
