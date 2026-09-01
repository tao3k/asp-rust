use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use crate::AspRustConfig;
use crate::parser::ParsedRustModule;

use super::RustSearchOptions;
use super::context::{PackageSearchContext, search_contexts};
use super::format::{
    append_block, compact_locations, display_project_path, package_label, query_set_terms,
    render_cargo_dependency_line, render_item_line,
};
use super::hits::{OwnerHit, dependency_usage, matching_dependencies};
use super::limits::{SEARCH_HIT_LIMIT, SEARCH_ITEM_LIMIT, SEARCH_OWNER_LIMIT, SEARCH_TEST_LIMIT};
use super::owner::{public_api_lines_for_dependency, test_lines_for_owner_modules};

pub(super) fn render_search_dependency(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let query_terms = query_set_terms(query);
    if options.output_view.as_deref() == Some("seeds") {
        let seed_terms = if query_terms.is_empty() {
            vec![query]
        } else {
            query_terms.clone()
        };
        return render_search_dependency_seed_view(
            project_root,
            config,
            query,
            &seed_terms,
            options,
        );
    }
    if query_terms.len() > 1 {
        return render_search_dependency_query_set(
            project_root,
            config,
            query,
            &query_terms,
            options,
        );
    }
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let block = render_search_dependency_block(project_root, &context, query, options);
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_dependency_seed_view(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Result<String, String> {
    let flags = DependencyPipeFlags::from_options(options);
    let query_terms = if query_terms.is_empty() {
        vec![query]
    } else {
        query_terms.to_vec()
    };
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let details = query_terms
            .iter()
            .map(|term| dependency_query_details(&context, term, &flags))
            .collect::<Vec<_>>();
        let dep_count = details.iter().map(|detail| detail.deps).sum();
        let mut block = dependency_seed_header(
            project_root,
            &context.package_root,
            query,
            query_terms.len(),
            dep_count,
            &flags,
        );
        let joined = query_terms.join(",");
        let _ = writeln!(block, "|seed deps:{joined}");
        let _ = writeln!(block, "|seed import:{joined}");
        let _ = writeln!(block, "|seed docs-use:{joined}");
        let _ = writeln!(block, "|seed crate-source:{joined}");
        let _ = writeln!(block, "|seed tests");
        let _ = writeln!(
            block,
            "|next dependency:{joined},docs-use:{joined},crate-source:{joined},tests"
        );
        let _ = writeln!(block, "avoid=web-search,docs.rs-search,raw-read");
        append_unique_detail_lines(
            &mut block,
            details.into_iter().flat_map(|detail| detail.lines),
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn dependency_seed_header(
    project_root: &Path,
    package_root: &Path,
    query: &str,
    query_set_count: usize,
    dep_count: usize,
    flags: &DependencyPipeFlags,
) -> String {
    let mut block = format!("[search-dependency] q={query}");
    if query_set_count > 1 {
        let _ = write!(block, " querySet={query_set_count} selector=exact-set");
    }
    let _ = write!(
        block,
        " pkg={} dep={} own=0 api=0",
        package_label(project_root, package_root),
        dep_count
    );
    if flags.items {
        block.push_str(" item=0");
    }
    if flags.docs {
        block.push_str(" docs=0");
    }
    if flags.tests {
        block.push_str(" tests=0");
    }
    block.push('\n');
    block
}

struct DependencyPipeFlags {
    items: bool,
    docs: bool,
    tests: bool,
    public_api: bool,
}

impl DependencyPipeFlags {
    fn from_options(options: &RustSearchOptions) -> Self {
        let docs = has_pipe(options, "docs") || has_pipe(options, "docs-use");
        Self {
            items: has_pipe(options, "items"),
            docs,
            tests: has_pipe(options, "tests"),
            public_api: has_pipe(options, "public-api") || docs,
        }
    }
}

fn render_search_dependency_block(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    options: &RustSearchOptions,
) -> String {
    let flags = DependencyPipeFlags::from_options(options);
    let details = dependency_query_details(context, query, &flags);
    let mut block = dependency_header(
        project_root,
        context,
        query,
        DependencyHeaderCounts {
            deps: details.deps,
            owners: details.owners,
            api: details.api,
            items: details.items,
            tests: details.tests,
        },
        &flags,
        None,
    );
    append_detail_lines(&mut block, details.lines);
    block
}

fn render_search_dependency_query_set(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    query_terms: &[&str],
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let flags = DependencyPipeFlags::from_options(options);
    let mut rendered = String::new();
    for context in contexts {
        let details = query_terms
            .iter()
            .map(|term| dependency_query_details(&context, term, &flags))
            .collect::<Vec<_>>();
        let counts = DependencyHeaderCounts {
            deps: details.iter().map(|detail| detail.deps).sum(),
            owners: details.iter().map(|detail| detail.owners).sum(),
            api: details.iter().map(|detail| detail.api).sum(),
            items: details.iter().map(|detail| detail.items).sum(),
            tests: details.iter().map(|detail| detail.tests).sum(),
        };
        let mut block = dependency_header(
            project_root,
            &context,
            query,
            counts,
            &flags,
            Some(query_terms.len()),
        );
        append_unique_detail_lines(
            &mut block,
            details.into_iter().flat_map(|detail| detail.lines),
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

struct DependencyQueryDetails {
    deps: usize,
    owners: usize,
    api: usize,
    items: usize,
    tests: usize,
    lines: Vec<String>,
}

fn dependency_query_details(
    context: &PackageSearchContext,
    query: &str,
    flags: &DependencyPipeFlags,
) -> DependencyQueryDetails {
    let deps = matching_dependencies(&context.cargo_dependencies, query);
    let usage = dependency_usage(context, query);
    let owner_modules = owner_modules_for_hits(context, &usage);
    let public_api = dependency_public_api_lines(context, query, &usage, flags.public_api);
    let api_count = public_api.len();
    let item_count = dependency_item_count(&owner_modules, flags.items);
    let test_lines = dependency_test_lines(context, &owner_modules, flags.tests);
    let test_count = rendered_test_count(&test_lines);
    let mut lines = String::new();
    append_dependency_facts(&mut lines, &deps);
    append_dependency_owner_hits(&mut lines, context, query, &usage, &owner_modules, flags);
    append_limited_lines(&mut lines, public_api, SEARCH_ITEM_LIMIT);
    append_limited_lines(&mut lines, test_lines, SEARCH_TEST_LIMIT);
    let _ = writeln!(
        lines,
        "|next deps:{query},import:{query},docs-use:{query},crate-source:{query},tests"
    );
    let _ = writeln!(lines, "avoid=web-search,docs.rs-search,raw-read");
    DependencyQueryDetails {
        deps: deps.len(),
        owners: usage.len(),
        api: api_count,
        items: item_count,
        tests: test_count,
        lines: lines.lines().map(ToOwned::to_owned).collect(),
    }
}

struct DependencyHeaderCounts {
    deps: usize,
    owners: usize,
    api: usize,
    items: usize,
    tests: usize,
}

fn dependency_header(
    project_root: &Path,
    context: &PackageSearchContext,
    query: &str,
    counts: DependencyHeaderCounts,
    flags: &DependencyPipeFlags,
    query_set_count: Option<usize>,
) -> String {
    let mut block = format!("[search-dependency] q={query}");
    if let Some(query_set_count) = query_set_count {
        let _ = write!(block, " querySet={query_set_count} selector=exact-set");
    }
    let _ = write!(
        block,
        " pkg={} dep={} own={} api={}",
        package_label(project_root, &context.package_root),
        counts.deps,
        counts.owners,
        counts.api
    );
    if flags.items {
        let _ = write!(block, " item={}", counts.items);
    }
    if flags.docs {
        let _ = write!(block, " docs={}", counts.api);
    }
    if flags.tests {
        let _ = write!(block, " tests={}", counts.tests);
    }
    block.push('\n');
    block
}

fn append_detail_lines(block: &mut String, lines: Vec<String>) {
    for line in lines {
        let _ = writeln!(block, "{line}");
    }
}

fn append_unique_detail_lines(block: &mut String, lines: impl IntoIterator<Item = String>) {
    let mut seen = BTreeSet::new();
    for line in lines {
        if seen.insert(line.clone()) {
            let _ = writeln!(block, "{line}");
        }
    }
}

fn dependency_public_api_lines(
    context: &PackageSearchContext,
    query: &str,
    usage: &[OwnerHit],
    enabled: bool,
) -> Vec<String> {
    if enabled {
        public_api_lines_for_dependency(context, query, usage, None)
    } else {
        Vec::new()
    }
}

fn dependency_item_count(owner_modules: &[&ParsedRustModule], enabled: bool) -> usize {
    if enabled {
        owner_modules
            .iter()
            .map(|module| named_top_level_item_count(module))
            .sum()
    } else {
        0
    }
}

fn dependency_test_lines(
    context: &PackageSearchContext,
    owner_modules: &[&ParsedRustModule],
    enabled: bool,
) -> Vec<String> {
    if enabled {
        test_lines_for_owner_modules(context, owner_modules)
    } else {
        Vec::new()
    }
}

fn rendered_test_count(lines: &[String]) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with("|test "))
        .count()
}

fn append_dependency_facts(block: &mut String, deps: &[&crate::parser::CargoDependencyFacts]) {
    for dependency in deps.iter().take(SEARCH_HIT_LIMIT) {
        let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
    }
}

fn append_dependency_owner_hits(
    block: &mut String,
    context: &PackageSearchContext,
    query: &str,
    usage: &[OwnerHit],
    owner_modules: &[&ParsedRustModule],
    flags: &DependencyPipeFlags,
) {
    for hit in usage.iter().take(SEARCH_OWNER_LIMIT) {
        append_dependency_owner_hit(block, context, query, hit, owner_modules, flags);
    }
}

fn append_dependency_owner_hit(
    block: &mut String,
    context: &PackageSearchContext,
    query: &str,
    hit: &OwnerHit,
    owner_modules: &[&ParsedRustModule],
    flags: &DependencyPipeFlags,
) {
    let owner_path = display_project_path(&context.package_root, &hit.path);
    let _ = writeln!(
        block,
        "|owner {} hit_kind=dependency locations={} next=tests",
        owner_path,
        compact_locations(&hit.locations)
    );
    let _ = writeln!(block, "|edge O:{owner_path} -ext:{query}-> D:{query}");
    if flags.items {
        append_hit_item_lines(block, hit, owner_modules);
    }
}

fn append_hit_item_lines(block: &mut String, hit: &OwnerHit, owner_modules: &[&ParsedRustModule]) {
    if let Some(module) = owner_modules
        .iter()
        .find(|module| module.report.path == hit.path)
    {
        append_item_lines(block, module);
    }
}

fn append_limited_lines(block: &mut String, lines: Vec<String>, limit: usize) {
    for line in lines.into_iter().take(limit) {
        let _ = writeln!(block, "{line}");
    }
}

fn owner_modules_for_hits<'a>(
    context: &'a PackageSearchContext,
    hits: &[OwnerHit],
) -> Vec<&'a ParsedRustModule> {
    let owner_paths = hits
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<BTreeSet<_>>();
    context
        .parsed_modules
        .iter()
        .filter(|module| owner_paths.contains(&module.report.path))
        .collect()
}

fn named_top_level_item_count(module: &ParsedRustModule) -> usize {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.name.is_some())
        .count()
}

fn append_item_lines(block: &mut String, module: &ParsedRustModule) {
    for item in module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.name.is_some())
        .take(SEARCH_ITEM_LIMIT)
    {
        let _ = writeln!(block, "{}", render_item_line(item));
    }
}

fn has_pipe(options: &RustSearchOptions, pipe: &str) -> bool {
    options.pipes.iter().any(|candidate| candidate == pipe)
}
