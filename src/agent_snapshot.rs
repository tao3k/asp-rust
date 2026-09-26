//! Low-noise agent snapshot rendering from parser reasoning-tree facts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::discovery::{asp_rust_scope, discover_cargo_package_roots, discover_rust_files};
use crate::model::AspRustConfig;
use crate::parser::{
    RustModuleChildEdge, RustReasoningImportFacts, RustReasoningOwnerBranchFacts,
    RustReasoningOwnerBranchRole, RustReasoningOwnerDependencyFacts, parse_rust_file,
    rust_reasoning_tree_facts,
};
use crate::rules::evaluate_default_rule_packs_with_config;
use crate::{AspRustFinding, AspRustScope, RustDiagnosticSeverity};

const MAX_AGENT_SNAPSHOT_BRANCH_LINES: usize = 24;
const MAX_AGENT_SNAPSHOT_CHILD_EDGES: usize = 8;

/// Render a compact project-structure snapshot for repair-oriented agents.
///
/// The snapshot is derived from parser reasoning-tree facts and intentionally
/// avoids the full `AspRustReport` JSON shape.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn render_asp_rust_agent_snapshot(project_root: &Path) -> Result<String, String> {
    render_asp_rust_agent_snapshot_with_config(project_root, &AspRustConfig::default())
}

/// Render a compact project-structure snapshot with explicit harness config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn render_asp_rust_agent_snapshot_with_config(
    project_root: &Path,
    config: &AspRustConfig,
) -> Result<String, String> {
    if !project_root.exists() {
        return Err(format!(
            "project root does not exist: {}",
            project_root.display()
        ));
    }
    let package_roots = discover_cargo_package_roots(
        project_root,
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    );
    let package_roots = if should_run_member_scopes(project_root, &package_roots) {
        package_roots
    } else {
        vec![project_root.to_path_buf()]
    };
    let include_package_heading = package_roots.len() > 1;
    let package_snapshots = package_roots
        .into_iter()
        .filter_map(|package_root| {
            render_package_snapshot_for_root(
                project_root,
                &package_root,
                config,
                include_package_heading,
            )
        })
        .collect::<Vec<_>>();
    Ok(package_snapshots.join("\n"))
}

fn render_package_snapshot_for_root(
    snapshot_root: &Path,
    package_root: &Path,
    config: &AspRustConfig,
    include_package_heading: bool,
) -> Option<String> {
    let scope = asp_rust_scope(
        package_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let parsed_modules = parse_scope(&scope, config);
    let findings = evaluate_default_rule_packs_with_config(Some(&scope), &parsed_modules, config);
    let package_snapshot = render_package_snapshot(
        snapshot_root,
        &scope,
        &parsed_modules,
        &findings,
        include_package_heading,
    );
    (!package_snapshot.is_empty()).then_some(package_snapshot)
}

fn render_package_snapshot(
    snapshot_root: &Path,
    scope: &AspRustScope,
    parsed_modules: &[crate::parser::ParsedRustModule],
    findings: &[AspRustFinding],
    include_package_heading: bool,
) -> String {
    let reasoning_tree = rust_reasoning_tree_facts(scope, parsed_modules);
    let source_module_count = reasoning_tree
        .modules
        .iter()
        .filter(|module| module.is_source_module)
        .count();
    let root_count = reasoning_tree
        .modules
        .iter()
        .filter(|module| module.is_module_tree_root)
        .count();
    let branch_count = reasoning_tree
        .modules
        .iter()
        .filter(|module| module.is_source_module && !module.declared_child_edges.is_empty())
        .count();
    let owner_dependencies = reasoning_tree
        .owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .collect::<Vec<_>>();
    if source_module_count == 0 && findings.is_empty() {
        return String::new();
    }
    let mut rendered = String::new();
    if include_package_heading {
        let _ = writeln!(
            rendered,
            "pkg {}",
            display_project_path(snapshot_root, &reasoning_tree.package_root)
        );
    }
    let _ = writeln!(
        rendered,
        "{}",
        display_module_summary(
            source_module_count,
            root_count,
            branch_count,
            owner_dependencies.len(),
            reasoning_tree.shadowed_module_sources.len(),
            reasoning_tree.unreachable_source_files.len(),
        )
    );
    let branch_lines = reasoning_tree
        .owner_branches
        .iter()
        .map(|branch| display_owner_branch_line(&reasoning_tree.package_root, branch))
        .collect::<Vec<_>>();
    if !branch_lines.is_empty() {
        rendered.push_str("OwnerBranches:\n");
        rendered.push_str(
            &compact_lines(
                &branch_lines,
                MAX_AGENT_SNAPSHOT_BRANCH_LINES,
                "owner branches",
            )
            .join("\n"),
        );
        rendered.push('\n');
    }
    let dependency_lines =
        display_owner_dependency_lines(&reasoning_tree.package_root, &owner_dependencies);
    if !dependency_lines.is_empty() {
        rendered.push_str("OwnerDependencies:\n");
        rendered.push_str(&dependency_lines.join("\n"));
        rendered.push('\n');
    }
    let finding_lines = grouped_findings(&reasoning_tree.package_root, findings);
    if !finding_lines.is_empty() {
        rendered.push_str("FindingGroups:\n");
        rendered.push_str(&finding_lines.join("\n"));
        rendered.push('\n');
    }
    rendered
}

fn display_module_summary(
    source_module_count: usize,
    root_count: usize,
    branch_count: usize,
    dependency_count: usize,
    shadowed_count: usize,
    orphaned_count: usize,
) -> String {
    let mut parts = vec![format!("source={source_module_count}")];
    push_metric_if(&mut parts, "roots", root_count, root_count > 1);
    push_metric_if(&mut parts, "branches", branch_count, branch_count > 0);
    push_metric_if(&mut parts, "deps", dependency_count, dependency_count > 0);
    push_metric_if(&mut parts, "shadowed", shadowed_count, shadowed_count > 0);
    push_metric_if(&mut parts, "orphaned", orphaned_count, orphaned_count > 0);
    format!("Modules: {}", parts.join(" "))
}

fn push_metric_if(parts: &mut Vec<String>, label: &str, count: usize, should_render: bool) {
    if should_render {
        parts.push(format!("{label}={count}"));
    }
}

fn parse_scope(
    scope: &AspRustScope,
    config: &AspRustConfig,
) -> Vec<crate::parser::ParsedRustModule> {
    discover_rust_files(
        &scope.monitored_paths(),
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    )
    .into_iter()
    .map(|path| parse_rust_file(&path))
    .collect()
}

fn owner_branch_role_labels(branch: &RustReasoningOwnerBranchFacts) -> Vec<String> {
    branch
        .roles
        .iter()
        .map(|role| match role {
            RustReasoningOwnerBranchRole::Root => "root".to_string(),
            RustReasoningOwnerBranchRole::Facade => "facade".to_string(),
            RustReasoningOwnerBranchRole::Interface => "interface".to_string(),
            RustReasoningOwnerBranchRole::Binary => "binary".to_string(),
            RustReasoningOwnerBranchRole::PackageEntrypoint => "package-entrypoint".to_string(),
            RustReasoningOwnerBranchRole::RepeatedNamespace(segments) => {
                format!("repeated:{}", segments.join(","))
            }
            RustReasoningOwnerBranchRole::Branch => "branch".to_string(),
        })
        .collect()
}

fn display_owner_namespace(branch: &RustReasoningOwnerBranchFacts) -> String {
    if branch.owner_namespace.is_empty() {
        return "-".to_string();
    }
    branch.owner_namespace.join("/")
}

fn display_import_summary(summary: &RustReasoningImportFacts) -> String {
    if summary.total_imports == 0 {
        return String::new();
    }
    let mut parts = Vec::new();
    push_count(&mut parts, "crate", summary.crate_imports);
    push_count(&mut parts, "self", summary.self_imports);
    push_count(&mut parts, "parent", summary.parent_imports);
    push_count(&mut parts, "external", summary.external_imports);
    push_count(&mut parts, "absolute", summary.absolute_imports);
    push_count(&mut parts, "unknown", summary.unknown_imports);
    push_count(&mut parts, "glob", summary.glob_imports);
    push_count(&mut parts, "deep", summary.deep_relative_imports);
    push_count(&mut parts, "prelude", summary.prelude_imports);
    push_count(&mut parts, "test", summary.test_context_imports);
    format!(" imports={}", parts.join(","))
}

fn display_owner_branch_line(
    package_root: &Path,
    branch: &RustReasoningOwnerBranchFacts,
) -> String {
    let mut line = format!(
        " - {} [{}] owner={}",
        display_project_path(package_root, &branch.path),
        owner_branch_role_labels(branch).join(", "),
        display_owner_namespace(branch) + &display_import_summary(&branch.import_summary),
    );
    if !branch.declared_child_edges.is_empty() {
        let _ = write!(
            line,
            " -> {}",
            display_child_edges(package_root, &branch.declared_child_edges)
        );
    }
    line
}

fn push_count(parts: &mut Vec<String>, label: &str, count: usize) {
    if count > 0 {
        parts.push(format!("{label}:{count}"));
    }
}

fn display_owner_dependency_lines(
    package_root: &Path,
    dependencies: &[&RustReasoningOwnerDependencyFacts],
) -> Vec<String> {
    let fan_out_groups = owner_dependency_groups(dependencies, OwnerDependencyDirection::FanOut);
    let fan_in_groups = owner_dependency_groups(dependencies, OwnerDependencyDirection::FanIn);
    if fan_in_groups.len() < dependencies.len() && fan_in_groups.len() < fan_out_groups.len() {
        return fan_in_groups
            .into_iter()
            .map(|((target, root), sources)| {
                format!(
                    " - {} <--{}-- {}",
                    display_project_path(package_root, &target),
                    import_root_label(root),
                    display_project_paths(package_root, sources),
                )
            })
            .collect();
    }
    if fan_out_groups.len() == dependencies.len() {
        return dependencies
            .iter()
            .map(|dependency| display_owner_dependency(package_root, dependency))
            .collect();
    }
    fan_out_groups
        .into_iter()
        .map(|((source, root), targets)| {
            format!(
                " - {} --{}--> {}",
                display_project_path(package_root, &source),
                import_root_label(root),
                display_project_paths(package_root, targets),
            )
        })
        .collect()
}

fn display_owner_dependency(
    package_root: &Path,
    dependency: &RustReasoningOwnerDependencyFacts,
) -> String {
    format!(
        " - {} --{}--> {}",
        display_project_path(package_root, &dependency.source_path),
        import_root_label(dependency.via_root),
        display_project_path(package_root, &dependency.target_path)
    )
}

#[derive(Debug, Clone, Copy)]
enum OwnerDependencyDirection {
    FanOut,
    FanIn,
}

fn owner_dependency_groups(
    dependencies: &[&RustReasoningOwnerDependencyFacts],
    direction: OwnerDependencyDirection,
) -> BTreeMap<(PathBuf, crate::parser::RustUseImportRootKind), BTreeSet<PathBuf>> {
    let mut groups =
        BTreeMap::<(PathBuf, crate::parser::RustUseImportRootKind), BTreeSet<PathBuf>>::new();
    for dependency in dependencies {
        let (owner, peer) = match direction {
            OwnerDependencyDirection::FanOut => (&dependency.source_path, &dependency.target_path),
            OwnerDependencyDirection::FanIn => (&dependency.target_path, &dependency.source_path),
        };
        groups
            .entry((owner.clone(), dependency.via_root))
            .or_default()
            .insert(peer.clone());
    }
    groups
}

fn display_project_paths(package_root: &Path, paths: BTreeSet<PathBuf>) -> String {
    paths
        .into_iter()
        .map(|path| display_project_path(package_root, &path))
        .collect::<Vec<_>>()
        .join(", ")
}

fn import_root_label(root: crate::parser::RustUseImportRootKind) -> &'static str {
    match root {
        crate::parser::RustUseImportRootKind::Absolute => "absolute",
        crate::parser::RustUseImportRootKind::Crate => "crate",
        crate::parser::RustUseImportRootKind::SelfScope => "self",
        crate::parser::RustUseImportRootKind::Parent => "parent",
        crate::parser::RustUseImportRootKind::External => "external",
        crate::parser::RustUseImportRootKind::Unknown => "unknown",
    }
}

fn grouped_findings(package_root: &Path, findings: &[AspRustFinding]) -> Vec<String> {
    let mut groups = BTreeMap::<(RustDiagnosticSeverity, String, String), FindingGroup>::new();
    for finding in findings {
        let key = (
            finding.severity,
            finding.rule_id.clone(),
            finding.title.clone(),
        );
        let group = groups.entry(key).or_insert_with(|| FindingGroup {
            count: 0,
            first_path: finding.location.path.clone(),
        });
        group.count += 1;
    }
    groups
        .into_iter()
        .map(|((severity, rule_id, title), group)| {
            let first_path = group.first_path.as_ref().map_or_else(
                || "<memory>".to_string(),
                |path| display_project_path(package_root, path),
            );
            format!(
                " - {} {} x{} first={} {}",
                severity.as_str(),
                rule_id,
                group.count,
                first_path,
                title
            )
        })
        .collect()
}

#[derive(Debug)]
struct FindingGroup {
    count: usize,
    first_path: Option<PathBuf>,
}

fn display_child_edges(package_root: &Path, edges: &[RustModuleChildEdge]) -> String {
    let mut labels = edges
        .iter()
        .take(MAX_AGENT_SNAPSHOT_CHILD_EDGES)
        .map(|edge| {
            format!(
                "{}:{}",
                edge.kind.as_str(),
                display_project_path(package_root, &edge.child_path)
            )
        })
        .collect::<Vec<_>>();
    if edges.len() > MAX_AGENT_SNAPSHOT_CHILD_EDGES {
        labels.push(format!(
            "... +{} children",
            edges.len() - MAX_AGENT_SNAPSHOT_CHILD_EDGES
        ));
    }
    labels.join(", ")
}

fn compact_lines(lines: &[String], max_lines: usize, label: &str) -> Vec<String> {
    if lines.len() <= max_lines {
        return lines.to_vec();
    }
    let mut compacted = lines.iter().take(max_lines).cloned().collect::<Vec<_>>();
    compacted.push(format!(" - ... +{} {label}", lines.len() - max_lines));
    compacted
}

fn display_project_path(package_root: &Path, path: &Path) -> String {
    path.strip_prefix(package_root)
        .map_or_else(|_| display_path(path), display_path)
}

fn display_path(path: &Path) -> String {
    let rendered = path.display().to_string().replace('\\', "/");
    if rendered.is_empty() {
        ".".to_string()
    } else {
        rendered
    }
}

fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}
