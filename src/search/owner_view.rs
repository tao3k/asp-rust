use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::AspRustConfig;
use crate::parser::{ParsedRustModule, RustReasoningOwnerBranchFacts, parse_rust_file};

use super::RustSearchOptions;
use super::context::{
    PackageSearchContext, exact_owner_path_matches, exact_rust_file_query, search_contexts,
    search_contexts_for_path_queries, search_contexts_for_path_query,
};
use super::format::{
    display_project_path, owner_role_for_path, package_label, package_roots_for_request,
    query_set_terms, render_owner_line,
};
use super::item_query::{
    owner_item_count, render_item_query_line, render_owner_item_frontier_lines,
    render_owner_item_hot_lines, render_owner_item_lines,
};
use super::limits::SEARCH_OWNER_LIMIT;
use super::owner as owner_search;
use super::owner_seed_view::render_exact_path_owner_seed_view;
use super::scope::{owner_branch_matches, owner_path_matches};

pub(super) fn render_search_owner(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let query_terms = query_set_terms(query);
    if query_terms.len() > 1 {
        return render_search_owner_query_set(project_root, config, query, &query_terms, options);
    }
    if let Some(rendered) = render_exact_path_owner(project_root, config, query, options)? {
        return Ok(rendered);
    }
    let contexts = search_contexts_for_path_query(project_root, config, options, query)?;
    Ok(contexts
        .iter()
        .map(|context| render_search_owner_block(project_root, context, query, options))
        .collect())
}

fn render_search_owner_query_set(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Result<String, String> {
    if query_terms.iter().all(|term| exact_rust_file_query(term))
        && let Some(rendered) =
            render_exact_path_owner_query_set(project_root, config, query, query_terms, options)?
    {
        return Ok(rendered);
    }
    let contexts = search_contexts_for_path_queries(project_root, config, options, query_terms)?;
    Ok(contexts
        .iter()
        .map(|context| {
            render_search_owner_query_set_block(project_root, context, query, query_terms, options)
        })
        .collect())
}

fn render_exact_path_owner_query_set(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Result<Option<String>, String> {
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let include_items = has_pipe(options, "items");
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for package_root in package_roots {
        let modules = query_terms
            .iter()
            .flat_map(|term| {
                exact_owner_path_matches(project_root, std::slice::from_ref(&package_root), term)
            })
            .map(|(_, path)| parse_rust_file(&path))
            .collect::<Vec<_>>();
        if modules.is_empty() {
            continue;
        }
        let module_refs = modules.iter().collect::<Vec<_>>();
        let item_count =
            owner_item_count(&module_refs, include_items, options.item_query.as_deref());
        let item_query_field = options
            .item_query
            .as_deref()
            .filter(|query| !query.is_empty())
            .map(|query| format!(" itemQuery={query}"))
            .unwrap_or_default();
        let mut block = format!(
            "[search-owner] q={} querySet={} selector=exact-set pkg={} own={} item={}{}\n",
            query,
            query_terms.len(),
            package_label(project_root, &package_root),
            modules.len(),
            item_count,
            item_query_field
        );
        for module in &modules {
            append_parser_visible_owner_line(&mut block, &package_root, module);
        }
        append_item_query_line(&mut block, &module_refs, options.item_query.as_deref());
        append_owner_item_hot_lines(
            &mut block,
            &package_root,
            &module_refs,
            include_items,
            options.item_query.as_deref(),
        );
        append_owner_item_lines(
            &mut block,
            project_root,
            &module_refs,
            include_items,
            options.item_query.as_deref(),
            options.item_projection_metadata,
        );
        append_unique_lines(
            &mut block,
            exact_owner_test_lines(&contexts, &package_root, &modules),
        );
        append_unique_lines(
            &mut block,
            exact_owner_synthesis_lines(&contexts, &package_root, &modules),
        );
        rendered.push_str(&block);
    }
    Ok((!rendered.is_empty()).then_some(rendered))
}

fn render_exact_path_owner(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<Option<String>, String> {
    if !exact_rust_file_query(query) {
        return Ok(None);
    }
    let include_items = has_pipe(options, "items");
    if options.output_view.as_deref() == Some("seeds") && !include_items {
        return render_exact_path_owner_seed_view(project_root, config, query, options);
    }
    if include_items
        && !has_pipe(options, "tests")
        && options.package.is_none()
        && let Some((package_root, path)) = direct_exact_owner_path_match(project_root, query)
    {
        let module = parse_rust_file(&path);
        return Ok(Some(render_exact_path_owner_block(ExactPathOwnerBlock {
            project_root,
            package_root: &package_root,
            query,
            module: &module,
            include_items,
            include_tests: false,
            item_query: options.item_query.as_deref(),
            item_projection_metadata: options.item_projection_metadata,
            test_lines: Vec::new(),
            synthesis_lines: Vec::new(),
        })));
    }
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let matches = exact_owner_path_matches(project_root, &package_roots, query);
    if matches.is_empty() {
        return Ok(None);
    }

    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for (package_root, path) in matches {
        let module = parse_rust_file(&path);
        let test_lines =
            exact_owner_test_lines(&contexts, &package_root, std::slice::from_ref(&module));
        let synthesis_lines =
            exact_owner_synthesis_lines(&contexts, &package_root, std::slice::from_ref(&module));
        rendered.push_str(&render_exact_path_owner_block(ExactPathOwnerBlock {
            project_root,
            package_root: &package_root,
            query,
            module: &module,
            include_items,
            include_tests: !include_items || has_pipe(options, "tests"),
            item_query: options.item_query.as_deref(),
            item_projection_metadata: options.item_projection_metadata,
            test_lines,
            synthesis_lines,
        }));
    }
    Ok(Some(rendered))
}

fn direct_exact_owner_path_match(project_root: &Path, query: &str) -> Option<(PathBuf, PathBuf)> {
    let query = query.replace('\\', "/");
    let query_path = Path::new(&query);
    let path = if query_path.is_absolute() {
        query_path.to_path_buf()
    } else {
        project_root.join(query_path)
    };
    let extension = path.extension().and_then(|extension| extension.to_str());
    if extension == Some("rs") && path.is_file() {
        let package_root = path
            .ancestors()
            .find(|candidate| candidate.join("Cargo.toml").is_file())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| project_root.to_path_buf());
        Some((package_root, path))
    } else {
        None
    }
}

struct ExactPathOwnerBlock<'a> {
    project_root: &'a Path,
    package_root: &'a Path,
    query: &'a str,
    module: &'a ParsedRustModule,
    include_items: bool,
    include_tests: bool,
    item_query: Option<&'a str>,
    item_projection_metadata: bool,
    test_lines: Vec<String>,
    synthesis_lines: Vec<String>,
}

fn render_exact_path_owner_block(input: ExactPathOwnerBlock<'_>) -> String {
    let mut block = render_exact_path_owner_header(
        input.project_root,
        input.package_root,
        input.query,
        input.module,
        input.include_items,
        input.item_query,
    );
    if input.include_items {
        append_parser_visible_owner_line_without_next(&mut block, input.package_root, input.module);
    } else {
        append_parser_visible_owner_line(&mut block, input.package_root, input.module);
    }
    append_item_query_line(&mut block, &[input.module], input.item_query);
    append_owner_item_hot_lines(
        &mut block,
        input.package_root,
        &[input.module],
        input.include_items,
        input.item_query,
    );
    append_owner_item_frontier_lines(
        &mut block,
        input.package_root,
        &[input.module],
        input.include_items,
    );
    append_owner_item_lines(
        &mut block,
        input.project_root,
        &[input.module],
        input.include_items,
        input.item_query,
        input.item_projection_metadata,
    );
    if input.include_tests {
        append_unique_lines(&mut block, input.test_lines);
    }
    if !input.include_items {
        append_unique_lines(&mut block, input.synthesis_lines);
    }
    block
}

fn render_search_owner_block(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> String {
    let include_items = options.pipes.iter().any(|pipe| pipe == "items");
    let include_tests = options.pipes.iter().any(|pipe| pipe == "tests");
    let matching_branches = matching_owner_branches(context, query);
    let matching_modules = matching_owner_modules(context, query);
    let mut block = render_search_owner_header(
        project_root,
        context,
        query,
        &matching_branches,
        &matching_modules,
        include_items,
        options.item_query.as_deref(),
    );
    append_owner_graph_or_fallback_lines(
        &mut block,
        context,
        query,
        &matching_branches,
        &matching_modules,
    );
    append_item_query_line(&mut block, &matching_modules, options.item_query.as_deref());
    append_owner_item_hot_lines(
        &mut block,
        &context.package_root,
        &matching_modules,
        include_items,
        options.item_query.as_deref(),
    );
    append_owner_item_frontier_lines(
        &mut block,
        &context.package_root,
        &matching_modules,
        include_items,
    );
    append_owner_item_lines(
        &mut block,
        project_root,
        &matching_modules,
        include_items,
        options.item_query.as_deref(),
        options.item_projection_metadata,
    );
    if include_tests {
        append_unique_lines(
            &mut block,
            owner_search::test_lines_for_owner_modules(context, &matching_modules),
        );
    }
    append_unique_lines(
        &mut block,
        owner_synthesis_lines_for_paths(
            context,
            matching_modules
                .iter()
                .map(|module| module.report.path.clone())
                .collect(),
        ),
    );
    block
}

fn render_search_owner_query_set_block(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> String {
    let include_items = options.pipes.iter().any(|pipe| pipe == "items");
    let include_tests = options.pipes.iter().any(|pipe| pipe == "tests");
    let details = query_terms
        .iter()
        .map(|term| {
            owner_query_details(
                project_root,
                context,
                term,
                include_items,
                include_tests,
                options.item_projection_metadata,
            )
        })
        .collect::<Vec<_>>();
    let mut block = format!(
        "[search-owner] q={} querySet={} selector=exact-set pkg={} own={} item={}\n",
        query,
        query_terms.len(),
        package_label(project_root, &context.package_root),
        details.iter().map(|detail| detail.owners).sum::<usize>(),
        details.iter().map(|detail| detail.items).sum::<usize>()
    );
    append_unique_lines(
        &mut block,
        details.into_iter().flat_map(|detail| detail.lines),
    );
    block
}

struct OwnerQueryDetails {
    owners: usize,
    items: usize,
    lines: Vec<String>,
}

fn owner_query_details(
    locator_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    include_items: bool,
    include_tests: bool,
    item_projection_metadata: bool,
) -> OwnerQueryDetails {
    let matching_branches = matching_owner_branches(context, query);
    let matching_modules = matching_owner_modules(context, query);
    let owners = search_owner_count(context, query, &matching_branches, &matching_modules);
    let items = if include_items {
        matching_modules
            .iter()
            .map(|module| module.syntax_facts.top_level_items.len())
            .sum()
    } else {
        0
    };
    let mut lines = String::new();
    append_owner_graph_or_fallback_lines(
        &mut lines,
        context,
        query,
        &matching_branches,
        &matching_modules,
    );
    append_owner_item_lines(
        &mut lines,
        locator_root,
        &matching_modules,
        include_items,
        None,
        item_projection_metadata,
    );
    append_owner_item_frontier_lines(
        &mut lines,
        &context.package_root,
        &matching_modules,
        include_items,
    );
    if include_tests {
        append_unique_lines(
            &mut lines,
            owner_search::test_lines_for_owner_modules(context, &matching_modules),
        );
    }
    append_unique_lines(
        &mut lines,
        owner_synthesis_lines_for_paths(
            context,
            matching_modules
                .iter()
                .map(|module| module.report.path.clone())
                .collect(),
        ),
    );
    OwnerQueryDetails {
        owners,
        items,
        lines: lines.lines().map(ToOwned::to_owned).collect(),
    }
}

fn exact_owner_test_lines(
    contexts: &[PackageSearchContext],
    package_root: &Path,
    modules: &[ParsedRustModule],
) -> Vec<String> {
    let Some(context) = contexts
        .iter()
        .find(|context| context.package_root == package_root)
    else {
        return Vec::new();
    };
    let owner_modules = modules
        .iter()
        .filter_map(|module| {
            context
                .parsed_modules
                .iter()
                .find(|candidate| candidate.report.path == module.report.path)
        })
        .collect::<Vec<_>>();
    owner_search::test_lines_for_owner_modules(context, &owner_modules)
}

fn exact_owner_synthesis_lines(
    contexts: &[PackageSearchContext],
    package_root: &Path,
    modules: &[ParsedRustModule],
) -> Vec<String> {
    let Some(context) = contexts
        .iter()
        .find(|context| context.package_root == package_root)
    else {
        return Vec::new();
    };
    owner_synthesis_lines_for_paths(
        context,
        modules
            .iter()
            .map(|module| module.report.path.clone())
            .collect(),
    )
}

fn owner_synthesis_lines_for_paths(
    context: &PackageSearchContext,
    owner_paths: Vec<PathBuf>,
) -> Vec<String> {
    let Some(line) = owner_graph_synthesis_line(context, &owner_paths) else {
        return Vec::new();
    };
    vec![line]
}

fn owner_graph_synthesis_line(
    context: &PackageSearchContext,
    owner_paths: &[PathBuf],
) -> Option<String> {
    if owner_paths.is_empty() {
        return None;
    }
    let selected = owner_paths.iter().cloned().collect::<BTreeSet<_>>();
    let mut incoming = BTreeSet::new();
    let mut outgoing = BTreeSet::new();
    for branch in &context.reasoning_tree.owner_branches {
        for edge in &branch.declared_child_edges {
            collect_owner_frontier_edge(
                &selected,
                &mut incoming,
                &mut outgoing,
                &branch.path,
                &edge.child_path,
            );
        }
    }
    for dependency in context
        .reasoning_tree
        .owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
    {
        collect_owner_frontier_edge(
            &selected,
            &mut incoming,
            &mut outgoing,
            &dependency.source_path,
            &dependency.target_path,
        );
    }
    let frontier = incoming
        .iter()
        .chain(outgoing.iter())
        .filter(|path| !selected.contains(*path))
        .map(|path| display_project_path(&context.package_root, path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(4)
        .collect::<Vec<_>>();
    let seeds = frontier
        .iter()
        .map(|path| format!("owner:{path}"))
        .collect::<Vec<_>>();
    let mut parts = vec![
        "algorithm=bounded-reachability-depth1".to_string(),
        "scope=owner".to_string(),
        "summary=owner-graph-frontier".to_string(),
        format!("selected_owners={}", selected.len()),
        format!("incoming_owners={}", incoming.len()),
        format!("outgoing_owners={}", outgoing.len()),
    ];
    if owner_paths.len() == 1 {
        parts.push(format!(
            "owner_path={}",
            display_project_path(&context.package_root, &owner_paths[0])
        ));
    }
    if !frontier.is_empty() {
        parts.push(format!("frontier_owners={}", frontier.join(",")));
    }
    if !seeds.is_empty() {
        parts.push(format!("seeds={}", seeds.join(",")));
    }
    Some(format!("|synthesis {}", parts.join(" ")))
}

fn collect_owner_frontier_edge(
    selected: &BTreeSet<PathBuf>,
    incoming: &mut BTreeSet<PathBuf>,
    outgoing: &mut BTreeSet<PathBuf>,
    source_path: &PathBuf,
    target_path: &PathBuf,
) {
    if selected.contains(source_path) && !selected.contains(target_path) {
        outgoing.insert(target_path.clone());
    }
    if selected.contains(target_path) && !selected.contains(source_path) {
        incoming.insert(source_path.clone());
    }
}

fn append_unique_lines(block: &mut String, lines: impl IntoIterator<Item = String>) {
    let mut seen = std::collections::BTreeSet::new();
    for line in lines {
        if seen.insert(line.clone()) {
            let _ = writeln!(block, "{line}");
        }
    }
}

fn matching_owner_branches<'a>(
    context: &'a PackageSearchContext,
    query: &str,
) -> Vec<&'a RustReasoningOwnerBranchFacts> {
    context
        .reasoning_tree
        .owner_branches
        .iter()
        .filter(|branch| owner_branch_matches(&context.package_root, branch, query))
        .collect()
}

fn matching_owner_modules<'a>(
    context: &'a PackageSearchContext,
    query: &str,
) -> Vec<&'a ParsedRustModule> {
    context
        .parsed_modules
        .iter()
        .filter(|module| owner_path_matches(&context.package_root, &module.report.path, query))
        .collect()
}

fn append_owner_graph_or_fallback_lines(
    block: &mut String,
    context: &PackageSearchContext,
    query: &str,
    matching_branches: &[&RustReasoningOwnerBranchFacts],
    matching_modules: &[&ParsedRustModule],
) {
    if matching_branches.is_empty() {
        append_owner_fallback_lines(block, context, query, matching_modules);
    } else {
        append_reasoning_owner_lines(block, context, matching_branches);
    }
}

fn append_reasoning_owner_lines(
    block: &mut String,
    context: &PackageSearchContext,
    matching_branches: &[&RustReasoningOwnerBranchFacts],
) {
    let modules_by_path = context
        .parsed_modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    for branch in matching_branches.iter().take(SEARCH_OWNER_LIMIT) {
        let module = modules_by_path.get(&branch.path).copied();
        let _ = writeln!(
            block,
            "{}",
            render_owner_line(&context.package_root, branch, module)
        );
    }
}

fn has_pipe(options: &RustSearchOptions, pipe: &str) -> bool {
    options.pipes.iter().any(|candidate| candidate == pipe)
}

fn append_owner_item_lines(
    block: &mut String,
    locator_root: &Path,
    matching_modules: &[&ParsedRustModule],
    include_items: bool,
    item_query: Option<&str>,
    item_projection_metadata: bool,
) {
    if !include_items {
        return;
    }
    let names_only = item_query.is_some();
    for line in render_owner_item_lines(
        locator_root,
        matching_modules,
        item_query,
        names_only,
        item_projection_metadata,
    ) {
        let _ = writeln!(block, "{line}");
    }
}

fn append_item_query_line(
    block: &mut String,
    matching_modules: &[&ParsedRustModule],
    item_query: Option<&str>,
) {
    let names_only = item_query.is_some();
    if let Some(line) = render_item_query_line(matching_modules, item_query, names_only) {
        let _ = writeln!(block, "{line}");
    }
}

fn append_owner_item_hot_lines(
    block: &mut String,
    package_root: &Path,
    matching_modules: &[&ParsedRustModule],
    include_items: bool,
    item_query: Option<&str>,
) {
    if !include_items {
        return;
    }
    for line in render_owner_item_hot_lines(package_root, matching_modules, item_query) {
        let _ = writeln!(block, "{line}");
    }
}

fn append_owner_item_frontier_lines(
    block: &mut String,
    package_root: &Path,
    matching_modules: &[&ParsedRustModule],
    include_items: bool,
) {
    if include_items {
        return;
    }
    for line in render_owner_item_frontier_lines(package_root, matching_modules) {
        let _ = writeln!(block, "{line}");
    }
}

fn render_search_owner_header(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    matching_branches: &[&RustReasoningOwnerBranchFacts],
    matching_modules: &[&ParsedRustModule],
    include_items: bool,
    item_query: Option<&str>,
) -> String {
    let item_count = owner_item_count(matching_modules, include_items, item_query);
    let item_query_field = item_query
        .filter(|query| !query.is_empty())
        .map(|query| format!(" itemQuery={query}"))
        .unwrap_or_default();
    format!(
        "[search-owner] q={} pkg={} own={} item={}{}\n",
        query,
        package_label(project_root, &context.package_root),
        search_owner_count(context, query, matching_branches, matching_modules),
        item_count,
        item_query_field
    )
}

fn render_exact_path_owner_header(
    project_root: &Path,
    package_root: &Path,
    query: &str,
    module: &ParsedRustModule,
    include_items: bool,
    item_query: Option<&str>,
) -> String {
    let item_count = owner_item_count(&[module], include_items, item_query);
    let item_query_field = item_query
        .filter(|query| !query.is_empty())
        .map(|query| format!(" itemQuery={query}"))
        .unwrap_or_default();
    format!(
        "[search-owner] q={} pkg={} own=1 item={}{}\n",
        query,
        package_label(project_root, package_root),
        item_count,
        item_query_field
    )
}

fn search_owner_count(
    context: &PackageSearchContext,
    query: &str,
    matching_branches: &[&RustReasoningOwnerBranchFacts],
    matching_modules: &[&ParsedRustModule],
) -> usize {
    if !matching_branches.is_empty() || !matching_modules.is_empty() {
        return matching_branches.len().max(matching_modules.len());
    }
    usize::from(path_only_owner_path(context, query).is_some())
}

fn append_owner_fallback_lines(
    block: &mut String,
    context: &PackageSearchContext,
    query: &str,
    matching_modules: &[&ParsedRustModule],
) {
    if !matching_modules.is_empty() {
        append_parser_visible_owner_lines(block, context, matching_modules);
    } else if let Some(path) = path_only_owner_path(context, query) {
        append_path_only_owner_line(block, context, &path);
    } else {
        append_owner_not_found_line(block, query);
    }
}

fn append_parser_visible_owner_lines(
    block: &mut String,
    context: &PackageSearchContext,
    matching_modules: &[&ParsedRustModule],
) {
    for module in matching_modules.iter().take(SEARCH_OWNER_LIMIT) {
        append_parser_visible_owner_line(block, &context.package_root, module);
    }
}

fn append_parser_visible_owner_line(
    block: &mut String,
    package_root: &Path,
    module: &ParsedRustModule,
) {
    let path = display_project_path(package_root, &module.report.path);
    let role = owner_role_for_path(package_root, &module.report.path);
    let syntax_diagnostics = usize::from(module.report.parse_error.is_some());
    let mut fields = vec![
        format!("|owner {path}"),
        format!("role={role}"),
        "source=parser-visible-module".to_string(),
        format!("lines={}", module.source.lines().count()),
    ];
    if !module.report.is_valid {
        fields.push("valid=false".to_string());
    }
    if syntax_diagnostics > 0 {
        fields.push(format!("syntaxDiagnostics={syntax_diagnostics}"));
    }
    let imports = module_import_count(module);
    if imports > 0 {
        fields.push(format!("imports={imports}"));
    }
    fields.push(format!("next=owner:{path},tests:{path}"));
    let _ = writeln!(block, "{}", fields.join(" "));
}

fn append_parser_visible_owner_line_without_next(
    block: &mut String,
    package_root: &Path,
    module: &ParsedRustModule,
) {
    let mut owner_line = String::new();
    append_parser_visible_owner_line(&mut owner_line, package_root, module);
    if let Some(next_index) = owner_line.find(" next=") {
        let line_end = owner_line.trim_end().len();
        owner_line.replace_range(next_index..line_end, "");
    }
    block.push_str(&owner_line);
}

fn append_path_only_owner_line(block: &mut String, context: &PackageSearchContext, path: &Path) {
    let path = display_project_path(&context.package_root, path);
    let role = owner_role_for_path(&context.package_root, Path::new(&path));
    let _ = writeln!(
        block,
        "|owner {path} role={role} source=path-only next=ingest:{path}"
    );
    block.push_str(
        "|note kind=owner-not-found message=\"path exists but is not a parser-visible owner; use search ingest for line evidence\"\n",
    );
}

fn append_owner_not_found_line(block: &mut String, query: &str) {
    let _ = writeln!(
        block,
        "|note kind=not-found message=\"owner not found: {query}\""
    );
    block.push_str("|next prime:search-prime\n");
}

fn module_import_count(module: &ParsedRustModule) -> usize {
    module
        .syntax_facts
        .use_statements
        .iter()
        .map(|statement| statement.imports.len())
        .sum()
}

fn path_only_owner_path(context: &PackageSearchContext, query: &str) -> Option<PathBuf> {
    let query_path = Path::new(query);
    let absolute = if query_path.is_absolute() {
        query_path.to_path_buf()
    } else {
        context.package_root.join(query_path)
    };
    if absolute.exists() {
        Some(absolute)
    } else if query_path.is_absolute() {
        None
    } else {
        context
            .package_root
            .ancestors()
            .map(|ancestor| ancestor.join(query_path))
            .find(|candidate| candidate.exists() && candidate.starts_with(&context.package_root))
    }
}
