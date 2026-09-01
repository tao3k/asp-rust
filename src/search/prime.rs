//! RFC `search prime` renderer for bounded package and workspace maps.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::discovery::{asp_rust_scope, discover_cargo_package_roots};
use crate::parser::{
    RustReasoningOwnerBranchFacts, RustReasoningOwnerBranchRole, RustReasoningTreeFacts,
    parse_cargo_dependency_facts, parse_cargo_manifest, rust_reasoning_tree_facts,
};
use crate::rules::evaluate_default_rule_packs_with_config;
use crate::{AspRustConfig, AspRustFinding};

use super::cargo::manifest_features;
use super::context::parse_scope;
use super::format::{
    display_project_path, package_label, render_cargo_dependency_line, render_owner_line,
    resolve_package_root, should_run_member_scopes,
};
use super::limits::{
    PRIME_EDGE_LIMIT, PRIME_FINDING_LIMIT, PRIME_OWNER_LIMIT, WORKSPACE_INDEX_PACKAGE_LIMIT,
    WORKSPACE_INDEX_THRESHOLD,
};
use super::prime_support::{
    api_candidate_lines, cfg_lines, child_edge_count, child_edge_lines, dependency_labels,
    feature_lines, grouped_finding_lines, owner_dependency_lines, surface_line, target_labels,
    target_lines, test_surface_line,
};
use super::recency::compare_paths_by_recency;

/// Render the RFC search-prime packet for a project root.
///
/// # Errors
///
/// Returns an error when the project root does not exist or when a selected
/// package cannot be resolved.
pub fn render_asp_rust_search_prime(project_root: &Path) -> Result<String, String> {
    render_asp_rust_search_prime_with_config(project_root, &AspRustConfig::default(), None)
}

/// Render the RFC search-prime packet with explicit harness config.
///
/// # Errors
///
/// Returns an error when the project root does not exist or when a selected
/// package cannot be resolved.
pub fn render_asp_rust_search_prime_with_config(
    project_root: &Path,
    config: &AspRustConfig,
    selected_package: Option<&str>,
) -> Result<String, String> {
    render_prime_with_config(
        project_root,
        config,
        selected_package,
        PrimeRenderMode::Full,
    )
}

pub(super) fn render_search_prime(
    project_root: &Path,
    config: &AspRustConfig,
    selected_package: Option<&str>,
    seed_limit: Option<usize>,
) -> Result<String, String> {
    render_prime_with_config(
        project_root,
        config,
        selected_package,
        prime_render_mode(seed_limit),
    )
}

fn render_prime_with_config(
    project_root: &Path,
    config: &AspRustConfig,
    selected_package: Option<&str>,
    mode: PrimeRenderMode,
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
    if let Some(selected_package) = selected_package {
        let package_root = resolve_package_root(project_root, &package_roots, selected_package)?;
        return Ok(render_package_prime(
            project_root,
            &package_root,
            config,
            mode,
        ));
    }
    if package_roots.len() > WORKSPACE_INDEX_THRESHOLD {
        return Ok(render_workspace_index_prime(project_root, &package_roots));
    }
    if package_roots.len() == 1 {
        return Ok(render_package_prime(
            project_root,
            &package_roots[0],
            config,
            mode,
        ));
    }

    let mut rendered = String::new();
    for package_root in package_roots {
        let package_prime = render_package_prime(project_root, &package_root, config, mode);
        if !rendered.is_empty() && !rendered.ends_with('\n') && !package_prime.is_empty() {
            rendered.push('\n');
        }
        rendered.push_str(&package_prime);
    }
    Ok(rendered)
}

#[derive(Clone, Copy)]
enum PrimeRenderMode {
    Full,
    SeedSource { seed_limit: usize },
}

fn prime_render_mode(seed_limit: Option<usize>) -> PrimeRenderMode {
    if let Some(seed_limit) = seed_limit {
        PrimeRenderMode::SeedSource { seed_limit }
    } else {
        PrimeRenderMode::Full
    }
}

fn render_workspace_index_prime(project_root: &Path, package_roots: &[PathBuf]) -> String {
    let mut rendered = format!(
        "[search-prime] mode=workspace-index workspace=large packages={}\n",
        package_roots.len()
    );
    for package_root in package_roots.iter().take(WORKSPACE_INDEX_PACKAGE_LIMIT) {
        let label = package_label(project_root, package_root);
        let _ = writeln!(
            rendered,
            "|package {label} next=package:{}",
            display_project_path(project_root, package_root)
        );
    }
    if package_roots.len() > WORKSPACE_INDEX_PACKAGE_LIMIT {
        let _ = writeln!(
            rendered,
            "|note truncated_packages={}",
            package_roots.len() - WORKSPACE_INDEX_PACKAGE_LIMIT
        );
    }
    rendered
}

fn render_package_prime(
    project_root: &Path,
    package_root: &Path,
    config: &AspRustConfig,
    mode: PrimeRenderMode,
) -> String {
    if let PrimeRenderMode::SeedSource { seed_limit } = mode {
        return render_package_prime_seed_source(project_root, package_root, config, seed_limit);
    }
    let scope = asp_rust_scope(
        package_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let parsed_modules = parse_scope(&scope, config);
    let findings = match mode {
        PrimeRenderMode::Full => {
            evaluate_default_rule_packs_with_config(Some(&scope), &parsed_modules, config)
        }
        PrimeRenderMode::SeedSource { .. } => Vec::new(),
    };
    let reasoning_tree = rust_reasoning_tree_facts(&scope, &parsed_modules);
    let source_modules = reasoning_tree
        .modules
        .iter()
        .filter(|module| module.is_source_module)
        .count();
    let owner_dependencies = reasoning_tree
        .owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .collect::<Vec<_>>();
    let owner_branches = ranked_owner_branches(package_root, &reasoning_tree, &findings);
    let cargo_dependencies = parse_cargo_dependency_facts(package_root);
    let cargo_manifest = parse_cargo_manifest(package_root);
    let features = manifest_features(package_root);
    let package_label = package_label(project_root, package_root);
    let module_by_path = parsed_modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let mut rendered = format!(
        "[search-prime] mode=package package={package_label} src={source_modules} own={} edge={} find={} dep={}\n",
        reasoning_tree.owner_branches.len(),
        owner_dependencies.len() + child_edge_count(&reasoning_tree),
        findings.len(),
        cargo_dependencies.len()
    );
    let _ = writeln!(
        rendered,
        "|package {package_label} t={} dep={}",
        target_labels(&scope),
        dependency_labels(&cargo_dependencies)
    );
    append_decision_primer_lines(&mut rendered, "rust");
    for dependency in cargo_dependencies.iter().take(6) {
        let _ = writeln!(rendered, "{}", render_cargo_dependency_line(dependency));
    }
    for line in feature_lines(&features) {
        let _ = writeln!(rendered, "{line}");
    }
    for line in cfg_lines(package_root, &parsed_modules) {
        let _ = writeln!(rendered, "{line}");
    }
    for line in target_lines(package_root, &cargo_manifest) {
        let _ = writeln!(rendered, "{line}");
    }
    if let Some(line) = surface_line(&parsed_modules) {
        let _ = writeln!(rendered, "{line}");
    }
    if let Some(line) = test_surface_line(&scope) {
        let _ = writeln!(rendered, "{line}");
    }
    for line in api_candidate_lines(package_root, &parsed_modules) {
        let _ = writeln!(rendered, "{line}");
    }
    for branch in owner_branches.iter().take(PRIME_OWNER_LIMIT) {
        let _ = writeln!(
            rendered,
            "{}",
            render_owner_line(
                package_root,
                branch,
                module_by_path.get(&branch.path).copied()
            )
        );
    }
    for edge in child_edge_lines(package_root, &reasoning_tree)
        .into_iter()
        .chain(owner_dependency_lines(package_root, &owner_dependencies))
        .take(PRIME_EDGE_LIMIT)
    {
        let _ = writeln!(rendered, "{edge}");
    }
    for finding_line in grouped_finding_lines(package_root, &findings)
        .into_iter()
        .take(PRIME_FINDING_LIMIT)
    {
        let _ = writeln!(rendered, "{finding_line}");
    }
    let selected_owner_paths = owner_branches
        .iter()
        .take(PRIME_OWNER_LIMIT)
        .map(|branch| branch.path.clone())
        .collect::<Vec<_>>();
    append_prime_graph_synthesis_line(
        &mut rendered,
        package_root,
        &selected_owner_paths,
        &reasoning_tree,
        &findings,
    );
    let next = owner_branches
        .iter()
        .take(3)
        .map(|branch| format!("owner:{}", display_project_path(package_root, &branch.path)))
        .collect::<Vec<_>>();
    if !next.is_empty() {
        let _ = writeln!(rendered, "|next {}", next.join(","));
    }
    rendered
}

fn render_package_prime_seed_source(
    project_root: &Path,
    package_root: &Path,
    config: &AspRustConfig,
    seed_limit: usize,
) -> String {
    let owner_paths = fast_prime_owner_seed_paths(package_root, seed_limit.max(1));
    let package_label = package_label(project_root, package_root);
    let mut rendered = String::new();
    rendered.push_str("[search-prime] ");
    let _ = write!(rendered, "mode=package package={package_label} ");
    let source_modules = owner_paths.len();
    let _ = write!(rendered, "src={source_modules} ");
    let _ = write!(rendered, "own={} ", owner_paths.len());
    let _ = write!(rendered, "edge=0 ");
    let _ = write!(rendered, "dep=0");
    rendered.push('\n');
    append_decision_primer_lines(&mut rendered, "rust");
    let feature_names = fast_manifest_feature_seed_names(package_root, seed_limit.max(1));
    if !feature_names.is_empty() {
        let _ = writeln!(rendered, "|seed features:{}", feature_names.join(","));
    }
    let cfg_names = feature_names
        .iter()
        .map(|name| format!("feature:{name}"))
        .collect::<Vec<_>>();
    if !cfg_names.is_empty() {
        let _ = writeln!(rendered, "|seed cfg:{}", cfg_names.join(","));
    }
    let owner_limit = seed_limit.min(owner_paths.len());
    let owners = owner_paths
        .iter()
        .take(owner_limit)
        .map(|path| display_project_path(project_root, path))
        .collect::<Vec<_>>();
    if !owners.is_empty() {
        let _ = writeln!(rendered, "|seed owner:{}", owners.join(","));
    }
    if fast_prime_has_test_surface(package_root, config) {
        let _ = writeln!(rendered, "|seed tests");
    }
    let selected_owner_paths = owner_paths
        .iter()
        .take(owner_limit)
        .cloned()
        .collect::<Vec<_>>();
    append_fast_prime_graph_synthesis_line(&mut rendered, package_root, &selected_owner_paths);
    if owner_paths.len() > owner_limit {
        let _ = writeln!(
            rendered,
            "|note seeds_truncated={} limit={}",
            owner_paths.len() - owner_limit,
            seed_limit
        );
    }
    rendered
}

fn fast_prime_owner_seed_paths(package_root: &Path, seed_limit: usize) -> Vec<PathBuf> {
    let mut paths = ["src/lib.rs", "src/main.rs"]
        .into_iter()
        .map(|relative| package_root.join(relative))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    if paths.len() < seed_limit {
        let src_dir = package_root.join("src");
        if let Ok(entries) = std::fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                    continue;
                }
                if !paths.contains(&path) {
                    paths.push(path);
                }
                if paths.len() >= seed_limit {
                    break;
                }
            }
        }
    }
    paths.sort_by(|left, right| compare_paths_by_recency(package_root, left, right));
    paths.truncate(seed_limit);
    paths
}

fn fast_prime_has_test_surface(package_root: &Path, config: &AspRustConfig) -> bool {
    config
        .test_dir_names
        .iter()
        .map(|dir| package_root.join(dir))
        .any(|dir| dir.is_dir())
}

fn fast_manifest_feature_seed_names(package_root: &Path, limit: usize) -> Vec<String> {
    let manifest_path = package_root.join("Cargo.toml");
    let Ok(manifest) = std::fs::read_to_string(manifest_path) else {
        return Vec::new();
    };
    let mut in_features = false;
    manifest
        .lines()
        .filter_map(|raw_line| {
            let line = raw_line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                in_features = line == "[features]";
                return None;
            }
            if !in_features || line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (raw_name, _) = line.split_once('=')?;
            let name = raw_name.trim().trim_matches('"');
            if name.is_empty() || matches!(name, "default" | "full") {
                None
            } else {
                Some(name.to_string())
            }
        })
        .take(limit)
        .collect()
}

fn append_decision_primer_lines(rendered: &mut String, _language_id: &str) {
    let _ = writeln!(
        rendered,
        "|decision purpose=decision-primer answer=false code=false route=evidence-state capabilities=pipe,lexical,fd-query,rg-query,owner-items,selector-code history=asp-artifacts:directReadRisk,repeatedPrime,repeatedPipe,bestPath risk=broad-direct-read,manual-window-scan,repeat-prime rule=\"prime maps workspace/owners only; choose the narrowest route justified by current evidence\" routeOptions=\"owner-items when owner known; selector-code when exact selector known; deps when dependency known; pipe/lexical only for ambiguous query refinement\""
    );
}

fn append_prime_graph_synthesis_line(
    rendered: &mut String,
    package_root: &Path,
    selected_owner_paths: &[PathBuf],
    reasoning_tree: &RustReasoningTreeFacts,
    findings: &[AspRustFinding],
) {
    if selected_owner_paths.is_empty() {
        return;
    }
    let high_impact_owners = selected_owner_paths
        .iter()
        .take(4)
        .map(|path| display_project_path(package_root, path))
        .collect::<Vec<_>>();
    let frontier_owners =
        ranked_frontier_owner_paths(package_root, reasoning_tree, selected_owner_paths)
            .into_iter()
            .take(4)
            .collect::<Vec<_>>();
    let finding_owners = finding_owner_paths(package_root, findings);
    let seeds = frontier_owners
        .iter()
        .map(|path| format!("owner:{path}"))
        .collect::<Vec<_>>();
    let mut parts = vec![
        "algorithm=owner-rank-frontier".to_string(),
        "scope=prime".to_string(),
        "summary=owner-graph-frontier".to_string(),
        format!("selected_owners={}", selected_owner_paths.len()),
        format!("selected_edges={}", graph_edge_count(reasoning_tree)),
        format!("high_impact_owners={}", high_impact_owners.join(",")),
    ];
    if !frontier_owners.is_empty() {
        parts.push(format!("frontier_owners={}", frontier_owners.join(",")));
    }
    if !finding_owners.is_empty() {
        parts.push(format!("finding_owners={}", finding_owners.join(",")));
    }
    if !seeds.is_empty() {
        parts.push(format!("seeds={}", seeds.join(",")));
    }
    let _ = writeln!(rendered, "|synthesis {}", parts.join(" "));
}

fn append_fast_prime_graph_synthesis_line(
    rendered: &mut String,
    package_root: &Path,
    selected_owner_paths: &[PathBuf],
) {
    if selected_owner_paths.is_empty() {
        return;
    }
    let high_impact_owners = selected_owner_paths
        .iter()
        .take(4)
        .map(|path| display_project_path(package_root, path))
        .collect::<Vec<_>>();
    let seeds = high_impact_owners
        .iter()
        .map(|path| format!("owner:{path}"))
        .collect::<Vec<_>>();
    let parts = [
        "algorithm=fast-owner-file-frontier".to_string(),
        "scope=prime".to_string(),
        "summary=owner-file-frontier".to_string(),
        format!("selected_owners={}", selected_owner_paths.len()),
        "selected_edges=0".to_string(),
        format!("high_impact_owners={}", high_impact_owners.join(",")),
        format!("frontier_owners={}", high_impact_owners.join(",")),
        format!("seeds={}", seeds.join(",")),
    ];
    let _ = writeln!(rendered, "|synthesis {}", parts.join(" "));
}

fn ranked_frontier_owner_paths(
    package_root: &Path,
    reasoning_tree: &RustReasoningTreeFacts,
    selected_owner_paths: &[PathBuf],
) -> Vec<String> {
    let selected = selected_owner_paths.iter().collect::<BTreeSet<_>>();
    let mut counts = BTreeMap::<PathBuf, usize>::new();
    for branch in &reasoning_tree.owner_branches {
        for edge in &branch.declared_child_edges {
            push_frontier_candidate(&selected, &mut counts, &branch.path, &edge.child_path);
        }
    }
    for dependency in reasoning_tree
        .owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
    {
        push_frontier_candidate(
            &selected,
            &mut counts,
            &dependency.source_path,
            &dependency.target_path,
        );
    }
    let mut frontier = counts.into_iter().collect::<Vec<_>>();
    frontier.sort_by(|(left_path, left_count), (right_path, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| compare_paths_by_recency(package_root, left_path, right_path))
    });
    frontier
        .into_iter()
        .map(|(path, _)| display_project_path(package_root, &path))
        .collect()
}

fn push_frontier_candidate(
    selected: &BTreeSet<&PathBuf>,
    counts: &mut BTreeMap<PathBuf, usize>,
    source_path: &PathBuf,
    target_path: &PathBuf,
) {
    if selected.contains(source_path) && !selected.contains(target_path) {
        *counts.entry(target_path.clone()).or_default() += 1;
    }
    if selected.contains(target_path) && !selected.contains(source_path) {
        *counts.entry(source_path.clone()).or_default() += 1;
    }
}

fn finding_owner_paths(package_root: &Path, findings: &[AspRustFinding]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    findings
        .iter()
        .filter_map(|finding| finding.location.path.as_ref())
        .filter_map(|path| {
            let display = display_project_path(package_root, path);
            seen.insert(display.clone()).then_some(display)
        })
        .take(4)
        .collect()
}

fn graph_edge_count(reasoning_tree: &RustReasoningTreeFacts) -> usize {
    child_edge_count(reasoning_tree)
        + reasoning_tree
            .owner_dependencies
            .iter()
            .filter(|dependency| !dependency.is_test_context)
            .count()
}

fn ranked_owner_branches<'a>(
    package_root: &Path,
    reasoning_tree: &'a RustReasoningTreeFacts,
    findings: &[AspRustFinding],
) -> Vec<&'a RustReasoningOwnerBranchFacts> {
    let finding_paths = findings
        .iter()
        .filter_map(|finding| finding.location.path.as_ref())
        .collect::<Vec<_>>();
    let mut branches = reasoning_tree.owner_branches.iter().collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        owner_rank(right, &finding_paths)
            .cmp(&owner_rank(left, &finding_paths))
            .then_with(|| compare_paths_by_recency(package_root, &left.path, &right.path))
    });
    branches
}

fn owner_rank(branch: &RustReasoningOwnerBranchFacts, finding_paths: &[&PathBuf]) -> usize {
    let mut rank = 0;
    if branch.roles.contains(&RustReasoningOwnerBranchRole::Root) {
        rank += 100;
    }
    if branch.roles.contains(&RustReasoningOwnerBranchRole::Facade) {
        rank += 80;
    }
    if branch
        .roles
        .contains(&RustReasoningOwnerBranchRole::Interface)
    {
        rank += 70;
    }
    if finding_paths.contains(&&branch.path) {
        rank += 60;
    }
    if branch.import_summary.external_imports > 0 {
        rank += 30;
    }
    if !branch.declared_child_edges.is_empty() {
        rank += 20;
    }
    rank
}
