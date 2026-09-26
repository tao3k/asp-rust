//! Agent policy rules derived from owner dependency graph facts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::parser::{
    ParsedRustModule, RustReasoningOwnerDependencyFacts, RustReasoningTreeFacts, file_location,
    path_line_location, source_line,
};
use crate::{AspRustFinding, AspRustRule};

use crate::rules::display_path;

use super::{
    RUST_AGENT_POLICY_DOCS_OWNER_FAN_OUT_V1, RUST_AGENT_POLICY_OWNER_DEPENDENCY_CYCLE_V1,
    RUST_AGENT_POLICY_OWNER_LEAF_IMPORT_V1,
};

const MIN_OWNER_FAN_OUT: usize = 3;

pub(super) fn dependency_graph_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module_by_path: &BTreeMap<PathBuf, &ParsedRustModule>,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut findings = Vec::new();
    findings.extend(owner_dependency_cycle_findings(
        reasoning_tree,
        module_by_path,
        rules,
    ));
    findings.extend(cross_owner_leaf_import_findings(
        reasoning_tree,
        module_by_path,
        rules,
    ));
    findings.extend(owner_fan_out_intent_findings(
        reasoning_tree,
        module_by_path,
        rules,
    ));
    findings
}

fn owner_dependency_cycle_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module_by_path: &BTreeMap<PathBuf, &ParsedRustModule>,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let dependencies = non_test_owner_dependencies(reasoning_tree);
    let proof_edges = dependencies
        .iter()
        .map(|dependency| OwnerDependencyProofEdge {
            source_namespace: dependency.source_namespace.clone(),
            target_namespace: dependency.target_namespace.clone(),
        })
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    for cycle_indices in owner_dependency_cycle_indices(&proof_edges) {
        let cycle = cycle_indices
            .iter()
            .map(|index| dependencies[*index])
            .collect::<Vec<_>>();
        let Some(dependency) = cycle.first().copied() else {
            continue;
        };
        let rule = &rules[RUST_AGENT_POLICY_OWNER_DEPENDENCY_CYCLE_V1];
        findings.push(AspRustFinding::from_rule(
            rule,
            format!(
                "Owner dependency cycle crosses {}.",
                display_dependency_cycle(reasoning_tree, &cycle)
            ),
            path_line_location(&dependency.source_path, dependency.line),
            dependency_source_line(module_by_path, dependency),
            "break the cycle by moving the shared contract behind one owner boundary",
        ));
    }
    findings
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnerDependencyProofEdge {
    pub(crate) source_namespace: Vec<String>,
    pub(crate) target_namespace: Vec<String>,
}

pub(crate) fn owner_dependency_cycle_indices(
    dependencies: &[OwnerDependencyProofEdge],
) -> Vec<Vec<usize>> {
    let outgoing = dependencies.iter().enumerate().fold(
        BTreeMap::<Vec<String>, Vec<usize>>::new(),
        |mut outgoing, (index, dependency)| {
            outgoing
                .entry(dependency.source_namespace.clone())
                .or_default()
                .push(index);
            outgoing
        },
    );
    let mut seen_cycles = BTreeSet::<String>::new();
    dependencies
        .iter()
        .enumerate()
        .filter_map(|(index, dependency)| {
            let mut tail = dependency_path_to_namespace(
                &dependency.target_namespace,
                &dependency.source_namespace,
                dependencies,
                &outgoing,
                &mut BTreeSet::new(),
            )?;
            let mut cycle = vec![index];
            cycle.append(&mut tail);
            let key = dependency_cycle_key(dependencies, &cycle);
            seen_cycles.insert(key).then_some(cycle)
        })
        .collect()
}

fn dependency_path_to_namespace(
    current_namespace: &[String],
    target_namespace: &[String],
    dependencies: &[OwnerDependencyProofEdge],
    outgoing: &BTreeMap<Vec<String>, Vec<usize>>,
    visited: &mut BTreeSet<Vec<String>>,
) -> Option<Vec<usize>> {
    if !visited.insert(current_namespace.to_vec()) {
        return None;
    }
    for dependency_index in outgoing.get(current_namespace)? {
        let dependency = &dependencies[*dependency_index];
        if dependency.target_namespace == target_namespace {
            return Some(vec![*dependency_index]);
        }
        if let Some(mut tail) = dependency_path_to_namespace(
            &dependency.target_namespace,
            target_namespace,
            dependencies,
            outgoing,
            visited,
        ) {
            let mut path = vec![*dependency_index];
            path.append(&mut tail);
            return Some(path);
        }
    }
    None
}

fn dependency_cycle_key(dependencies: &[OwnerDependencyProofEdge], cycle: &[usize]) -> String {
    let nodes = cycle
        .iter()
        .map(|index| dependencies[*index].source_namespace.join("/"))
        .collect::<Vec<_>>();
    (0..nodes.len())
        .map(|index| {
            nodes[index..]
                .iter()
                .chain(nodes[..index].iter())
                .cloned()
                .collect::<Vec<_>>()
                .join("->")
        })
        .min()
        .unwrap_or_default()
}

fn display_dependency_cycle(
    reasoning_tree: &RustReasoningTreeFacts,
    dependencies: &[&RustReasoningOwnerDependencyFacts],
) -> String {
    let mut paths = dependencies
        .iter()
        .map(|dependency| display_project_path(reasoning_tree, &dependency.source_path))
        .collect::<Vec<_>>();
    if let Some(first) = dependencies.first() {
        paths.push(display_project_path(reasoning_tree, &first.source_path));
    }
    paths.join(" -> ")
}

fn cross_owner_leaf_import_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module_by_path: &BTreeMap<PathBuf, &ParsedRustModule>,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let owner_branch_namespaces = owner_branch_namespaces(reasoning_tree);
    let owner_branch_paths = owner_branch_paths(reasoning_tree);
    let rule = &rules[RUST_AGENT_POLICY_OWNER_LEAF_IMPORT_V1];
    reasoning_tree
        .owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .filter(|dependency| owner_branch_paths.contains(&dependency.source_path))
        .filter(|dependency| !owner_branch_paths.contains(&dependency.target_path))
        .filter_map(|dependency| {
            let target_owner = nearest_owner_branch_namespace(
                &dependency.target_namespace,
                &owner_branch_namespaces,
            )?;
            if target_owner == dependency.source_namespace {
                return None;
            }
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} imports leaf {} across owner boundary.",
                    display_path(&dependency.source_path),
                    display_path(&dependency.target_path)
                ),
                path_line_location(&dependency.source_path, dependency.line),
                dependency_source_line(module_by_path, dependency),
                "depend on the target owner branch instead of reaching into its leaf module",
            ))
        })
        .collect()
}

fn owner_fan_out_intent_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module_by_path: &BTreeMap<PathBuf, &ParsedRustModule>,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let owner_branch_namespaces = owner_branch_namespaces(reasoning_tree);
    let mut dependencies_by_source = BTreeMap::<PathBuf, BTreeSet<Vec<String>>>::new();
    for dependency in reasoning_tree
        .owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
    {
        let Some(target_owner) =
            nearest_owner_branch_namespace(&dependency.target_namespace, &owner_branch_namespaces)
        else {
            continue;
        };
        if target_owner == dependency.source_namespace {
            continue;
        }
        dependencies_by_source
            .entry(dependency.source_path.clone())
            .or_default()
            .insert(target_owner);
    }

    let rule = &rules[RUST_AGENT_POLICY_DOCS_OWNER_FAN_OUT_V1];
    reasoning_tree
        .owner_branches
        .iter()
        .filter_map(|branch| {
            let fan_out = dependencies_by_source.get(&branch.path)?;
            if fan_out.len() < MIN_OWNER_FAN_OUT {
                return None;
            }
            let module = module_by_path.get(&branch.path)?;
            if module.syntax_facts.has_module_doc {
                return None;
            }
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} depends on {} owner branches without an intent doc.",
                    display_path(&branch.path),
                    fan_out.len()
                ),
                file_location(&branch.path),
                source_line(&module.source, 1),
                "add an owner fan-out intent doc using doc_markdown style",
            ))
        })
        .collect()
}

fn non_test_owner_dependencies(
    reasoning_tree: &RustReasoningTreeFacts,
) -> Vec<&RustReasoningOwnerDependencyFacts> {
    reasoning_tree
        .owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .collect()
}

fn owner_branch_paths(reasoning_tree: &RustReasoningTreeFacts) -> BTreeSet<PathBuf> {
    reasoning_tree
        .owner_branches
        .iter()
        .map(|branch| branch.path.clone())
        .collect()
}

fn owner_branch_namespaces(reasoning_tree: &RustReasoningTreeFacts) -> BTreeSet<Vec<String>> {
    reasoning_tree
        .owner_branches
        .iter()
        .map(|branch| branch.owner_namespace.clone())
        .collect()
}

fn nearest_owner_branch_namespace(
    namespace: &[String],
    owner_branch_namespaces: &BTreeSet<Vec<String>>,
) -> Option<Vec<String>> {
    (1..=namespace.len()).rev().find_map(|length| {
        let prefix = namespace.iter().take(length).cloned().collect::<Vec<_>>();
        owner_branch_namespaces.contains(&prefix).then_some(prefix)
    })
}

fn dependency_source_line(
    module_by_path: &BTreeMap<PathBuf, &ParsedRustModule>,
    dependency: &RustReasoningOwnerDependencyFacts,
) -> Option<String> {
    module_by_path
        .get(&dependency.source_path)
        .and_then(|module| source_line(&module.source, dependency.line))
}

fn display_project_path(reasoning_tree: &RustReasoningTreeFacts, path: &Path) -> String {
    path.strip_prefix(&reasoning_tree.package_root)
        .map_or_else(|_| display_path(path), display_path)
}
