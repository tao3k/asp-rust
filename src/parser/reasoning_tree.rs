//! Project reasoning-tree facts derived from parsed Rust modules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::AspRustScope;

use super::module_tree::{
    RustModuleChildEdge, RustModuleSourceShadow, external_child_module_edges, is_module_tree_root,
    rust_module_tree_facts,
};
use super::{
    ParsedRustModule, RustSourcePathFacts, RustUseDeepRelativeImportSyntax, RustUseImportRootKind,
    RustUseImportSyntax, RustUseStatementSyntax, RustUseVisibilityKind, rust_source_path_facts,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustReasoningTreeFacts {
    pub(crate) package_root: PathBuf,
    pub(crate) source_roots: Vec<PathBuf>,
    pub(crate) package_entrypoints: Vec<PathBuf>,
    pub(crate) modules: Vec<RustReasoningModuleFacts>,
    pub(crate) owner_branches: Vec<RustReasoningOwnerBranchFacts>,
    pub(crate) owner_dependencies: Vec<RustReasoningOwnerDependencyFacts>,
    pub(crate) shadowed_module_sources: Vec<RustModuleSourceShadow>,
    pub(crate) unreachable_source_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustReasoningModuleFacts {
    pub(crate) path: PathBuf,
    pub(crate) source_path: RustSourcePathFacts,
    pub(crate) import_summary: RustReasoningImportFacts,
    pub(crate) public_api_summary: RustReasoningPublicApiFacts,
    pub(crate) is_source_module: bool,
    pub(crate) is_module_tree_root: bool,
    pub(crate) declared_child_edges: Vec<RustModuleChildEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustReasoningOwnerBranchFacts {
    pub(crate) path: PathBuf,
    pub(crate) owner_namespace: Vec<String>,
    pub(crate) roles: Vec<RustReasoningOwnerBranchRole>,
    pub(crate) import_summary: RustReasoningImportFacts,
    pub(crate) declared_child_edges: Vec<RustModuleChildEdge>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustReasoningImportFacts {
    pub(crate) total_imports: usize,
    pub(crate) crate_imports: usize,
    pub(crate) self_imports: usize,
    pub(crate) parent_imports: usize,
    pub(crate) external_imports: usize,
    pub(crate) absolute_imports: usize,
    pub(crate) unknown_imports: usize,
    pub(crate) glob_imports: usize,
    pub(crate) deep_relative_imports: usize,
    pub(crate) deep_relative_import_facts: Vec<RustReasoningDeepRelativeImportFacts>,
    pub(crate) prelude_imports: usize,
    pub(crate) test_context_imports: usize,
    pub(crate) production_external_imports: Vec<Vec<String>>,
    pub(crate) local_owner_imports: Vec<Vec<String>>,
    pub(crate) local_owner_dependencies: Vec<RustReasoningOwnerDependencyFacts>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustReasoningPublicApiFacts {
    pub(crate) public_items: usize,
    pub(crate) public_exports: usize,
    pub(crate) public_functions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustReasoningDeepRelativeImportFacts {
    pub(crate) line: usize,
    pub(crate) original_segments: Vec<String>,
    pub(crate) crate_segments: Option<Vec<String>>,
    pub(crate) parent_hops: usize,
    pub(crate) visibility: RustUseVisibilityKind,
    pub(crate) is_reexport: bool,
    pub(crate) is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RustReasoningOwnerDependencyFacts {
    pub(crate) source_path: PathBuf,
    pub(crate) source_namespace: Vec<String>,
    pub(crate) target_path: PathBuf,
    pub(crate) target_namespace: Vec<String>,
    pub(crate) via_root: RustUseImportRootKind,
    pub(crate) line: usize,
    pub(crate) is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RustReasoningOwnerDependencyKey {
    source_path: PathBuf,
    target_path: PathBuf,
    via_root: RustUseImportRootKind,
    is_test_context: bool,
}

struct RustReasoningImportAccumulators<'a> {
    summary: &'a mut RustReasoningImportFacts,
    local_owner_imports: &'a mut BTreeSet<Vec<String>>,
    local_owner_dependencies:
        &'a mut BTreeMap<RustReasoningOwnerDependencyKey, RustReasoningOwnerDependencyFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RustReasoningOwnerBranchRole {
    Root,
    Facade,
    Interface,
    Binary,
    PackageEntrypoint,
    RepeatedNamespace(Vec<String>),
    Branch,
}

impl RustReasoningTreeFacts {
    pub(crate) fn module(&self, path: &Path) -> Option<&RustReasoningModuleFacts> {
        self.modules.iter().find(|module| module.path == path)
    }
}

impl RustReasoningDeepRelativeImportFacts {
    pub(crate) fn rendered_path(&self) -> String {
        self.original_segments.join("::")
    }

    pub(crate) fn rendered_crate_path(&self) -> Option<String> {
        self.crate_segments
            .as_ref()
            .map(|segments| segments.join("::"))
    }
}

pub(crate) fn rust_reasoning_tree_facts(
    scope: &AspRustScope,
    modules: &[ParsedRustModule],
) -> RustReasoningTreeFacts {
    let module_tree = rust_module_tree_facts(&scope.source_paths, modules);
    let source_files = modules
        .iter()
        .filter(|module| is_under_any_dir(&module.report.path, &scope.source_paths))
        .map(|module| module.report.path.clone())
        .collect::<BTreeSet<_>>();
    let preliminary_modules = modules
        .iter()
        .map(|module| {
            let is_source_module = source_files.contains(&module.report.path);
            RustReasoningModuleFacts {
                path: module.report.path.clone(),
                source_path: rust_source_path_facts(
                    &scope.project_root,
                    &scope.source_paths,
                    &scope.test_paths,
                    &scope.package_paths,
                    &module.report.path,
                ),
                import_summary: RustReasoningImportFacts::default(),
                public_api_summary: public_api_summary(module),
                is_source_module,
                is_module_tree_root: is_source_module
                    && is_module_tree_root(&scope.source_paths, &module.report.path),
                declared_child_edges: if is_source_module {
                    external_child_module_edges(module, &source_files)
                } else {
                    Vec::new()
                },
            }
        })
        .collect::<Vec<_>>();
    let known_module_namespace_paths = known_module_namespace_paths(&preliminary_modules);
    let module_facts = preliminary_modules
        .into_iter()
        .zip(modules)
        .map(|(mut module_facts, module)| {
            module_facts.import_summary =
                import_summary(module, &module_facts, &known_module_namespace_paths);
            module_facts
        })
        .collect::<Vec<_>>();
    let owner_branches = owner_branch_facts(&module_facts);
    let owner_dependencies = owner_dependency_facts(&module_facts);
    RustReasoningTreeFacts {
        package_root: scope.project_root.clone(),
        source_roots: scope.source_paths.clone(),
        package_entrypoints: scope.package_paths.clone(),
        modules: module_facts,
        owner_branches,
        owner_dependencies,
        shadowed_module_sources: module_tree.shadowed_module_sources,
        unreachable_source_files: module_tree.unreachable_source_files,
    }
}

fn owner_branch_facts(modules: &[RustReasoningModuleFacts]) -> Vec<RustReasoningOwnerBranchFacts> {
    let mut branches = modules
        .iter()
        .filter(|module| module.is_source_module)
        .filter(|module| {
            module.is_module_tree_root
                || !module.declared_child_edges.is_empty()
                || module.source_path.is_special_entrypoint
                || !module.source_path.repeated_namespace_segments.is_empty()
        })
        .map(|module| RustReasoningOwnerBranchFacts {
            path: module.path.clone(),
            owner_namespace: module.source_path.namespace_components.clone(),
            roles: owner_branch_roles(module),
            import_summary: module.import_summary.clone(),
            declared_child_edges: module.declared_child_edges.clone(),
        })
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        right
            .roles
            .contains(&RustReasoningOwnerBranchRole::Root)
            .cmp(&left.roles.contains(&RustReasoningOwnerBranchRole::Root))
            .then_with(|| left.path.cmp(&right.path))
    });
    branches
}

fn owner_dependency_facts(
    modules: &[RustReasoningModuleFacts],
) -> Vec<RustReasoningOwnerDependencyFacts> {
    let mut dependencies = modules
        .iter()
        .flat_map(|module| module.import_summary.local_owner_dependencies.iter())
        .fold(BTreeMap::new(), |mut dependencies, dependency| {
            merge_owner_dependency(&mut dependencies, dependency.clone());
            dependencies
        })
        .into_values()
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| {
        left.source_namespace
            .len()
            .cmp(&right.source_namespace.len())
            .then_with(|| left.source_path.cmp(&right.source_path))
            .then_with(|| left.target_path.cmp(&right.target_path))
    });
    dependencies
}

fn known_module_namespace_paths(
    modules: &[RustReasoningModuleFacts],
) -> BTreeMap<Vec<String>, PathBuf> {
    modules
        .iter()
        .filter(|module| module.is_source_module)
        .filter(|module| !module.source_path.namespace_components.is_empty())
        .map(|module| {
            (
                module.source_path.namespace_components.clone(),
                module.path.clone(),
            )
        })
        .collect()
}

fn import_summary(
    module: &ParsedRustModule,
    module_facts: &RustReasoningModuleFacts,
    known_module_namespace_paths: &BTreeMap<Vec<String>, PathBuf>,
) -> RustReasoningImportFacts {
    let mut summary = RustReasoningImportFacts::default();
    let mut production_external_imports = BTreeSet::<Vec<String>>::new();
    let mut local_owner_imports = BTreeSet::<Vec<String>>::new();
    let mut local_owner_dependencies =
        BTreeMap::<RustReasoningOwnerDependencyKey, RustReasoningOwnerDependencyFacts>::new();
    for use_statement in &module.syntax_facts.use_statements {
        if !use_statement.context.is_inside_cfg_test_module {
            production_external_imports.extend(
                use_statement
                    .imports
                    .iter()
                    .filter(|import| {
                        matches!(
                            import.root_kind,
                            RustUseImportRootKind::External | RustUseImportRootKind::Absolute
                        )
                    })
                    .map(|import| import.segments.clone()),
            );
        }
        let mut accumulators = RustReasoningImportAccumulators {
            summary: &mut summary,
            local_owner_imports: &mut local_owner_imports,
            local_owner_dependencies: &mut local_owner_dependencies,
        };
        record_use_statement_imports(
            &mut accumulators,
            module_facts,
            use_statement,
            known_module_namespace_paths,
        );
    }
    summary.production_external_imports = production_external_imports.into_iter().collect();
    summary.local_owner_imports = local_owner_imports.into_iter().collect();
    summary.local_owner_dependencies = local_owner_dependencies.into_values().collect();
    summary
}

fn public_api_summary(module: &ParsedRustModule) -> RustReasoningPublicApiFacts {
    let mut summary = RustReasoningPublicApiFacts::default();
    for item in &module.syntax_facts.top_level_items {
        if item.is_public && item.kind != "mod" {
            summary.public_items += 1;
        }
        if item.is_public_use {
            summary.public_exports += 1;
        }
        if item.is_public && item.kind == "fn" {
            summary.public_functions += 1;
        }
    }
    summary
}

fn record_use_statement_imports(
    accumulators: &mut RustReasoningImportAccumulators<'_>,
    module_facts: &RustReasoningModuleFacts,
    use_statement: &RustUseStatementSyntax,
    known_module_namespace_paths: &BTreeMap<Vec<String>, PathBuf>,
) {
    let import_namespace = use_statement_namespace(
        &module_facts.source_path.namespace_components,
        &use_statement.context.enclosing_modules,
    );
    let import_count = use_statement.imports.len();
    accumulators.summary.total_imports += import_count;
    if use_statement.context.is_inside_cfg_test_module {
        accumulators.summary.test_context_imports += import_count;
    }
    for import in &use_statement.imports {
        record_import_fact(
            accumulators,
            module_facts,
            &import_namespace,
            use_statement,
            import,
            known_module_namespace_paths,
        );
    }
    record_deep_relative_import_facts(accumulators.summary, &import_namespace, use_statement);
}

fn record_import_fact(
    accumulators: &mut RustReasoningImportAccumulators<'_>,
    module_facts: &RustReasoningModuleFacts,
    import_namespace: &[String],
    use_statement: &RustUseStatementSyntax,
    import: &RustUseImportSyntax,
    known_module_namespace_paths: &BTreeMap<Vec<String>, PathBuf>,
) {
    record_import_root(accumulators.summary, import.root_kind);
    record_import_shape(accumulators.summary, import);
    if let Some(dependency) = local_owner_dependency(
        &module_facts.path,
        &module_facts.source_path.namespace_components,
        import_namespace,
        import,
        use_statement.line,
        use_statement.context.is_inside_cfg_test_module,
        known_module_namespace_paths,
    ) {
        accumulators
            .local_owner_imports
            .insert(dependency.target_namespace.clone());
        merge_owner_dependency(accumulators.local_owner_dependencies, dependency);
    }
}

fn record_import_root(summary: &mut RustReasoningImportFacts, root_kind: RustUseImportRootKind) {
    match root_kind {
        RustUseImportRootKind::Absolute => summary.absolute_imports += 1,
        RustUseImportRootKind::Crate => summary.crate_imports += 1,
        RustUseImportRootKind::SelfScope => summary.self_imports += 1,
        RustUseImportRootKind::Parent => summary.parent_imports += 1,
        RustUseImportRootKind::External => summary.external_imports += 1,
        RustUseImportRootKind::Unknown => summary.unknown_imports += 1,
    }
}

fn record_import_shape(summary: &mut RustReasoningImportFacts, import: &RustUseImportSyntax) {
    if import.is_glob {
        summary.glob_imports += 1;
    }
    if import.is_prelude_import {
        summary.prelude_imports += 1;
    }
}

fn record_deep_relative_import_facts(
    summary: &mut RustReasoningImportFacts,
    import_namespace: &[String],
    use_statement: &RustUseStatementSyntax,
) {
    summary.deep_relative_imports += use_statement.deep_relative_imports.len();
    summary
        .deep_relative_import_facts
        .extend(use_statement.deep_relative_imports.iter().map(|import| {
            deep_relative_import_fact(
                import_namespace,
                import,
                use_statement.context.is_inside_cfg_test_module,
            )
        }));
}

fn use_statement_namespace(
    module_namespace: &[String],
    enclosing_modules: &[String],
) -> Vec<String> {
    let mut namespace = module_namespace.to_vec();
    namespace.extend(enclosing_modules.iter().cloned());
    namespace
}

fn deep_relative_import_fact(
    current_namespace: &[String],
    import: &RustUseDeepRelativeImportSyntax,
    is_test_context: bool,
) -> RustReasoningDeepRelativeImportFacts {
    RustReasoningDeepRelativeImportFacts {
        line: import.line,
        original_segments: import.prefix_segments.clone(),
        crate_segments: crate_relative_deep_relative_import_segments(current_namespace, import),
        parent_hops: import.parent_hops,
        visibility: import.visibility.clone(),
        is_reexport: import.is_reexport,
        is_test_context,
    }
}

fn crate_relative_deep_relative_import_segments(
    current_namespace: &[String],
    import: &RustUseDeepRelativeImportSyntax,
) -> Option<Vec<String>> {
    let candidate = parent_import_candidate_namespace(
        current_namespace,
        &import.prefix_segments,
        import.parent_hops,
    )?;
    Some(crate_relative_segments(current_namespace, &candidate))
}

fn crate_relative_segments(current_namespace: &[String], candidate: &[String]) -> Vec<String> {
    let mut segments = vec!["crate".to_string()];
    let candidate_segments =
        if !current_namespace.is_empty() && candidate.first() == current_namespace.first() {
            &candidate[1..]
        } else {
            candidate
        };
    segments.extend(candidate_segments.iter().cloned());
    segments
}

fn local_owner_dependency(
    current_path: &Path,
    source_namespace: &[String],
    import_namespace: &[String],
    import: &RustUseImportSyntax,
    line: usize,
    is_test_context: bool,
    known_module_namespace_paths: &BTreeMap<Vec<String>, PathBuf>,
) -> Option<RustReasoningOwnerDependencyFacts> {
    let candidate = local_import_candidate_namespace(import_namespace, import)?;
    let target_namespace =
        longest_known_namespace_prefix(&candidate, known_module_namespace_paths)?;
    if target_namespace.len() <= 1 || target_namespace.as_slice() == source_namespace {
        return None;
    }
    let target_path = known_module_namespace_paths.get(&target_namespace)?.clone();
    Some(RustReasoningOwnerDependencyFacts {
        source_path: current_path.to_path_buf(),
        source_namespace: source_namespace.to_vec(),
        target_path,
        target_namespace,
        via_root: import.root_kind,
        line,
        is_test_context,
    })
}

fn merge_owner_dependency(
    dependencies: &mut BTreeMap<RustReasoningOwnerDependencyKey, RustReasoningOwnerDependencyFacts>,
    dependency: RustReasoningOwnerDependencyFacts,
) {
    let key = RustReasoningOwnerDependencyKey {
        source_path: dependency.source_path.clone(),
        target_path: dependency.target_path.clone(),
        via_root: dependency.via_root,
        is_test_context: dependency.is_test_context,
    };
    dependencies
        .entry(key)
        .and_modify(|existing| {
            if dependency.line < existing.line {
                *existing = dependency.clone();
            }
        })
        .or_insert(dependency);
}

fn local_import_candidate_namespace(
    current_namespace: &[String],
    import: &RustUseImportSyntax,
) -> Option<Vec<String>> {
    match import.root_kind {
        RustUseImportRootKind::Crate => {
            let root = current_namespace.first()?.clone();
            let mut namespace = vec![root];
            namespace.extend(import.segments.iter().skip(1).cloned());
            Some(namespace)
        }
        RustUseImportRootKind::SelfScope => {
            let mut namespace = current_namespace.to_vec();
            namespace.extend(import.segments.iter().skip(1).cloned());
            Some(namespace)
        }
        RustUseImportRootKind::Parent => parent_import_candidate_namespace(
            current_namespace,
            &import.segments,
            import.parent_hops,
        ),
        RustUseImportRootKind::Absolute
        | RustUseImportRootKind::External
        | RustUseImportRootKind::Unknown => None,
    }
}

fn parent_import_candidate_namespace(
    current_namespace: &[String],
    segments: &[String],
    parent_hops: usize,
) -> Option<Vec<String>> {
    if parent_hops > current_namespace.len() {
        return None;
    }
    let mut namespace = current_namespace
        .iter()
        .take(current_namespace.len() - parent_hops)
        .cloned()
        .collect::<Vec<_>>();
    namespace.extend(segments.iter().skip(parent_hops).cloned());
    Some(namespace)
}

fn longest_known_namespace_prefix(
    candidate: &[String],
    known_module_namespace_paths: &BTreeMap<Vec<String>, PathBuf>,
) -> Option<Vec<String>> {
    (1..=candidate.len()).rev().find_map(|length| {
        let prefix = candidate.iter().take(length).cloned().collect::<Vec<_>>();
        known_module_namespace_paths
            .contains_key(&prefix)
            .then_some(prefix)
    })
}

fn owner_branch_roles(module: &RustReasoningModuleFacts) -> Vec<RustReasoningOwnerBranchRole> {
    let mut roles = Vec::new();
    if module.is_module_tree_root {
        roles.push(RustReasoningOwnerBranchRole::Root);
    }
    if module.source_path.is_crate_facade {
        roles.push(RustReasoningOwnerBranchRole::Facade);
    }
    if module.source_path.is_interface_mod {
        roles.push(RustReasoningOwnerBranchRole::Interface);
    }
    if module.source_path.is_binary_entrypoint {
        roles.push(RustReasoningOwnerBranchRole::Binary);
    }
    if module.source_path.is_package_entrypoint {
        roles.push(RustReasoningOwnerBranchRole::PackageEntrypoint);
    }
    if !module.source_path.repeated_namespace_segments.is_empty() {
        roles.push(RustReasoningOwnerBranchRole::RepeatedNamespace(
            module
                .source_path
                .repeated_namespace_segments
                .iter()
                .cloned()
                .collect(),
        ));
    }
    if roles.is_empty() {
        roles.push(RustReasoningOwnerBranchRole::Branch);
    }
    roles
}

fn is_under_any_dir(path: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| path.starts_with(dir))
}
