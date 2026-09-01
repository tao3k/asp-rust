use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use crate::AspRustConfig;
use crate::search::{
    CargoDependencyFacts, PackageSearchContext, ParsedRustModule, RustSearchOptions,
    SEARCH_HIT_LIMIT, SearchHit, append_block, asp_rust_scope, compact_locations,
    compare_paths_by_recency, discover_rust_files, display_project_path, import_hits,
    matching_dependencies, module_allowed, package_label, package_roots_for_request,
    parse_rust_file, path_allowed_by_scope, render_search_dependency, render_search_owner,
    search_contexts, search_contexts_for_path_query, sort_search_hits_by_recency, symbol_calls,
    symbol_definitions, version_requirement_matches_request,
};

pub(in crate::search) fn render_search_symbol(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    if options.output_view.as_deref() == Some("seeds") {
        return render_search_symbol_seed_hits(project_root, config, query, options);
    }
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let defs = symbol_definitions(&context, query, options);
        let calls = symbol_calls(&context, query, options);
        let mut block = format!(
            "[search-symbol] q={} pkg={} defs={} calls={}\n",
            query,
            package_label(project_root, &context.package_root),
            defs.len(),
            calls.len()
        );
        for hit in defs.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "|def {}", hit.render(&context.package_root));
        }
        for hit in calls.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "|call {}", hit.render(&context.package_root));
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_symbol_seed_hits(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
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
        let mut defs = 0;
        let mut calls = 0;
        let mut owners = Vec::<PathBuf>::new();
        for path in discover_rust_files(
            &scope.monitored_paths(),
            &config.ignored_dir_names,
            &config.include_hidden_dir_names,
        ) {
            let Ok(source) = fs::read_to_string(&path) else {
                continue;
            };
            if !source.contains(query) {
                continue;
            }
            let module = parse_rust_file(&path);
            if !path_allowed_by_scope(&scope, &package_root, &module.report.path, options) {
                continue;
            }
            let module_defs = module
                .syntax_facts
                .top_level_items
                .iter()
                .filter(|item| {
                    item.name.as_deref() == Some(query)
                        || item.function_name.as_deref() == Some(query)
                })
                .count();
            let module_calls = module
                .syntax_facts
                .function_calls
                .iter()
                .filter(|call| call.terminal_name == query)
                .count()
                + module
                    .syntax_facts
                    .path_references
                    .iter()
                    .filter(|reference| reference.terminal_name == query)
                    .count();
            if module_defs == 0 && module_calls == 0 {
                continue;
            }
            defs += module_defs;
            calls += module_calls;
            owners.push(module.report.path);
        }
        owners.sort_by(|left, right| compare_paths_by_recency(&package_root, left, right));
        owners.dedup();
        let mut block = format!(
            "[search-symbol] q={} pkg={} defs={} calls={}\n",
            query,
            package_label(project_root, &package_root),
            defs,
            calls
        );
        let seed_limit = options.seeds.unwrap_or(8);
        let owner_limit = seed_limit.min(owners.len());
        let owner_paths = owners
            .iter()
            .take(owner_limit)
            .map(|path| format!("owner:{}", display_project_path(&package_root, path)))
            .collect::<Vec<_>>();
        if !owner_paths.is_empty() {
            let _ = writeln!(block, "|seed {}", owner_paths.join(","));
        }
        if owners.len() > owner_limit {
            let _ = writeln!(
                block,
                "|note seeds_truncated={} limit={}",
                owners.len() - owner_limit,
                seed_limit
            );
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(in crate::search) fn render_search_callsite(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    Ok(render_search_callsite_from_contexts(
        project_root,
        query,
        options,
        &contexts,
    ))
}

fn render_search_callsite_from_contexts(
    project_root: &Path,
    query: &str,
    options: &RustSearchOptions,
    contexts: &[PackageSearchContext],
) -> String {
    let mut rendered = String::new();
    for context in contexts {
        let calls = symbol_calls(context, query, options);
        let mut block = format!(
            "[search-callsite] q={} pkg={} calls={}\n",
            query,
            package_label(project_root, &context.package_root),
            calls.len()
        );
        for hit in calls.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "|call {}", hit.render(&context.package_root));
        }
        append_block(&mut rendered, &block);
    }
    rendered
}

pub(in crate::search) fn render_search_import(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let hits = import_hits(&context, query, options);
        let mut block = format!(
            "[search-import] q={} pkg={} own={}\n",
            query,
            package_label(project_root, &context.package_root),
            hits.len()
        );
        for hit in hits.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|owner {} hit_kind=import locations={} next=owner:{}",
                display_project_path(&context.package_root, &hit.path),
                compact_locations(&hit.locations),
                display_project_path(&context.package_root, &hit.path)
            );
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(in crate::search) fn render_search_patterns() -> String {
    [
        "[search-patterns] n=4",
        "|pat public-anyhow-result lang=rust scope=src",
        "|pat public-error-boundary lang=rust scope=src",
        "|pat public-external-type lang=rust scope=src option=dependency",
        "|pat public-api-shape lang=rust scope=src option=owner",
    ]
    .join("\n")
        + "\n"
}

pub(in crate::search) fn render_search_pattern(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    if query == "public-anyhow-result" {
        return render_public_error_boundary_pattern(
            project_root,
            config,
            query,
            Some("anyhow::Result"),
            options,
        );
    }
    if query == "public-error-boundary" {
        return render_public_error_boundary_pattern(project_root, config, query, None, options);
    }
    if query == "public-external-type" {
        return render_public_external_type_pattern(project_root, config, options);
    }
    if query == "public-api-shape" {
        return render_public_api_shape_pattern(project_root, config, options);
    }
    Ok(format!(
        "[search-pattern] q={query} hits=0\n|note unknown_pattern=true\n"
    ))
}

fn render_public_error_boundary_pattern(
    project_root: &Path,
    config: &AspRustConfig,
    recipe: &str,
    boundary_filter: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let hits = public_error_boundary_hits(&context, boundary_filter, options);
        let mut block = format!(
            "[search-pattern] pattern={} pkg={} hit={} source=native-parser\n",
            recipe,
            package_label(project_root, &context.package_root),
            hits.len()
        );
        for hit in hits.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "{}",
                render_api_line(&context.package_root, &context.parsed_modules, &hit, false)
            );
        }
        if block.lines().count() == 1 {
            let _ = writeln!(
                block,
                "|note error_boundary_source=native-parser missing=true"
            );
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn public_error_boundary_hits(
    context: &PackageSearchContext,
    boundary_filter: Option<&str>,
    options: &RustSearchOptions,
) -> Vec<SearchHit> {
    let mut hits = context
        .parsed_modules
        .iter()
        .filter(|module| module_allowed(context, module, options))
        .flat_map(|module| {
            module
                .syntax_facts
                .public_function_returns
                .iter()
                .filter(move |return_fact| {
                    !return_fact.is_test_context
                        && return_fact
                            .application_error_boundary
                            .as_deref()
                            .is_some_and(|boundary| {
                                boundary_filter.is_none_or(|filter| boundary == filter)
                            })
                })
                .map(move |return_fact| SearchHit {
                    path: module.report.path.clone(),
                    line: return_fact.line,
                    kind: if return_fact.receiver.is_some() {
                        "method".to_string()
                    } else {
                        "fn".to_string()
                    },
                    name: return_fact.function_name.clone(),
                })
        })
        .collect::<Vec<_>>();
    sort_search_hits_by_recency(&context.package_root, &mut hits);
    hits.dedup_by(|left, right| {
        left.path == right.path
            && left.line == right.line
            && left.kind == right.kind
            && left.name == right.name
    });
    hits
}

fn render_public_external_type_pattern(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let Some(dependency) = options.dependency.as_deref() else {
        return Ok(
            "[search-pattern] pattern=public-external-type hits=0\n|note missing_dependency=true\n"
                .to_string(),
        );
    };
    let mut dependency_options = options.clone();
    if !dependency_options
        .pipes
        .iter()
        .any(|pipe| pipe == "public-api")
    {
        dependency_options.pipes.push("public-api".to_string());
    }
    let rendered = render_search_dependency(project_root, config, dependency, &dependency_options)?;
    Ok(rendered.replacen(
        "[search-dependency]",
        "[search-pattern] pattern=public-external-type",
        1,
    ))
}

fn render_public_api_shape_pattern(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let Some(owner) = options.owner.as_deref() else {
        return Ok(
            "[search-pattern] pattern=public-api-shape hits=0\n|note missing_owner=true\n"
                .to_string(),
        );
    };
    let mut owner_options = options.clone();
    if !owner_options.pipes.iter().any(|pipe| pipe == "items") {
        owner_options.pipes.push("items".to_string());
    }
    let rendered = render_search_owner(project_root, config, owner, &owner_options)?;
    Ok(rendered.replacen(
        "[search-owner]",
        "[search-pattern] pattern=public-api-shape",
        1,
    ))
}

pub(in crate::search) fn render_search_docs(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    Ok(render_search_docs_from_contexts(
        project_root,
        query,
        options,
        &contexts,
    ))
}

fn render_search_docs_from_contexts(
    project_root: &Path,
    query: &str,
    options: &RustSearchOptions,
    contexts: &[PackageSearchContext],
) -> String {
    let docs_query = ApiDocsQuery::parse(query);
    let mut rendered = String::new();
    for context in contexts {
        let source = docs_query.source(context);
        let defs = if source == "native-parser" {
            api_hits(
                &context.package_root,
                &context.parsed_modules,
                &symbol_definitions(context, &docs_query.item_name, options),
                &docs_query.item_name,
            )
        } else {
            Vec::new()
        };
        let mut block = docs_query.header_line(
            "search-docs",
            project_root,
            context,
            "docs",
            defs.len(),
            source,
        );
        for hit in defs.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "{}",
                render_api_line(&context.package_root, &context.parsed_modules, &hit, true)
            );
        }
        if block.lines().count() == 1 {
            let _ = writeln!(block, "|note docsSource={source} missing=true");
        }
        append_block(&mut rendered, &block);
    }
    rendered
}

pub(in crate::search) fn render_search_api(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let docs_query = ApiDocsQuery::parse(query);
    let mut rendered = String::new();
    for context in contexts {
        let source = docs_query.source(&context);
        let defs = if source == "native-parser" {
            api_hits(
                &context.package_root,
                &context.parsed_modules,
                &symbol_definitions(&context, &docs_query.item_name, options),
                &docs_query.item_name,
            )
        } else {
            Vec::new()
        };
        let mut block = docs_query.header_line(
            "search-api",
            project_root,
            &context,
            "api",
            defs.len(),
            source,
        );
        for hit in defs.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "{}",
                render_api_line(&context.package_root, &context.parsed_modules, &hit, false)
            );
        }
        if block.lines().count() == 1 {
            let _ = writeln!(block, "|note apiSource={source} missing=true");
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(in crate::search) fn render_search_docs_use(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let item_name = query.rsplit("::").next().unwrap_or(query);
    let contexts = search_contexts_for_path_query(project_root, config, options, item_name)?;
    let docs = render_search_docs_from_contexts(project_root, query, options, &contexts);
    let calls = render_search_callsite_from_contexts(project_root, item_name, options, &contexts);
    Ok(format!("{}{}", docs, calls))
}

struct ApiDocsQuery {
    raw: String,
    item_name: String,
    crate_name: Option<String>,
    requested_version: Option<String>,
}

impl ApiDocsQuery {
    fn parse(query: &str) -> Self {
        let item_name = query.rsplit("::").next().unwrap_or(query).to_string();
        let root = query.split("::").next().unwrap_or(query);
        let has_crate = query.contains("::") || root.contains('/') || root.contains('@');
        let (crate_path, requested_version) = root
            .rsplit_once('@')
            .map_or((root, None), |(crate_path, version)| {
                (crate_path, Some(version.to_string()))
            });
        let crate_name = has_crate
            .then(|| crate_path.split('/').next())
            .flatten()
            .filter(|crate_name| !crate_name.is_empty())
            .map(ToOwned::to_owned);
        Self {
            raw: query.to_string(),
            item_name,
            crate_name,
            requested_version,
        }
    }

    fn source(&self, context: &PackageSearchContext) -> &'static str {
        if self.crate_name.as_deref().is_some_and(|crate_name| {
            !matching_dependencies(&context.cargo_dependencies, crate_name).is_empty()
        }) {
            return "registry-source";
        }
        if let (Some(crate_name), Some(requested_version)) = (
            self.crate_name.as_deref(),
            self.requested_version.as_deref(),
        ) && !current_workspace_version_matches(context, crate_name, requested_version)
        {
            return "registry-source";
        }
        "native-parser"
    }

    fn header_line(
        &self,
        view: &str,
        project_root: &Path,
        context: &PackageSearchContext,
        count_label: &str,
        count: usize,
        source: &str,
    ) -> String {
        let mut block = format!(
            "[{view}] q={} pkg={} {count_label}={} source={}",
            self.raw,
            package_label(project_root, &context.package_root),
            count,
            source
        );
        if let Some(crate_name) = self.crate_name.as_deref() {
            let _ = write!(block, " crate={crate_name}");
        }
        if let Some(requested_version) = self.requested_version.as_deref() {
            let crate_name = self.crate_name.as_deref().unwrap_or("-");
            let version_scope =
                if current_workspace_version_matches(context, crate_name, requested_version) {
                    "current"
                } else {
                    "external"
                };
            let _ = write!(
                block,
                " requestedVersion={} versionScope={} currentWorkspaceVersion={}",
                requested_version,
                version_scope,
                current_workspace_versions(context, crate_name)
            );
        }
        block.push('\n');
        block
    }
}

fn current_workspace_version_matches(
    context: &PackageSearchContext,
    crate_name: &str,
    requested_version: &str,
) -> bool {
    matching_dependencies(&context.cargo_dependencies, crate_name)
        .iter()
        .any(|dependency| {
            version_requirement_matches_request(
                dependency.version_req.as_deref(),
                requested_version,
            )
        })
}

fn current_workspace_versions(context: &PackageSearchContext, crate_name: &str) -> String {
    let versions = matching_dependencies(&context.cargo_dependencies, crate_name)
        .into_iter()
        .filter_map(|dependency| dependency.version_req.as_deref())
        .collect::<BTreeSet<_>>();
    if versions.is_empty() {
        "-".to_string()
    } else {
        versions.into_iter().collect::<Vec<_>>().join(",")
    }
}

pub(in crate::search) fn render_search_public_external_types(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let dependencies = options.dependency.as_deref().map_or_else(
            || context.cargo_dependencies.iter().collect::<Vec<_>>(),
            |dependency| matching_dependencies(&context.cargo_dependencies, dependency),
        );
        let hits = public_external_type_hits(
            &context.package_root,
            &context.parsed_modules,
            &dependencies,
        );
        let mut block = format!(
            "[search-public-external-types] pkg={} dep={} hit={}\n",
            package_label(project_root, &context.package_root),
            dependencies.len(),
            hits.len()
        );
        for hit in hits.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{hit}");
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_api_line(
    package_root: &Path,
    parsed_modules: &[ParsedRustModule],
    hit: &SearchHit,
    include_docs: bool,
) -> String {
    let docs = if include_docs {
        " docs=local-parser"
    } else {
        ""
    };
    format!(
        "|api {} source=native-parser{}{}",
        hit.render(package_root),
        docs,
        api_fact_fields_for_hit(parsed_modules, hit)
    )
}

#[path = "api_hits.rs"]
mod api_hits;
use api_hits::{api_fact_fields_for_hit, api_hits, compact_api_value};

#[derive(Debug)]
struct PublicApiTypeSurface {
    path: PathBuf,
    line: usize,
    item: String,
    surface: String,
    type_text: String,
}

fn public_external_type_hits(
    package_root: &Path,
    parsed_modules: &[ParsedRustModule],
    dependencies: &[&CargoDependencyFacts],
) -> Vec<String> {
    let mut hits = BTreeSet::new();
    for module in parsed_modules {
        let surfaces = public_api_type_surfaces(module);
        for dependency in dependencies {
            let aliases = dependency_aliases(module, dependency);
            for surface in &surfaces {
                if type_text_mentions_any_alias(&surface.type_text, &aliases) {
                    hits.insert(format!(
                        "|external-type {}:{} dep={} surface={} item={} type={} source=native-parser next=dependency:{},docs:{}",
                        display_project_path(package_root, &surface.path),
                        surface.line,
                        dependency.dependency_key,
                        surface.surface,
                        surface.item,
                        compact_api_value(&surface.type_text),
                        dependency.dependency_key,
                        compact_api_value(&surface.type_text)
                    ));
                }
            }
        }
    }
    hits.into_iter().collect()
}

fn type_text_mentions_any_alias(type_text: &str, aliases: &BTreeSet<String>) -> bool {
    aliases
        .iter()
        .any(|alias| type_text_mentions_alias(type_text, alias))
}

fn public_api_type_surfaces(module: &ParsedRustModule) -> Vec<PublicApiTypeSurface> {
    let mut surfaces = Vec::new();
    surfaces.extend(
        module
            .syntax_facts
            .public_function_params
            .iter()
            .filter(|param| !param.is_test_context)
            .map(|param| PublicApiTypeSurface {
                path: module.report.path.clone(),
                line: param.line,
                item: param.function_name.clone(),
                surface: format!("param:{}", param.param_name),
                type_text: param.type_text.clone(),
            }),
    );
    surfaces.extend(
        module
            .syntax_facts
            .public_function_returns
            .iter()
            .filter(|return_fact| !return_fact.is_test_context)
            .map(|return_fact| PublicApiTypeSurface {
                path: module.report.path.clone(),
                line: return_fact.line,
                item: return_fact.function_name.clone(),
                surface: "return".to_string(),
                type_text: return_fact.type_text.clone(),
            }),
    );
    surfaces.extend(
        module
            .syntax_facts
            .public_struct_fields
            .iter()
            .filter(|field| !field.is_test_context)
            .map(|field| PublicApiTypeSurface {
                path: module.report.path.clone(),
                line: field.line,
                item: field.struct_name.clone(),
                surface: format!("field:{}", field.field_name),
                type_text: field.type_text.clone(),
            }),
    );
    surfaces.extend(
        module
            .syntax_facts
            .public_enum_variant_fields
            .iter()
            .filter(|field| !field.is_test_context)
            .map(|field| PublicApiTypeSurface {
                path: module.report.path.clone(),
                line: field.line,
                item: format!("{}::{}", field.enum_name, field.variant_name),
                surface: format!("field:{}", field.field_name),
                type_text: field.type_text.clone(),
            }),
    );
    surfaces.extend(
        module
            .syntax_facts
            .public_enum_tuple_variant_fields
            .iter()
            .filter(|field| !field.is_test_context)
            .map(|field| PublicApiTypeSurface {
                path: module.report.path.clone(),
                line: field.line,
                item: format!("{}::{}", field.enum_name, field.variant_name),
                surface: format!("tuple-field:{}", field.field_index),
                type_text: field.type_text.clone(),
            }),
    );
    surfaces.extend(
        module
            .syntax_facts
            .public_type_aliases
            .iter()
            .filter(|alias| !alias.is_test_context)
            .map(|alias| PublicApiTypeSurface {
                path: module.report.path.clone(),
                line: alias.line,
                item: alias.alias_name.clone(),
                surface: "alias".to_string(),
                type_text: alias.target_type_text.clone(),
            }),
    );
    surfaces
}

fn dependency_aliases(
    module: &ParsedRustModule,
    dependency: &CargoDependencyFacts,
) -> BTreeSet<String> {
    let mut aliases = BTreeSet::from([
        dependency.dependency_key.replace('-', "_"),
        dependency.import_name.clone(),
        dependency.package_name.replace('-', "_"),
    ]);
    for import in module
        .syntax_facts
        .use_statements
        .iter()
        .flat_map(|use_statement| &use_statement.imports)
    {
        let Some(root) = import.segments.first() else {
            continue;
        };
        if root == &dependency.import_name || root == &dependency.package_name.replace('-', "_") {
            if let Some(exposed_name) = &import.exposed_name {
                aliases.insert(exposed_name.clone());
            }
            if let Some(last_segment) = import.segments.last() {
                aliases.insert(last_segment.clone());
            }
        }
    }
    aliases
}

fn type_text_mentions_alias(type_text: &str, alias: &str) -> bool {
    type_text
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == alias)
}
