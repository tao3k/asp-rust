//! Agent policy rules derived from source surface facts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::parser::{
    ParsedRustModule, RustReasoningTreeFacts, RustTopLevelItemSyntax, RustUseImportRootKind,
    file_location, path_line_location, source_line,
};
use crate::{AspRustFinding, AspRustRule};

use crate::rules::display_path;

use super::doc_boundary::{documented_agent_boundary, module_doc_contains};
use super::{
    RUST_AGENT_POLICY_API_ERROR_BOUNDARY_V1, RUST_AGENT_POLICY_API_FACADE_EXPORT_GROUPS_V1,
    RUST_AGENT_POLICY_API_FLAG_PARAMETER_SURFACE_V1,
    RUST_AGENT_POLICY_API_POSITIONAL_PARAMETER_SURFACE_V1,
    RUST_AGENT_POLICY_API_PUBLIC_NAME_CONFLICT_V1,
    RUST_AGENT_POLICY_API_SEMANTIC_IDENTIFIER_TYPE_V1, RUST_AGENT_POLICY_DOCS_BRANCH_INTENT_V1,
    RUST_AGENT_POLICY_DOCS_MODULE_INTENT_V1, RUST_AGENT_POLICY_DOCS_PUBLIC_ITEM_V1,
    RUST_AGENT_POLICY_SOURCE_MODULE_PATH_NAME_V1, RUST_AGENT_POLICY_SOURCE_NAMESPACE_REPEAT_V1,
    RUST_AGENT_POLICY_SOURCE_PUBLIC_MODULE_NAME_V1, RUST_AGENT_POLICY_TEST_SUPPORT_REEXPORT_V1,
};

const MAX_FACADE_REEXPORTS: usize = 28;
const MIN_BRANCH_CHILD_MODULES: usize = 2;
const MIN_BROAD_POSITIONAL_PARAMS: usize = 5;
const GENERIC_MODULE_NAMES: &[&str] = &[
    "common", "helper", "helpers", "misc", "shared", "stuff", "util", "utils",
];

pub(super) fn source_module_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut findings = Vec::new();
    findings.extend(module_intent_findings(module, rules));
    findings.extend(public_doc_findings(module, rules));
    findings.extend(facade_reexport_findings(reasoning_tree, module, rules));
    findings.extend(generic_public_module_findings(module, rules));
    findings.extend(branch_module_intent_findings(reasoning_tree, module, rules));
    findings.extend(public_primitive_identifier_findings(module, rules));
    findings.extend(public_flag_parameter_findings(module, rules));
    findings.extend(public_broad_parameter_surface_findings(module, rules));
    findings.extend(public_application_error_boundary_findings(module, rules));
    findings
}

pub(super) fn repeated_namespace_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_SOURCE_NAMESPACE_REPEAT_V1];
    let mut branches = BTreeMap::<PathBuf, (PathBuf, BTreeSet<String>)>::new();
    for module in modules {
        let Some(module_facts) = reasoning_tree.module(&module.report.path) else {
            continue;
        };
        let Some(branch) = module_facts.source_path.repeated_namespace_branch.clone() else {
            continue;
        };
        if module_facts
            .source_path
            .repeated_namespace_segments
            .is_empty()
        {
            continue;
        }
        let (_, branch_repeated) = branches
            .entry(branch)
            .or_insert_with(|| (module.report.path.clone(), BTreeSet::new()));
        branch_repeated.extend(module_facts.source_path.repeated_namespace_segments.clone());
    }
    branches
        .into_iter()
        .map(|(branch, (path, repeated))| {
            let repeated = repeated.into_iter().collect::<Vec<_>>().join(", ");
            AspRustFinding::from_rule(
                rule,
                format!(
                    "{} repeats namespace segment(s): {repeated}.",
                    display_path(&branch)
                ),
                file_location(path),
                None,
                "rename the deepest repeated path segment to its real responsibility",
            )
        })
        .collect()
}

pub(super) fn generic_module_path_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_SOURCE_MODULE_PATH_NAME_V1];
    modules
        .iter()
        .filter_map(|module| {
            let module_facts = reasoning_tree.module(&module.report.path)?;
            let generic_segment = module_facts
                .source_path
                .namespace_components
                .iter()
                .find(|component| is_generic_module_name(component.as_str()))?;
            if module_doc_contains(
                &module.source,
                &["generic path boundary", "compatibility path boundary"],
            ) {
                return None;
            }
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} uses generic module path segment `{generic_segment}`.",
                    display_path(&module.report.path)
                ),
                file_location(&module.report.path),
                source_line(&module.source, 1),
                "rename this module path segment to the responsibility it owns",
            ))
        })
        .collect()
}

pub(super) fn public_name_conflict_findings(
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut names = BTreeMap::<String, Vec<(&ParsedRustModule, usize)>>::new();
    for module in modules {
        if !module.report.is_valid {
            continue;
        };
        for item in &module.syntax_facts.top_level_items {
            let Some(name) = public_named_item(item) else {
                continue;
            };
            if documented_agent_boundary(
                &module.source,
                item.line,
                &["namespace boundary", "compatibility namespace boundary"],
            ) {
                continue;
            }
            names
                .entry(name.to_owned())
                .or_default()
                .push((module, item.line));
        }
    }
    let rule = &rules[RUST_AGENT_POLICY_API_PUBLIC_NAME_CONFLICT_V1];
    let mut findings = Vec::new();
    for (name, locations) in names {
        if locations.len() < 2 {
            continue;
        }
        for (module, line) in locations {
            findings.push(AspRustFinding::from_rule(
                rule,
                format!(
                    "Public item `{name}` appears in multiple Rust modules and may be ambiguous for agents."
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "rename or namespace this public item so the owner is unambiguous",
            ));
        }
    }
    findings
}

pub(super) fn test_support_reexport_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let support_modules = test_support_modules(reasoning_tree);
    let consumed_support_names =
        consumed_test_support_names(reasoning_tree, modules, &support_modules);
    modules
        .iter()
        .filter(|module| module.report.is_valid)
        .filter(|module| {
            reasoning_tree
                .module(&module.report.path)
                .is_some_and(|module_facts| module_facts.source_path.is_test_support_module)
        })
        .flat_map(|module| {
            test_support_module_reexport_findings(
                module,
                consumed_support_names.get(&module.report.path),
                rules,
            )
        })
        .collect()
}

fn module_intent_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    if module.syntax_facts.has_module_doc || !has_public_surface(module) {
        return Vec::new();
    }
    let rule = &rules[RUST_AGENT_POLICY_DOCS_MODULE_INTENT_V1];
    vec![AspRustFinding::from_rule(
        rule,
        format!(
            "{} has public Rust surface without a module intent doc.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        source_line(&module.source, 1),
        "add a module intent doc using doc_markdown style",
    )]
}

fn public_doc_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_DOCS_PUBLIC_ITEM_V1];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| {
            let public_name = public_named_item(item)?;
            if item.has_doc {
                return None;
            }
            let line = item.line;
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public item `{public_name}` without a doc comment.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "add a doc comment using clippy::doc_markdown style",
            ))
        })
        .collect()
}

fn facade_reexport_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    if !reasoning_tree
        .module(&module.report.path)
        .is_some_and(|module_facts| module_facts.source_path.is_crate_facade)
    {
        return Vec::new();
    }
    let reexport_count = module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.is_public_use)
        .count();
    if reexport_count <= MAX_FACADE_REEXPORTS {
        return Vec::new();
    }
    let rule = &rules[RUST_AGENT_POLICY_API_FACADE_EXPORT_GROUPS_V1];
    vec![AspRustFinding::from_rule(
        rule,
        format!(
            "{} re-exports {reexport_count} public use groups from the crate facade.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        source_line(&module.source, 1),
        "group facade exports behind smaller owner modules",
    )]
}

fn generic_public_module_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_SOURCE_PUBLIC_MODULE_NAME_V1];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| {
            let module_decl = item.module.as_ref()?;
            if !item.is_public {
                return None;
            }
            let module_name = &module_decl.ident;
            if !is_generic_module_name(module_name.as_str()) {
                return None;
            }
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes generic public module `{module_name}`.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, item.line),
                source_line(&module.source, item.line),
                "rename this public module to the domain it owns",
            ))
        })
        .collect()
}

fn branch_module_intent_findings(
    reasoning_tree: &RustReasoningTreeFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    if module.syntax_facts.has_module_doc {
        return Vec::new();
    }
    let child_modules = reasoning_tree
        .module(&module.report.path)
        .map_or(0, |module_facts| module_facts.declared_child_edges.len());
    if child_modules < MIN_BRANCH_CHILD_MODULES {
        return Vec::new();
    }
    let rule = &rules[RUST_AGENT_POLICY_DOCS_BRANCH_INTENT_V1];
    vec![AspRustFinding::from_rule(
        rule,
        format!(
            "{} owns {child_modules} resolved child edges without a reasoning-tree intent doc.",
            display_path(&module.report.path)
        ),
        file_location(&module.report.path),
        source_line(&module.source, 1),
        "add a branch intent doc using doc_markdown style",
    )]
}

fn public_primitive_identifier_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_API_SEMANTIC_IDENTIFIER_TYPE_V1];
    module
        .syntax_facts
        .public_function_params
        .iter()
        .filter_map(|param| {
            if param.is_test_context || !is_semantic_identifier_param(&param.param_name) {
                return None;
            }
            let primitive_type = param.primitive_contract_type.as_ref()?;
            if documented_agent_boundary(
                &module.source,
                param.function_line,
                &["primitive boundary", "identifier boundary", "raw dto boundary"],
            ) {
                return None;
            }
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public function `{}` parameter `{}` as primitive `{primitive_type}`.",
                    display_path(&module.report.path),
                    param.function_name,
                    param.param_name
                ),
                path_line_location(&module.report.path, param.line),
                source_line(&module.source, param.line),
                "wrap this identifier in an owner-named newtype or document why the primitive boundary is intentional",
            ))
        })
        .collect()
}

fn public_flag_parameter_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut params_by_function = BTreeMap::<String, Vec<usize>>::new();
    for (index, param) in module
        .syntax_facts
        .public_function_params
        .iter()
        .enumerate()
    {
        if param.is_test_context || param.flag_contract_type.is_none() {
            continue;
        }
        params_by_function
            .entry(param.function_name.clone())
            .or_default()
            .push(index);
    }

    let rule = &rules[RUST_AGENT_POLICY_API_FLAG_PARAMETER_SURFACE_V1];
    params_by_function
        .into_iter()
        .filter_map(|(function_name, param_indices)| {
            const MIN_FLAG_PARAMS: usize = 2;
            if param_indices.len() < MIN_FLAG_PARAMS {
                return None;
            }
            let params = param_indices
                .into_iter()
                .map(|index| &module.syntax_facts.public_function_params[index])
                .collect::<Vec<_>>();
            let function_line = params.first()?.function_line;
            if documented_agent_boundary(
                &module.source,
                function_line,
                &["flag mode boundary", "typed flag boundary", "positional boundary"],
            ) {
                return None;
            }
            let first_param = params.iter().min_by_key(|param| param.line)?;
            let flag_list = params
                .iter()
                .map(|param| {
                    let flag_type = param.flag_contract_type.as_deref().unwrap_or("bool");
                    format!("{}: {flag_type}", param.param_name)
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public function `{function_name}` with multiple flag parameters: {flag_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, first_param.line),
                source_line(&module.source, first_param.line),
                "replace these public flags with a typed mode or config surface",
            ))
        })
        .collect()
}

fn public_broad_parameter_surface_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut params_by_function = BTreeMap::<(usize, String), Vec<usize>>::new();
    for (index, param) in module
        .syntax_facts
        .public_function_params
        .iter()
        .enumerate()
    {
        if param.is_test_context {
            continue;
        }
        params_by_function
            .entry((param.function_line, param.function_name.clone()))
            .or_default()
            .push(index);
    }

    let rule = &rules[RUST_AGENT_POLICY_API_POSITIONAL_PARAMETER_SURFACE_V1];
    params_by_function
        .into_iter()
        .filter_map(|((function_line, function_name), param_indices)| {
            if param_indices.len() < MIN_BROAD_POSITIONAL_PARAMS {
                return None;
            }
            if documented_agent_boundary(
                &module.source,
                function_line,
                &["positional boundary", "builder compatibility boundary"],
            ) {
                return None;
            }
            let params = param_indices
                .into_iter()
                .map(|index| &module.syntax_facts.public_function_params[index])
                .collect::<Vec<_>>();
            let param_names = params
                .iter()
                .map(|param| param.param_name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public function `{function_name}` with {} positional parameters: {param_names}.",
                    display_path(&module.report.path),
                    params.len()
                ),
                path_line_location(&module.report.path, function_line),
                source_line(&module.source, function_line),
                "replace this positional surface with a named config, request type, or builder",
            ))
        })
        .collect()
}

fn public_application_error_boundary_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_API_ERROR_BOUNDARY_V1];
    module
        .syntax_facts
        .public_function_returns
        .iter()
        .filter_map(|function_return| {
            if function_return.is_test_context {
                return None;
            }
            let boundary = function_return.application_error_boundary.as_ref()?;
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public function `{}` with application error boundary `{boundary}`.",
                    display_path(&module.report.path),
                    function_return.function_name
                ),
                path_line_location(&module.report.path, function_return.line),
                source_line(&module.source, function_return.line),
                "return a crate-owned typed error at the public boundary or document why this is an application boundary",
            ))
        })
        .collect()
}

fn test_support_module_reexport_findings(
    module: &ParsedRustModule,
    consumed_support_names: Option<&BTreeSet<String>>,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let local_references = local_path_reference_lines_by_name(module);
    let mut unused_names_by_line = BTreeMap::<usize, Vec<String>>::new();
    for use_statement in &module.syntax_facts.use_statements {
        for reexport in &use_statement.reexports {
            if !reexport.visibility.is_reexport()
                || locally_referenced_after_reexport(
                    &local_references,
                    &reexport.exposed_name,
                    reexport.line,
                )
                || consumed_support_names
                    .is_some_and(|names| names.contains(&reexport.exposed_name))
            {
                continue;
            }
            unused_names_by_line
                .entry(reexport.line)
                .or_default()
                .push(reexport.exposed_name.clone());
        }
    }
    let rule = &rules[RUST_AGENT_POLICY_TEST_SUPPORT_REEXPORT_V1];
    unused_names_by_line
        .into_iter()
        .map(|(line, mut names)| {
            names.sort();
            names.dedup();
            let names = names
                .into_iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            AspRustFinding::from_rule(
                rule,
                format!(
                    "{} re-exports unused test support scope name(s): {names}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "remove unused support re-export names or import them directly at the call site",
            )
        })
        .collect()
}

fn test_support_modules(reasoning_tree: &RustReasoningTreeFacts) -> BTreeMap<Vec<String>, PathBuf> {
    reasoning_tree
        .modules
        .iter()
        .filter(|module| module.source_path.is_test_support_module)
        .filter(|module| !module.source_path.namespace_components.is_empty())
        .map(|module| {
            (
                module.source_path.namespace_components.clone(),
                module.path.clone(),
            )
        })
        .collect()
}

fn consumed_test_support_names(
    reasoning_tree: &RustReasoningTreeFacts,
    modules: &[ParsedRustModule],
    support_modules: &BTreeMap<Vec<String>, PathBuf>,
) -> BTreeMap<PathBuf, BTreeSet<String>> {
    let mut names = BTreeMap::<PathBuf, BTreeSet<String>>::new();
    for (support_path, name) in modules.iter().flat_map(|module| {
        consumed_test_support_references(reasoning_tree, module, support_modules)
    }) {
        names.entry(support_path).or_default().insert(name);
    }
    names
}

fn consumed_test_support_references(
    reasoning_tree: &RustReasoningTreeFacts,
    module: &ParsedRustModule,
    support_modules: &BTreeMap<Vec<String>, PathBuf>,
) -> Vec<(PathBuf, String)> {
    let Some(module_facts) = reasoning_tree.module(&module.report.path) else {
        return Vec::new();
    };
    let current_namespace = &module_facts.source_path.namespace_components;
    let import_references = module
        .syntax_facts
        .use_statements
        .iter()
        .flat_map(|use_statement| &use_statement.imports)
        .filter_map(|import| {
            test_support_reference(
                current_namespace,
                &import.segments,
                import.root_kind,
                import.parent_hops,
                support_modules,
            )
        });
    let path_references = module
        .syntax_facts
        .path_references
        .iter()
        .filter_map(|reference| {
            test_support_reference(
                current_namespace,
                &reference.segments,
                import_root_kind(&reference.segments),
                parent_hops(&reference.segments),
                support_modules,
            )
        });
    import_references.chain(path_references).collect()
}

fn test_support_reference(
    current_namespace: &[String],
    segments: &[String],
    root_kind: RustUseImportRootKind,
    parent_hops: usize,
    support_modules: &BTreeMap<Vec<String>, PathBuf>,
) -> Option<(PathBuf, String)> {
    let candidate =
        local_reference_candidate_namespace(current_namespace, segments, root_kind, parent_hops)?;
    let (support_namespace, support_path) =
        longest_test_support_namespace_prefix(&candidate, support_modules)?;
    let exposed_name = candidate.get(support_namespace.len())?.clone();
    Some((support_path.clone(), exposed_name))
}

fn local_reference_candidate_namespace(
    current_namespace: &[String],
    segments: &[String],
    root_kind: RustUseImportRootKind,
    parent_hops: usize,
) -> Option<Vec<String>> {
    match root_kind {
        RustUseImportRootKind::Crate => {
            let root = current_namespace.first()?.clone();
            let mut namespace = vec![root];
            namespace.extend(segments.iter().skip(1).cloned());
            Some(namespace)
        }
        RustUseImportRootKind::SelfScope => {
            let mut namespace = current_namespace.to_vec();
            namespace.extend(segments.iter().skip(1).cloned());
            Some(namespace)
        }
        RustUseImportRootKind::Parent => {
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
        RustUseImportRootKind::Absolute
        | RustUseImportRootKind::External
        | RustUseImportRootKind::Unknown => None,
    }
}

fn longest_test_support_namespace_prefix<'a>(
    candidate: &[String],
    support_modules: &'a BTreeMap<Vec<String>, PathBuf>,
) -> Option<(&'a Vec<String>, &'a PathBuf)> {
    (1..=candidate.len()).rev().find_map(|length| {
        let prefix = candidate.iter().take(length).cloned().collect::<Vec<_>>();
        support_modules.get_key_value(&prefix)
    })
}

fn import_root_kind(segments: &[String]) -> RustUseImportRootKind {
    let Some(first_segment) = segments.first() else {
        return RustUseImportRootKind::Unknown;
    };
    match first_segment.as_str() {
        "crate" => RustUseImportRootKind::Crate,
        "self" => RustUseImportRootKind::SelfScope,
        "super" => RustUseImportRootKind::Parent,
        _ => RustUseImportRootKind::External,
    }
}

fn parent_hops(segments: &[String]) -> usize {
    segments
        .iter()
        .take_while(|segment| segment.as_str() == "super")
        .count()
}

fn local_path_reference_lines_by_name(
    module: &ParsedRustModule,
) -> BTreeMap<String, BTreeSet<usize>> {
    let mut references = BTreeMap::<String, BTreeSet<usize>>::new();
    for reference in &module.syntax_facts.path_references {
        references
            .entry(reference.terminal_name.clone())
            .or_default()
            .insert(reference.line);
    }
    references
}

fn locally_referenced_after_reexport(
    references: &BTreeMap<String, BTreeSet<usize>>,
    exposed_name: &str,
    reexport_line: usize,
) -> bool {
    references
        .get(exposed_name)
        .is_some_and(|lines| lines.iter().any(|line| *line != reexport_line))
}

fn has_public_surface(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .any(|item| public_named_item(item).is_some())
}

fn public_named_item(item: &RustTopLevelItemSyntax) -> Option<&str> {
    if item.kind == "mod" {
        return None;
    }
    item.is_public.then_some(item.name.as_deref()).flatten()
}

fn is_generic_module_name(name: &str) -> bool {
    GENERIC_MODULE_NAMES.contains(&name)
}

fn is_semantic_identifier_param(name: &str) -> bool {
    name == "id" || name.ends_with("_id")
}
