use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use crate::AspRustConfig;
use crate::parser::{
    CargoDependencyFacts, parse_cargo_cfg_facts, parse_cargo_dependency_facts, parse_cargo_manifest,
};

use super::RustSearchOptions;
use super::context::{PackageSearchContext, search_contexts};
use super::format::{
    append_block, compact_locations, display_project_path, empty_dash, package_label,
    package_roots_for_request, render_cargo_dependency_line, sort_locations,
};
use super::hits::{
    OwnerHit, dependency_usage, matching_dependencies, sort_owner_hits_by_recency, text_hits,
};
use super::limits::SEARCH_HIT_LIMIT;
use super::owner::{public_api_lines_for_dependency, test_lines_for_owner_modules};
use super::scope::module_is_scope;
use super::version::version_requirement_matches_request;

pub(super) fn render_search_workspace(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let roots = package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = format!(
        "[search-workspace] root={} pkg={}\n",
        display_project_path(project_root, project_root),
        roots.len()
    );
    for package_root in roots.into_iter().take(SEARCH_HIT_LIMIT) {
        let _ = writeln!(
            rendered,
            "|package {} root={} manifest={} source=manifest manager=cargo next=package:{}",
            package_label(project_root, &package_root),
            display_project_path(project_root, &package_root),
            display_project_path(project_root, &package_root.join("Cargo.toml")),
            package_label(project_root, &package_root)
        );
    }
    Ok(rendered)
}

pub(super) fn render_search_targets(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let roots = package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = String::new();
    for package_root in roots {
        let manifest = parse_cargo_manifest(&package_root);
        let mut block = format!(
            "[search-targets] pkg={} source={} test={} bench={} example={}\n",
            package_label(project_root, &package_root),
            manifest.source_target_files.len(),
            manifest.test_target_files.len(),
            manifest.bench_targets.len(),
            manifest.example_targets.len()
        );
        for path in manifest
            .source_target_files
            .iter()
            .chain(manifest.test_target_files.iter())
            .chain(manifest.example_targets.iter().map(|target| &target.path))
            .take(SEARCH_HIT_LIMIT)
        {
            let _ = writeln!(
                block,
                "|target path={} source=manifest manager=cargo next=owner:{}",
                display_project_path(&package_root, path),
                display_project_path(&package_root, path)
            );
        }
        for bench in manifest.bench_targets.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|target bench:{} path={} harness={} required_features={} source=manifest manager=cargo",
                bench.name,
                display_project_path(&package_root, &bench.path),
                bench.harness,
                empty_dash(&bench.required_features)
            );
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(super) fn render_search_deps(
    project_root: &Path,
    config: &AspRustConfig,
    query: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    match query {
        Some(query) => render_search_dep(project_root, config, query, options),
        None => render_search_deps_list(project_root, config, options),
    }
}

fn render_search_deps_list(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = dependency_search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let mut block = format!(
            "[search-deps] pkg={} dep={}\n",
            package_label(project_root, &context.package_root),
            context.cargo_dependencies.len()
        );
        for dependency in context.cargo_dependencies.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_dep(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let parsed_query = DependencySearchQuery::parse(query);
    if !dependency_search_requires_full_context(&parsed_query, options) {
        return render_search_dep_fast(project_root, config, query, options, &parsed_query);
    }
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let deps = matching_dependencies(&context.cargo_dependencies, &parsed_query.dependency);
        let version_scope = parsed_query
            .requested_version
            .as_deref()
            .map(|version| dependency_version_scope(&deps, version))
            .unwrap_or(DependencyVersionScope::Current);
        let dependency_usage = if version_scope == DependencyVersionScope::Current {
            dependency_usage(&context, &parsed_query.dependency)
        } else {
            Vec::new()
        };
        let path_usage = parsed_query.subpath.as_deref().map_or_else(
            || dependency_usage.clone(),
            |subpath| {
                dependency_subpath_usage(
                    &context,
                    &dependency_usage,
                    &parsed_query.dependency,
                    subpath,
                )
            },
        );
        let usage = parsed_query.api.as_deref().map_or_else(
            || path_usage.clone(),
            |api| dependency_api_usage(&context, &path_usage, api),
        );
        let public_api = if version_scope == DependencyVersionScope::Current
            && has_pipe(options, "public-api")
        {
            public_api_lines_for_dependency(
                &context,
                &parsed_query.dependency,
                &usage,
                parsed_query.api.as_deref(),
            )
        } else {
            Vec::new()
        };
        if deps.is_empty()
            && dependency_usage.is_empty()
            && usage.is_empty()
            && public_api.is_empty()
        {
            append_block(
                &mut rendered,
                &render_missing_local_dependency_block(
                    project_root,
                    &context.package_root,
                    query,
                    &parsed_query,
                ),
            );
            continue;
        }
        let mut block = format!(
            "[search-deps] q={} pkg={} dep={} own={} api={}",
            query,
            package_label(project_root, &context.package_root),
            deps.len(),
            usage.len(),
            public_api.len()
        );
        if let Some(requested_version) = parsed_query.requested_version.as_deref() {
            let _ = write!(
                block,
                " requestedVersion={} currentWorkspaceVersion={} versionScope={}",
                requested_version,
                current_workspace_versions(&context.cargo_dependencies, &parsed_query.dependency),
                version_scope.label()
            );
        }
        if let Some(subpath) = parsed_query.subpath.as_deref() {
            let _ = write!(block, " subpath={subpath}");
        }
        if let Some(api_query) = parsed_query.api.as_deref() {
            let _ = write!(block, " apiQuery={api_query}");
        }
        block.push('\n');
        for dependency in deps.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        if let Some(guidance) =
            dependency_capability_guidance(&parsed_query.dependency, &dependency_usage)
        {
            let _ = writeln!(block, "{guidance}");
        }
        if version_scope == DependencyVersionScope::External {
            let _ = writeln!(
                block,
                "|note kind=version-scope message=requested-version-is-outside-current-workspace-version"
            );
        }
        for hit in usage.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|owner {} hit_kind={}{} locations={} next=owner:{}",
                display_project_path(&context.package_root, &hit.path),
                parsed_query.owner_hit_kind(),
                parsed_query.owner_metadata(),
                compact_locations(&hit.locations),
                display_project_path(&context.package_root, &hit.path)
            );
        }
        for line in public_api.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{line}");
        }
        let _ = writeln!(
            block,
            "|next {}",
            parsed_query.next_actions(version_scope, include_docs_use_next(options))
        );
        let _ = writeln!(block, "avoid=web-search,docs.rs-search,raw-read");
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

struct DependencySearchContext {
    package_root: PathBuf,
    cargo_dependencies: Vec<CargoDependencyFacts>,
}

fn dependency_search_contexts(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<Vec<DependencySearchContext>, String> {
    package_roots_for_request(project_root, config, options.package.as_deref()).map(|roots| {
        roots
            .into_iter()
            .map(|package_root| {
                let cargo_dependencies = parse_cargo_dependency_facts(&package_root);
                DependencySearchContext {
                    package_root,
                    cargo_dependencies,
                }
            })
            .collect()
    })
}

fn dependency_search_requires_full_context(
    parsed_query: &DependencySearchQuery,
    options: &RustSearchOptions,
) -> bool {
    parsed_query.subpath.is_some()
        || parsed_query.api.is_some()
        || options
            .pipes
            .iter()
            .any(|pipe| matches!(pipe.as_str(), "public-api" | "usage"))
}

fn render_search_dep_fast(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
    parsed_query: &DependencySearchQuery,
) -> Result<String, String> {
    let contexts = dependency_search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let deps = matching_dependencies(&context.cargo_dependencies, &parsed_query.dependency);
        let version_scope = parsed_query
            .requested_version
            .as_deref()
            .map(|version| dependency_version_scope(&deps, version))
            .unwrap_or(DependencyVersionScope::Current);
        if deps.is_empty() {
            append_block(
                &mut rendered,
                &render_missing_local_dependency_block(
                    project_root,
                    &context.package_root,
                    query,
                    parsed_query,
                ),
            );
            continue;
        }
        let mut block = format!(
            "[search-deps] q={} pkg={} dep={} own={} api={}",
            query,
            package_label(project_root, &context.package_root),
            deps.len(),
            0,
            0
        );
        if let Some(requested_version) = parsed_query.requested_version.as_deref() {
            let _ = write!(
                block,
                " requestedVersion={} currentWorkspaceVersion={} versionScope={}",
                requested_version,
                current_workspace_versions(&context.cargo_dependencies, &parsed_query.dependency),
                version_scope.label()
            );
        }
        block.push('\n');
        for dependency in deps.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        let _ = writeln!(
            block,
            "{}",
            dependency_manifest_topology_guidance(&parsed_query.dependency)
        );
        if version_scope == DependencyVersionScope::External {
            let _ = writeln!(
                block,
                "|note kind=version-scope message=requested-version-is-outside-current-workspace-version"
            );
        }
        let _ = writeln!(
            block,
            "|next {}",
            parsed_query.next_actions(version_scope, include_docs_use_next(options))
        );
        let _ = writeln!(block, "avoid=web-search,docs.rs-search,raw-read");
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_missing_local_dependency_block(
    project_root: &Path,
    package_root: &Path,
    query: &str,
    parsed_query: &DependencySearchQuery,
) -> String {
    let mut block = format!(
        "[search-deps] q={} pkg={} dep=0 own=0 api=0",
        query,
        package_label(project_root, package_root)
    );
    if let Some(requested_version) = parsed_query.requested_version.as_deref() {
        let _ = write!(
            block,
            " requestedVersion={} currentWorkspaceVersion=- versionScope=missing",
            requested_version
        );
    }
    if let Some(subpath) = parsed_query.subpath.as_deref() {
        let _ = write!(block, " subpath={subpath}");
    }
    if let Some(api_query) = parsed_query.api.as_deref() {
        let _ = write!(block, " apiQuery={api_query}");
    }
    block.push('\n');
    let _ = writeln!(
        block,
        "noOutput reason=no-local-dependency sourceTrace=cargo:manifest-empty,cargo:usage-empty"
    );
    let _ = writeln!(
        block,
        "nextCommand=asp rust search deps {} --workspace . --view seeds",
        shell_single_quote(query)
    );
    let _ = writeln!(
        block,
        "avoid=web-search,docs.rs-search,raw-read,external-doc-lookup"
    );
    block
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[derive(Debug)]
struct DependencySearchQuery {
    dependency: String,
    subpath: Option<String>,
    requested_version: Option<String>,
    api: Option<String>,
}

impl DependencySearchQuery {
    fn parse(query: &str) -> Self {
        let (dependency_and_version, api) = query
            .split_once("::")
            .map_or((query, None), |(dependency, api)| (dependency, Some(api)));
        let (dependency_path, requested_version) = dependency_and_version
            .split_once('@')
            .map_or((dependency_and_version, None), |(dependency, version)| {
                (dependency, Some(version))
            });
        let (dependency, subpath) = dependency_path
            .split_once('/')
            .map_or((dependency_path, None), |(dependency, subpath)| {
                (dependency, Some(subpath))
            });
        Self {
            dependency: dependency.trim().to_string(),
            subpath: subpath
                .map(str::trim)
                .filter(|subpath| !subpath.is_empty())
                .map(ToOwned::to_owned),
            requested_version: requested_version
                .map(str::trim)
                .filter(|version| !version.is_empty())
                .map(ToOwned::to_owned),
            api: api
                .map(str::trim)
                .filter(|api| !api.is_empty())
                .map(ToOwned::to_owned),
        }
    }

    fn next_actions(
        &self,
        version_scope: DependencyVersionScope,
        include_docs_use: bool,
    ) -> String {
        let docs_dependency = self.docs_dependency(version_scope);
        if let Some(api) = self.api.as_deref() {
            if !include_docs_use {
                return format!(
                    "dependency:{},crate-source:{},import:{},tests:{}",
                    self.dependency, self.dependency, self.dependency, api
                );
            }
            format!(
                "dependency:{},docs-use:{}::{},crate-source:{},import:{},tests:{}",
                self.dependency, docs_dependency, api, self.dependency, self.dependency, api
            )
        } else {
            if !include_docs_use {
                return format!(
                    "dependency:{},crate-source:{},import:{},tests",
                    self.dependency, self.dependency, self.dependency
                );
            }
            format!(
                "dependency:{},docs-use:{},crate-source:{},import:{},tests",
                self.dependency, docs_dependency, self.dependency, self.dependency
            )
        }
    }

    fn docs_dependency(&self, _version_scope: DependencyVersionScope) -> String {
        self.subpath.as_deref().map_or_else(
            || self.dependency.clone(),
            |subpath| format!("{}/{subpath}", self.dependency),
        )
    }

    fn owner_hit_kind(&self) -> &'static str {
        if self.api.is_some() {
            "dependency-api"
        } else if self.subpath.is_some() {
            "dependency-path"
        } else {
            "dependency"
        }
    }

    fn owner_metadata(&self) -> String {
        let mut metadata = String::new();
        if let Some(subpath) = self.subpath.as_deref() {
            let _ = write!(metadata, " subpath={subpath}");
        }
        if let Some(api) = self.api.as_deref() {
            let _ = write!(metadata, " apiQuery={api}");
        }
        metadata
    }
}

fn include_docs_use_next(options: &RustSearchOptions) -> bool {
    !options.pipes.iter().any(|pipe| pipe == "no-docs-use")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyVersionScope {
    Current,
    External,
}

impl DependencyVersionScope {
    fn label(self) -> &'static str {
        match self {
            DependencyVersionScope::Current => "current",
            DependencyVersionScope::External => "external",
        }
    }
}

fn dependency_version_scope(
    dependencies: &[&crate::parser::CargoDependencyFacts],
    requested_version: &str,
) -> DependencyVersionScope {
    if dependencies.iter().any(|dependency| {
        version_requirement_matches_request(dependency.version_req.as_deref(), requested_version)
    }) {
        DependencyVersionScope::Current
    } else {
        DependencyVersionScope::External
    }
}

fn current_workspace_versions(
    dependencies: &[crate::parser::CargoDependencyFacts],
    dependency: &str,
) -> String {
    let versions = matching_dependencies(dependencies, dependency)
        .into_iter()
        .filter_map(|dependency| dependency.version_req.as_deref())
        .collect::<Vec<_>>();
    if versions.is_empty() {
        "-".to_string()
    } else {
        versions.join(",")
    }
}

fn dependency_api_usage(
    context: &PackageSearchContext,
    dependency_usage: &[OwnerHit],
    api: &str,
) -> Vec<OwnerHit> {
    let dependency_owner_paths = dependency_usage
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<BTreeSet<_>>();
    dependency_index_hits(context, &dependency_owner_paths, api)
}

pub(super) fn dependency_capability_guidance(
    dependency: &str,
    dependency_usage: &[OwnerHit],
) -> Option<String> {
    if dependency_usage.is_empty() {
        return None;
    }
    Some(format!(
        "|dependency-topology dep={} usageLevel=local_usage topology=needs-index ownerUsage={} source=manifest,usage-index next=dependency-topology:{},crate-source:{},docs-use:{}",
        dependency,
        dependency_usage.len(),
        dependency,
        dependency,
        dependency
    ))
}

fn dependency_manifest_topology_guidance(dependency: &str) -> String {
    format!(
        "|dependency-topology dep={} usageLevel=manifest topology=asp-owned ownerUsage=0 source=manifest next=dependency-topology:{},crate-source:{},docs-use:{},import:{}",
        dependency, dependency, dependency, dependency, dependency
    )
}

fn dependency_subpath_usage(
    context: &PackageSearchContext,
    dependency_usage: &[OwnerHit],
    dependency: &str,
    subpath: &str,
) -> Vec<OwnerHit> {
    let dependency_owner_paths = dependency_usage
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<BTreeSet<_>>();
    let rust_path = format!("{dependency}::{}", subpath.replace('/', "::"));
    dependency_index_hits(context, &dependency_owner_paths, &rust_path)
}

fn dependency_index_hits(
    context: &PackageSearchContext,
    dependency_owner_paths: &BTreeSet<PathBuf>,
    symbol: &str,
) -> Vec<OwnerHit> {
    let mut hits = context
        .source_symbol_index
        .get(symbol)
        .map(|owners| {
            owners
                .iter()
                .filter(|(path, _)| dependency_owner_paths.contains(*path))
                .map(|(path, locations)| OwnerHit {
                    path: path.clone(),
                    locations: locations.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for hit in &mut hits {
        sort_locations(&mut hit.locations);
        hit.locations.dedup();
    }
    sort_owner_hits_by_recency(&context.package_root, &mut hits);
    hits
}

pub(super) fn render_search_features(
    project_root: &Path,
    config: &AspRustConfig,
    query: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    match query {
        Some(query) => render_search_feature(project_root, config, query, options),
        None => render_search_features_list(project_root, config, options),
    }
}

fn render_search_features_list(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let roots = package_roots_for_request(project_root, config, options.package.as_deref())?;
    let mut rendered = String::new();
    for package_root in roots {
        let features = manifest_features(&package_root);
        let mut block = format!(
            "[search-features] pkg={} feat={}\n",
            package_label(project_root, &package_root),
            features.len()
        );
        for (name, enables) in features.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|feature {} enables={} source=manifest manager=cargo",
                name,
                empty_dash(enables)
            );
        }
        if !features.is_empty() {
            let next = features
                .iter()
                .take(4)
                .map(|(name, _)| format!("features:{name}"))
                .collect::<Vec<_>>();
            let _ = writeln!(block, "|next {}", next.join(","));
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_feature(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let features = manifest_features(&context.package_root);
        let features_by_name = features
            .iter()
            .map(|(name, enables)| (name.as_str(), enables))
            .collect::<BTreeMap<_, _>>();
        let feature_present = features_by_name.contains_key(query);
        let enables = features_by_name
            .get(query)
            .map(|enables| (*enables).clone())
            .unwrap_or_default();
        let enabled_dependency_keys = enables
            .iter()
            .map(|enabled| enabled.strip_prefix("dep:").unwrap_or(enabled))
            .collect::<BTreeSet<_>>();
        let requested_feature = query.to_string();
        let feature_deps = context
            .cargo_dependencies
            .iter()
            .filter(|dependency| {
                enabled_dependency_keys.contains(dependency.dependency_key.as_str())
                    || dependency.features.contains(&requested_feature)
            })
            .collect::<Vec<_>>();
        let cfgs = if has_pipe(options, "cfg") {
            parse_cargo_cfg_facts(&context.package_root)
                .into_iter()
                .filter(|cfg| {
                    cfg.cfg == query
                        || cfg.cfg.strip_prefix("feature:") == Some(query)
                        || cfg.expression.contains(query)
                        || cfg.declared_in.contains(query)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let owner_hits = if has_pipe(options, "owners") {
            let mut owner_options = options.clone();
            owner_options.scope = Some("src".to_string());
            text_hits(&context, query, &owner_options)
        } else {
            Vec::new()
        };
        let owner_hit_paths = owner_hits
            .iter()
            .map(|hit| hit.path.clone())
            .collect::<BTreeSet<_>>();
        let owner_modules = context
            .parsed_modules
            .iter()
            .filter(|module| owner_hit_paths.contains(&module.report.path))
            .collect::<Vec<_>>();
        let mut test_lines = if has_pipe(options, "tests") {
            test_lines_for_owner_modules(&context, &owner_modules)
        } else {
            Vec::new()
        };
        if has_pipe(options, "tests") && test_lines.is_empty() {
            test_lines = feature_test_lines(&context, query);
        }
        let test_count = test_lines
            .iter()
            .filter(|line| line.starts_with("|test "))
            .count();
        let mut block = format!(
            "[search-features] q={} pkg={} feat={} dep={}",
            query,
            package_label(project_root, &context.package_root),
            usize::from(feature_present),
            feature_deps.len()
        );
        if has_pipe(options, "cfg") {
            let _ = write!(block, " cfg={}", cfgs.len());
        }
        if has_pipe(options, "owners") {
            let _ = write!(block, " own={}", owner_hits.len());
        }
        if has_pipe(options, "tests") {
            let _ = write!(block, " tests={test_count}");
        }
        block.push('\n');
        let _ = writeln!(
            block,
            "|feature {} enables={} source=manifest manager=cargo",
            query,
            empty_dash(&enables)
        );
        for dependency in feature_deps.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        for cfg in cfgs.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|cfg {} declared_in={} expr={} source=manifest manager=cargo next=text:{}(scope=src)",
                cfg.cfg, cfg.declared_in, cfg.expression, query
            );
        }
        for hit in owner_hits.into_iter().take(SEARCH_HIT_LIMIT) {
            let owner_path = display_project_path(&context.package_root, &hit.path);
            let _ = writeln!(
                block,
                "|owner {owner_path} hit_kind=feature locations={} next=owner:{owner_path}",
                compact_locations(&hit.locations)
            );
        }
        for line in test_lines.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{line}");
        }
        let _ = writeln!(block, "|next cfg:{query},text:{query}(scope=src),tests");
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(super) fn render_search_cfg(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let mut cfg_options = options.clone();
    cfg_options.scope.get_or_insert_with(|| "all".to_string());
    let contexts = search_contexts(project_root, config, &cfg_options)?;
    let mut rendered = String::new();
    for context in contexts {
        let cfgs = parse_cargo_cfg_facts(&context.package_root)
            .into_iter()
            .filter(|cfg| {
                cfg.cfg == query
                    || cfg.cfg.strip_prefix("feature:") == Some(query)
                    || cfg.expression.contains(query)
                    || cfg.declared_in.contains(query)
            })
            .collect::<Vec<_>>();
        let dependencies = parse_cargo_dependency_facts(&context.package_root)
            .into_iter()
            .filter(|dependency| {
                dependency
                    .target
                    .as_deref()
                    .is_some_and(|target| target.contains(query))
            })
            .collect::<Vec<_>>();
        let source_hits = text_hits(&context, query, &cfg_options);
        let mut block = format!(
            "[search-cfg] q={} pkg={} cfg={} dep={} own={}\n",
            query,
            package_label(project_root, &context.package_root),
            cfgs.len(),
            dependencies.len(),
            source_hits.len()
        );
        for cfg in cfgs.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|cfg {} declared_in={} expr={} source=manifest manager=cargo next=text:{}(scope=src)",
                cfg.cfg, cfg.declared_in, cfg.expression, query
            );
        }
        for dependency in dependencies.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(&dependency));
        }
        for hit in source_hits.into_iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|owner {} hit_kind=cfg locations={} next=owner:{}",
                display_project_path(&context.package_root, &hit.path),
                compact_locations(&hit.locations),
                display_project_path(&context.package_root, &hit.path)
            );
        }
        let _ = writeln!(
            block,
            "|next text:cfg({query})(scope=src),text:{query}(scope=tests)"
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn feature_test_lines(context: &PackageSearchContext, query: &str) -> Vec<String> {
    context
        .parsed_modules
        .iter()
        .filter(|module| module_is_scope(&context.scope, module, "tests"))
        .filter(|module| module.source.contains(query))
        .map(|module| {
            let path = display_project_path(&context.package_root, &module.report.path);
            format!(
                "|test {path} functions={} reason=feature:{query} next=owner:{path}",
                module.syntax_facts.test_function_count
            )
        })
        .collect()
}

fn has_pipe(options: &RustSearchOptions, pipe: &str) -> bool {
    options.pipes.iter().any(|candidate| candidate == pipe)
}

pub(super) fn manifest_features(package_root: &Path) -> Vec<(String, Vec<String>)> {
    let Ok(content) = fs::read_to_string(package_root.join("Cargo.toml")) else {
        return Vec::new();
    };
    let Ok(value) = content.parse::<toml::Table>() else {
        return Vec::new();
    };
    let Some(features) = value.get("features").and_then(toml::Value::as_table) else {
        return Vec::new();
    };
    let mut parsed = features
        .iter()
        .map(|(name, value)| {
            let enables = value
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(toml::Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            (name.clone(), enables)
        })
        .collect::<Vec<_>>();
    parsed.sort_by(|left, right| left.0.cmp(&right.0));
    parsed
}
