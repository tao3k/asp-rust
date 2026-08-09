//! Public RFC search request and dispatch surface.

use std::path::Path;

use crate::RustHarnessConfig;

use super::{
    cargo, compare, density, dependency, format, guide, namespace, owner, owner_view, prime, query,
};

/// Options shared by RFC search renderers.
#[derive(Debug, Clone, Default)]
pub struct RustSearchOptions {
    /// Optional package selector for workspaces.
    pub package: Option<String>,
    /// Optional owner/path selector for hit-level views.
    pub owner: Option<String>,
    /// Optional dependency selector for dependency-aware pattern recipes.
    pub dependency: Option<String>,
    /// Optional scope selector such as `src`, `tests`, or `all`.
    pub scope: Option<String>,
    /// Optional pipe stages after the source view.
    pub pipes: Vec<String>,
    /// Whether source lines may be emitted.
    pub lines: bool,
    /// Optional compact output view requested by the CLI.
    pub output_view: Option<String>,
    /// Optional seed limit requested by the CLI.
    pub seeds: Option<usize>,
    /// Optional exact-set query terms for providers that can merge same-view probes.
    pub query_set: Vec<String>,
    /// Optional parser-native item query inside an owner/module result.
    pub item_query: Option<String>,
    /// Include parser projection metadata for schema packet rendering.
    pub item_projection_metadata: bool,
}

/// Request object for rendering an RFC search view.
#[derive(Debug, Clone, Copy)]
pub struct RustSearchViewRequest<'a> {
    /// Project root to inspect.
    pub project_root: &'a Path,
    /// Harness configuration used for discovery and parser policy.
    pub config: &'a RustHarnessConfig,
    /// Search source view such as `prime`, `dependency`, or `deps`.
    pub view: &'a str,
    /// Optional query consumed by views such as `symbol` or `deps`.
    pub query: Option<&'a str>,
    /// Shared search filters and pipe stages.
    pub options: &'a RustSearchOptions,
}

/// Renders the Rust search view through an explicit request object.
pub fn render_rust_project_harness_search_view_with_config(
    request: &RustSearchViewRequest<'_>,
) -> Result<String, String> {
    let rendered = render_search_view_packet(request)?;
    render_search_view_output(rendered, request.options)
}

fn render_search_view_packet(request: &RustSearchViewRequest<'_>) -> Result<String, String> {
    let project_root = request.project_root;
    let config = request.config;
    let options = request.options;
    match request.view {
        "guide" => Ok(guide::render_search_guide()),
        "prime" => prime::render_search_prime(
            project_root,
            config,
            options.package.as_deref(),
            prime_seed_limit(options),
        ),
        "workspace" => cargo::render_search_workspace(project_root, config, options),
        "targets" => cargo::render_search_targets(project_root, config, options),
        "deps" => cargo::render_search_deps(project_root, config, request.query, options),
        "features" => cargo::render_search_features(project_root, config, request.query, options),
        "policy" => super::policy::render_search_policy(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "code" => namespace::render_search_code(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "env" => namespace::render_search_env(project_root, config, request.query, options),
        "compare" => compare::render_search_compare(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "extension" => namespace::render_search_extension(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "owner" => owner_view::render_search_owner(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "dependency" => dependency::render_search_dependency(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "tests" => owner::render_search_tests(project_root, config, request.query, options),
        "symbol" => query::render_search_symbol(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "callsite" => query::render_search_callsite(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "import" => query::render_search_import(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "query" => super::syntax_query::render_search_query(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "cfg" => cargo::render_search_cfg(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "patterns" => Ok(query::render_search_patterns()),
        "pattern" => query::render_search_pattern(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "docs" => query::render_search_docs(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "docs-use" => query::render_search_docs_use(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "api" => query::render_search_api(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        "public-external-types" => {
            query::render_search_public_external_types(project_root, config, options)
        }
        "reasoning" => render_reasoning_profile(
            project_root,
            config,
            format::required_query(request.view, request.query)?,
            options,
        ),
        other => Err(format!("unknown search view: {other}")),
    }
}

fn render_search_view_output(
    rendered: String,
    options: &RustSearchOptions,
) -> Result<String, String> {
    if options.output_view.as_deref() == Some("seeds") {
        Ok(rendered)
    } else {
        Ok(density::render_terse_search_packet(&rendered))
    }
}

fn required_reasoning_selector<'a>(name: &str, value: Option<&'a str>) -> Result<&'a str, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("search reasoning requires --{name}"))
}

fn dependency_profile_query(query: &str, dependency: &str) -> String {
    if query == dependency || query.contains("::") {
        query.to_string()
    } else {
        format!("{dependency}::{query}")
    }
}

fn ensure_reasoning_pipe(options: &mut RustSearchOptions, pipe: &str) {
    if !options.pipes.iter().any(|existing| existing == pipe) {
        options.pipes.push(pipe.to_string());
    }
}

fn clone_reasoning_options(options: &RustSearchOptions) -> RustSearchOptions {
    RustSearchOptions {
        package: options.package.clone(),
        owner: options.owner.clone(),
        dependency: options.dependency.clone(),
        scope: options.scope.clone(),
        pipes: options.pipes.clone(),
        lines: options.lines,
        output_view: options.output_view.clone(),
        seeds: options.seeds,
        query_set: options.query_set.clone(),
        item_query: options.item_query.clone(),
        item_projection_metadata: options.item_projection_metadata,
    }
}

fn push_reasoning_body(rendered: &mut String, body: &str) {
    if !rendered.ends_with('\n') && !body.is_empty() {
        rendered.push('\n');
    }
    rendered.push_str(body);
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
}

fn encode_line_protocol_field(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '%' => escaped.push_str("%25"),
            character if character.is_whitespace() => escaped.push_str("%20"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn reasoning_block(
    profile: &str,
    selector: &str,
    algorithm: &str,
    extra_fields: &[(&str, &str)],
    body: &str,
) -> String {
    let extra_fields = extra_fields
        .iter()
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(name, value)| format!(" {name}={}", encode_line_protocol_field(value)))
        .collect::<String>();
    let selector = encode_line_protocol_field(selector);
    let mut rendered = format!(
        "[search-reasoning] q={profile} selector={selector} alg={algorithm}{extra_fields}\n",
    );

    let selector_action = match profile {
        "feature-cfg" => selector
            .strip_prefix("feature=")
            .map(|value| ("feature", value)),
        "finding-frontier" => selector
            .strip_prefix("finding=")
            .map(|value| ("finding", value)),
        _ => None,
    };
    if let Some((action_kind, value)) = selector_action {
        rendered.push_str(&format!("|next {action_kind}:{value}\n"));
    }

    let profile_entry = match profile {
        "owner-query" => "entries=owner-query(O,Q=>items+tests+dependency-usage)",
        "query-deps" => {
            "entries=query-deps(Q,D=>owners+imports+local-docs+crate-source+usage-tests)"
        }
        "owner-tests" => "entries=owner-tests(O=>covering-tests+test-entrypoints+fixtures)",
        "finding-frontier" => {
            "entries=finding-frontier(F,O=>affected-owners+tests+verification-actions)"
        }
        "feature-cfg" => "entries=feature-cfg(F=>cfg-gates+owners+verification-surfaces)",
        _ => "",
    };
    if !profile_entry.is_empty() {
        rendered.push_str(profile_entry);
        rendered.push('\n');
    }
    push_reasoning_body(&mut rendered, body);
    if profile == "query-deps" {
        rendered.push_str("avoid=web-search,docs.rs-search,raw-read\n");
    } else {
        rendered.push_str("avoid=raw-read\n");
    }
    rendered
}

fn render_reasoning_profile(
    project_root: &Path,
    config: &RustHarnessConfig,
    profile: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    match profile {
        "owner-tests" => {
            let owner = required_reasoning_selector("owner", options.owner.as_deref())?;
            let mut query_reasoning_options = clone_reasoning_options(options);
            ensure_reasoning_pipe(&mut query_reasoning_options, "tests");
            let body = owner_view::render_search_owner(
                project_root,
                config,
                owner,
                &query_reasoning_options,
            )?;
            Ok(reasoning_block(
                "owner-tests",
                &format!("owner={owner}"),
                "owner-test-frontier",
                &[],
                &body,
            ))
        }
        "owner-query" => {
            let owner = required_reasoning_selector("owner", options.owner.as_deref())?;
            let query = required_reasoning_selector("query", options.item_query.as_deref())?;
            let mut reasoning_search_options = clone_reasoning_options(options);
            reasoning_search_options.item_query = Some(query.to_string());
            ensure_reasoning_pipe(&mut reasoning_search_options, "items");
            ensure_reasoning_pipe(&mut reasoning_search_options, "tests");
            let mut body = owner_view::render_search_owner(
                project_root,
                config,
                owner,
                &reasoning_search_options,
            )?;
            if let Some(dependency) = options.dependency.as_deref() {
                let dep_query = dependency_profile_query(query, dependency);
                let mut dep_options = clone_reasoning_options(options);
                ensure_reasoning_pipe(&mut dep_options, "public-api");
                ensure_reasoning_pipe(&mut dep_options, "tests");
                ensure_reasoning_pipe(&mut dep_options, "no-docs-use");
                let dep_body = cargo::render_search_deps(
                    project_root,
                    config,
                    Some(&dep_query),
                    &dep_options,
                )?;
                push_reasoning_body(&mut body, &dep_body);
            }
            Ok(reasoning_block(
                "owner-query",
                &format!("owner={owner}"),
                "owner-query-frontier",
                &[("query", query)],
                &body,
            ))
        }
        "query-deps" => {
            let query = required_reasoning_selector("query", options.item_query.as_deref())?;
            let dependency =
                required_reasoning_selector("dependency", options.dependency.as_deref())?;
            let dep_query = dependency_profile_query(query, dependency);
            let mut reasoning_options = clone_reasoning_options(options);
            ensure_reasoning_pipe(&mut reasoning_options, "public-api");
            ensure_reasoning_pipe(&mut reasoning_options, "tests");
            ensure_reasoning_pipe(&mut reasoning_options, "no-docs-use");
            let body = cargo::render_search_deps(
                project_root,
                config,
                Some(&dep_query),
                &reasoning_options,
            )?;
            Ok(reasoning_block(
                "query-deps",
                &format!("query={query}"),
                "query-dependency-frontier",
                &[("query", query), ("dependency", dependency)],
                &body,
            ))
        }
        "feature-cfg" => {
            let feature = required_reasoning_selector("query", options.item_query.as_deref())?;
            let mut cfg_options = clone_reasoning_options(options);
            ensure_reasoning_pipe(&mut cfg_options, "owner");
            let body = cargo::render_search_cfg(project_root, config, feature, &cfg_options)?;
            Ok(reasoning_block(
                "feature-cfg",
                &format!("feature={feature}"),
                "feature-cfg-frontier",
                &[("query", feature)],
                &body,
            ))
        }
        "finding-frontier" => {
            let finding = required_reasoning_selector("query", options.item_query.as_deref())?;
            let mut body = String::new();
            let symbol_body = query::render_search_symbol(project_root, config, finding, options)?;
            push_reasoning_body(&mut body, &symbol_body);
            let api_body = query::render_search_api(project_root, config, finding, options)?;
            push_reasoning_body(&mut body, &api_body);
            if let Some(owner) = options
                .owner
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                let mut owner_options = clone_reasoning_options(options);
                owner_options.item_query = Some(finding.to_string());
                ensure_reasoning_pipe(&mut owner_options, "items");
                ensure_reasoning_pipe(&mut owner_options, "tests");
                let owner_body =
                    owner_view::render_search_owner(project_root, config, owner, &owner_options)?;
                push_reasoning_body(&mut body, &owner_body);
            }
            Ok(reasoning_block(
                "finding-frontier",
                &format!("finding={finding}"),
                "finding-frontier",
                &[
                    ("query", finding),
                    ("owner", options.owner.as_deref().unwrap_or("")),
                ],
                &body,
            ))
        }
        other => Err(format!("unknown reasoning profile '{other}'")),
    }
}

fn prime_seed_limit(options: &RustSearchOptions) -> Option<usize> {
    (options.output_view.as_deref() == Some("seeds")).then_some(options.seeds.unwrap_or(8))
}
