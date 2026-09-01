use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::AspRustConfig;
use crate::AspRustScope;
use crate::discovery::{asp_rust_scope, discover_rust_files};
use crate::parser::{ParsedRustModule, parse_rust_file};

use super::RustSearchOptions;
use super::context::{
    exact_owner_path_matches, exact_rust_file_query, search_contexts,
    search_contexts_for_path_query,
};
use super::format::{
    append_block, display_project_path, package_label, package_roots_for_request, query_set_terms,
    render_public_api_line,
};
use super::hits::OwnerHit;
use super::limits::{SEARCH_OWNER_LIMIT, SEARCH_TEST_LIMIT};
use super::scope::{module_is_scope, owner_path_matches};

pub(super) fn render_search_tests(
    project_root: &Path,
    config: &AspRustConfig,
    query: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    if let Some(query) = query
        && query_set_terms(query).len() > 1
    {
        return render_search_tests_query_set(project_root, config, query, options);
    }
    if let Some(query) = query
        && let Some(rendered) = render_exact_path_tests(project_root, config, query, options)?
    {
        return Ok(rendered);
    }
    let contexts = query.map_or_else(
        || search_contexts(project_root, config, options),
        |query| search_contexts_for_path_query(project_root, config, options, query),
    )?;
    let mut rendered = String::new();
    for context in contexts {
        let owner_modules = query
            .map(|query| test_subject_modules(&context, query))
            .unwrap_or_default();
        let owner_tokens = test_subject_tokens(&owner_modules);
        let tests = context
            .parsed_modules
            .iter()
            .filter(|module| module_is_scope(&context.scope, module, "tests"))
            .filter_map(|module| {
                test_match(
                    &context.package_root,
                    module,
                    query,
                    !owner_modules.is_empty(),
                    &owner_tokens,
                )
            })
            .collect::<Vec<_>>();
        let tests = sort_test_matches(tests);
        let block = render_search_tests_block(
            project_root,
            &context.package_root,
            query.unwrap_or("-"),
            &owner_modules,
            tests,
            None,
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_tests_query_set(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let query_terms = query_set_terms(query);
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = String::new();
    for package_root in package_roots {
        let scope = asp_rust_scope(
            &package_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        );
        let owner_modules = query_set_owner_modules(&package_root, &scope, config, &query_terms);
        let owner_refs = owner_modules.iter().collect::<Vec<_>>();
        let owner_tokens = test_subject_tokens(&owner_refs);
        let test_modules = parse_test_scope(&scope, config);
        let tests = test_modules
            .iter()
            .filter_map(|module| {
                test_match(&package_root, module, Some(query), true, &owner_tokens)
            })
            .collect::<Vec<_>>();
        let tests = sort_test_matches(tests);
        let block = render_search_tests_block(
            project_root,
            &package_root,
            query,
            &owner_refs,
            tests,
            Some(query_terms.len()),
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn query_set_owner_modules(
    package_root: &Path,
    scope: &AspRustScope,
    config: &AspRustConfig,
    query_terms: &[&str],
) -> Vec<ParsedRustModule> {
    let query_paths = query_terms
        .iter()
        .filter(|query| exact_rust_file_query(query))
        .collect::<Vec<_>>();
    if query_paths.is_empty() {
        return Vec::new();
    }
    discover_rust_files(
        &scope.source_paths,
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    )
    .into_iter()
    .filter(|path| {
        query_paths
            .iter()
            .any(|query| owner_path_matches(package_root, path, query))
    })
    .map(|path| parse_rust_file(&path))
    .filter(|module| !module_is_scope(scope, module, "tests"))
    .collect()
}

fn render_exact_path_tests(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<Option<String>, String> {
    if !exact_rust_file_query(query) {
        return Ok(None);
    }
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let matches = exact_owner_path_matches(project_root, &package_roots, query);
    if matches.is_empty() {
        return Ok(None);
    }

    let mut rendered = String::new();
    for (package_root, path) in matches {
        let scope = asp_rust_scope(
            &package_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        );
        let owner_module = parse_rust_file(&path);
        if module_is_scope(&scope, &owner_module, "tests") {
            return Ok(None);
        }
        let test_modules = parse_test_scope(&scope, config);
        let owner_modules = [&owner_module];
        let owner_tokens = test_subject_tokens(&owner_modules);
        let tests = test_modules
            .iter()
            .filter_map(|module| {
                test_match(&package_root, module, Some(query), true, &owner_tokens)
            })
            .collect::<Vec<_>>();
        let tests = sort_test_matches(tests);
        let block = render_search_tests_block(
            project_root,
            &package_root,
            query,
            &owner_modules,
            tests,
            None,
        );
        append_block(&mut rendered, &block);
    }
    Ok(Some(rendered))
}

fn parse_test_scope(scope: &AspRustScope, config: &AspRustConfig) -> Vec<ParsedRustModule> {
    discover_rust_files(
        &scope.test_paths,
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    )
    .into_iter()
    .map(|path| parse_rust_file(&path))
    .collect()
}

fn render_search_tests_block(
    project_root: &Path,
    package_root: &Path,
    query: &str,
    owner_modules: &[&ParsedRustModule],
    tests: Vec<TestSearchMatch>,
    query_set_count: Option<usize>,
) -> String {
    let mut block = format!("[search-tests] q={}", query,);
    if let Some(query_set_count) = query_set_count {
        let _ = write!(block, " querySet={query_set_count} selector=exact-set");
    }
    let _ = writeln!(
        block,
        " pkg={} tests={} own={}",
        package_label(project_root, package_root),
        tests.len(),
        owner_modules.len()
    );
    for module in owner_modules.iter().take(SEARCH_OWNER_LIMIT) {
        let owner_path = display_project_path(package_root, &module.report.path);
        let _ = writeln!(
            block,
            "|node O:{owner_path} kind=owner path={owner_path} next=owner:{owner_path}"
        );
    }
    for test in tests.into_iter().take(SEARCH_TEST_LIMIT) {
        append_test_lines(&mut block, &test);
    }
    block
}

struct TestSearchMatch {
    path: String,
    functions: usize,
    owner_path: Option<String>,
    reasons: Vec<String>,
}

fn sort_test_matches(mut tests: Vec<TestSearchMatch>) -> Vec<TestSearchMatch> {
    tests.sort_by(|left, right| {
        test_match_score(right)
            .cmp(&test_match_score(left))
            .then_with(|| left.path.cmp(&right.path))
    });
    tests.dedup_by(|left, right| left.path == right.path);
    tests
}

fn test_match_score(test: &TestSearchMatch) -> usize {
    let mut score = 0;
    if let Some(owner_path) = test.owner_path.as_deref() {
        score += owner_path_test_score(owner_path, &test.path);
    }
    score += test
        .reasons
        .iter()
        .filter(|reason| !reason.ends_with(":Builder"))
        .count()
        * 10;
    score + test.reasons.len()
}

fn owner_path_test_score(owner_path: &str, test_path: &str) -> usize {
    let owner = Path::new(owner_path);
    let test = Path::new(test_path);
    let owner_stem = owner
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let test_stem = test
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let mut score = 0;
    if !owner_stem.is_empty() && test_stem.contains(owner_stem) {
        score += 1000;
    }
    if let Some(owner_parent) = owner
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        && !owner_parent.is_empty()
        && test_stem.contains(owner_parent)
    {
        score += 100;
    }
    score
}

impl TestSearchMatch {
    fn metadata(&self) -> String {
        let mut parts = Vec::new();
        if let Some(owner_path) = self.owner_path.as_deref() {
            parts.push(format!("owner={owner_path}"));
        }
        if !self.reasons.is_empty() {
            parts.push(format!("reason={}", self.reasons.join(",")));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" {}", parts.join(" "))
        }
    }
}

fn test_subject_modules<'a>(
    context: &'a super::context::PackageSearchContext,
    query: &str,
) -> Vec<&'a ParsedRustModule> {
    context
        .parsed_modules
        .iter()
        .filter(|module| !module_is_scope(&context.scope, module, "tests"))
        .filter(|module| owner_path_matches(&context.package_root, &module.report.path, query))
        .collect()
}

fn test_subject_tokens(modules: &[&ParsedRustModule]) -> Vec<(PathBuf, String)> {
    let mut seen = BTreeSet::new();
    let mut tokens = Vec::new();
    for module in modules {
        for item in &module.syntax_facts.top_level_items {
            for token in item.name.iter().chain(item.function_name.iter()) {
                if token.len() > 1 && seen.insert((module.report.path.clone(), token.clone())) {
                    tokens.push((module.report.path.clone(), token.clone()));
                }
            }
        }
    }
    tokens
}

pub(super) fn test_lines_for_owner_modules(
    context: &super::context::PackageSearchContext,
    owner_modules: &[&ParsedRustModule],
) -> Vec<String> {
    let owner_tokens = public_test_subject_tokens(owner_modules);
    if owner_tokens.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for module in context
        .parsed_modules
        .iter()
        .filter(|module| module_is_scope(&context.scope, module, "tests"))
    {
        let Some((owner_path, reasons)) =
            test_owner_symbol_reasons(&context.package_root, &module.source, &owner_tokens)
        else {
            continue;
        };
        let mut rendered = String::new();
        append_test_lines(
            &mut rendered,
            &TestSearchMatch {
                path: display_project_path(&context.package_root, &module.report.path),
                functions: module.syntax_facts.test_function_count,
                owner_path: Some(owner_path),
                reasons,
            },
        );
        lines.extend(rendered.lines().map(ToOwned::to_owned));
    }
    lines
}

fn public_test_subject_tokens(modules: &[&ParsedRustModule]) -> Vec<(PathBuf, String)> {
    let mut seen = BTreeSet::new();
    modules
        .iter()
        .flat_map(|module| public_module_test_tokens(module))
        .filter(|(path, token)| token.len() > 1 && seen.insert((path.clone(), token.clone())))
        .collect()
}

fn public_module_test_tokens(module: &ParsedRustModule) -> Vec<(PathBuf, String)> {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.is_public)
        .flat_map(|item| item.name.iter().chain(item.function_name.iter()))
        .map(|token| (module.report.path.clone(), token.clone()))
        .collect()
}

fn test_match(
    package_root: &Path,
    module: &ParsedRustModule,
    query: Option<&str>,
    has_owner_query: bool,
    owner_tokens: &[(PathBuf, String)],
) -> Option<TestSearchMatch> {
    let path = display_project_path(package_root, &module.report.path);
    let Some(query) = query else {
        return Some(TestSearchMatch {
            path,
            functions: module.syntax_facts.test_function_count,
            owner_path: None,
            reasons: Vec::new(),
        });
    };
    if owner_path_matches(package_root, &module.report.path, query)
        || (!has_owner_query && module.source.contains(query))
    {
        return Some(TestSearchMatch {
            path,
            functions: module.syntax_facts.test_function_count,
            owner_path: None,
            reasons: Vec::new(),
        });
    }
    let (owner_path, reasons) =
        test_owner_symbol_reasons(package_root, &module.source, owner_tokens)?;
    Some(TestSearchMatch {
        path,
        functions: module.syntax_facts.test_function_count,
        owner_path: Some(owner_path),
        reasons,
    })
}

fn test_owner_symbol_reasons(
    package_root: &Path,
    source: &str,
    owner_tokens: &[(PathBuf, String)],
) -> Option<(String, Vec<String>)> {
    let (owner_path, _) = owner_tokens
        .iter()
        .find(|(_, token)| source.contains(token))?;
    let reasons = owner_tokens
        .iter()
        .filter(|(candidate_path, token)| candidate_path == owner_path && source.contains(token))
        .map(|(_, token)| format!("symbol:{token}"))
        .collect::<Vec<_>>();
    Some((display_project_path(package_root, owner_path), reasons))
}

fn append_test_lines(block: &mut String, test: &TestSearchMatch) {
    let _ = writeln!(
        block,
        "|test {} functions={}{} next=owner:{}",
        test.path,
        test.functions,
        test.metadata(),
        test.path
    );
    if let Some(owner_path) = test.owner_path.as_deref() {
        let _ = writeln!(block, "|edge O:{owner_path} -test-> T:{}", test.path);
    }
}

pub(super) fn public_api_lines_for_dependency(
    context: &super::context::PackageSearchContext,
    query: &str,
    usage: &[OwnerHit],
    api_filter: Option<&str>,
) -> Vec<String> {
    let api_filter = api_filter.map(ToOwned::to_owned);
    usage
        .iter()
        .flat_map(|hit| {
            let api_filter = api_filter.clone();
            context
                .parsed_modules
                .iter()
                .filter(move |module| module.report.path == hit.path)
                .flat_map(move |module| {
                    let api_filter = api_filter.clone();
                    module
                        .syntax_facts
                        .top_level_items
                        .iter()
                        .filter_map(move |item| {
                            if !item.is_public
                                || !public_item_matches_api_filter(
                                    module,
                                    item,
                                    api_filter.as_deref(),
                                )
                            {
                                return None;
                            }
                            render_public_api_line(
                                &context.package_root,
                                &module.report.path,
                                query,
                                item,
                            )
                        })
                })
        })
        .collect()
}

fn public_item_matches_api_filter(
    module: &ParsedRustModule,
    item: &crate::parser::RustTopLevelItemSyntax,
    api_filter: Option<&str>,
) -> bool {
    let Some(api_filter) = api_filter else {
        return true;
    };
    item.name.as_deref() == Some(api_filter)
        || item.function_name.as_deref() == Some(api_filter)
        || item_context_mentions_api(&module.source, item.line, api_filter)
}

fn item_context_mentions_api(source: &str, line: usize, api_filter: &str) -> bool {
    let line_index = line.saturating_sub(1);
    source
        .lines()
        .skip(line_index.saturating_sub(2))
        .take(3)
        .any(|line| line.contains(api_filter))
}
