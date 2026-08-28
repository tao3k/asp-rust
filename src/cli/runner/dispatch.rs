//! CLI runner, argument parsing, and semantic protocol dispatch.

#[path = "exact_source/mod.rs"]
mod exact_source;
#[path = "owner_search.rs"]
mod owner_search;
#[path = "project_resolution.rs"]
mod project_resolution;
#[path = "provider_runtime.rs"]
mod provider_runtime;

use std::env;
#[cfg(feature = "search")]
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::{
    QueryCommand, discover_rust_project_root, is_command, is_known_search_view, is_search_pipe,
    parse_query, parse_usize_option, print_agent_doctor, print_agent_help, print_agent_registry,
    print_check_help, print_guide, print_help, print_query_guide, print_query_help,
    print_search_help, query_guide_kind, run_flow_lite_query_catalog, rust_package_root_for_path,
    rust_project_root_for_path, search_view_accepts_optional_query, search_view_requires_query,
    search_view_supports_query_set, split_csv_values,
};
#[cfg(feature = "search")]
use crate::cli::{
    SearchOutputControls, apply_search_output_controls, render_compact_graph_seed_packet,
};
#[cfg(feature = "search")]
use crate::cli::{SearchPlanOptions, render_search_plan};
#[cfg(feature = "search")]
use crate::cli::{SearchTraceOptions, render_search_trace};
#[cfg(feature = "search")]
use crate::cli::{SemanticSearchJsonOptions, render_search_json};
use crate::{
    RustHarnessRunScope, render_rust_project_harness, render_rust_project_harness_failure_frontier,
    render_rust_project_harness_json, run_rust_project_harness_for_scope,
    rust_harness_config_for_project,
};
#[cfg(feature = "search")]
use crate::{
    RustSearchOptions, RustSearchViewRequest, render_rust_project_harness_dependency_topology_json,
    render_rust_project_harness_dependency_topology_metadata_json,
    render_rust_project_harness_search_compare_json_with_config,
    render_rust_project_harness_search_ingest_with_config,
    render_rust_project_harness_search_semantic_facts_json,
    render_rust_project_harness_search_view_with_config,
};
use exact_source::run_exact_source_query;

/// Run the Rust harness CLI from process arguments and return its exit code.
pub fn run_cli_from_env() -> ExitCode {
    let argv = env::args_os().collect::<Vec<_>>();
    let log = crate::cli::DevCommandLog::start(&argv);
    let result = match run(argv.iter().skip(1).cloned()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    };
    log.finish(if result == ExitCode::SUCCESS { 0 } else { 2 });
    result
}

fn run(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if is_command(&args, "owner-search-stdin") {
        return owner_search::run_owner_search(&args);
    }
    if is_command(&args, "project-resolution-stdin") {
        return project_resolution::run_project_resolution(&args);
    }
    if is_command(&args, "serve") {
        return provider_runtime::run();
    }
    if is_command(&args, "search") {
        return run_search(args.into_iter().skip(1));
    }
    if is_command(&args, "query") {
        return run_query(args.into_iter().skip(1));
    }
    if is_command(&args, "projection") {
        return crate::cli::run_language_projection(args.into_iter().skip(1));
    }
    if is_command(&args, "check") {
        return run_check(args.into_iter().skip(1));
    }
    if is_command(&args, "behavior") {
        return crate::cli::run_behavior(args.into_iter().skip(1));
    }
    if is_command(&args, "determinism") {
        return crate::cli::run_determinism(args.into_iter().skip(1));
    }
    if is_command(&args, "receipt") {
        return crate::cli::run_receipt(args.into_iter().skip(1));
    }
    if is_command(&args, "ast-patch") {
        return crate::cli::run_ast_patch(args.into_iter().skip(1));
    }
    if is_command(&args, "proof") {
        return crate::cli::run_proof(args.into_iter().skip(1));
    }
    if is_command(&args, "review") {
        return crate::cli::run_review(args.into_iter().skip(1));
    }
    if is_command(&args, "evidence") {
        return crate::cli::run_evidence(args.into_iter().skip(1));
    }
    if is_command(&args, "guide") {
        let options = CliOptions::parse(args.into_iter().skip(1))?;
        if options.help {
            print_help();
            return Ok(ExitCode::SUCCESS);
        }
        print_guide(&options.target()?.root);
        return Ok(ExitCode::SUCCESS);
    }
    if is_command(&args, "agent") {
        return run_agent(args.into_iter().skip(1));
    }
    if args.len() <= 1 && args.first().is_some_and(|path| !is_option_like(path)) {
        let mut check_args = vec![std::ffi::OsString::from("--full")];
        check_args.extend(args);
        return run_check(check_args);
    }

    Err("expected an explicit asp-rust command; use `asp-rust check --changed|--full [PROJECT_ROOT]`"
        .to_string())
}

fn is_option_like(value: &std::ffi::OsStr) -> bool {
    value.to_str().is_some_and(|value| value.starts_with('-'))
}

fn run_check(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let options = CheckOptions::parse(args)?;
    if options.help {
        print_check_help();
        return Ok(ExitCode::SUCCESS);
    }
    let mode = options
        .mode
        .as_deref()
        .ok_or_else(|| "expected check mode: --changed or --full".to_string())?;
    let target = options.target()?;
    let report = run_rust_project_harness_for_scope(&target.root, target.scope)?;
    if options.json {
        println!(
            "{}",
            render_rust_project_harness_json(&report)
                .map_err(|error| format!("failed to render JSON report: {error}"))?
        );
    } else if mode == "changed" {
        let frontier = render_rust_project_harness_failure_frontier(&report, &target.root, 4);
        if frontier.is_empty() {
            print!("{}", render_rust_project_harness(&report));
        } else {
            print!("{frontier}");
        }
    } else {
        print!("{}", render_rust_project_harness(&report));
    }
    if report.is_clean() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn run_agent(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let options = AgentOptions::parse(args)?;
    if options.help {
        print_agent_help();
        return Ok(ExitCode::SUCCESS);
    }
    let project_root = options.project_root()?;
    if options.json {
        print_agent_registry(&project_root)?;
    } else {
        print_agent_doctor(&project_root, options.client.as_deref());
    }
    Ok(ExitCode::SUCCESS)
}

fn run_search(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let options = SearchOptions::parse(args)?;
    if options.help {
        print_search_help();
        return Ok(ExitCode::SUCCESS);
    }
    run_search_view(&options)
}

fn run_query(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<ExitCode, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if query_guide_kind(&args) {
        print_query_guide();
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(exit_code) = run_flow_lite_query_catalog(&args)? {
        return Ok(exit_code);
    }
    match parse_query(args)? {
        QueryCommand::Help => {
            print_query_help();
            Ok(ExitCode::SUCCESS)
        }
        QueryCommand::ExactSource(options) => run_exact_source_query(options),
        QueryCommand::TreeSitter(options) => {
            super::tree_sitter_query::run_tree_sitter_query(*options)
        }
    }
}

#[cfg(feature = "search")]
fn run_search_view(options: &SearchOptions) -> Result<ExitCode, String> {
    let project_root = options.project_root()?;
    let render_options = options.render_options();
    let config = rust_harness_config_for_project(&project_root);
    if options.view == "dependency-topology" {
        if !options.json {
            return Err("search dependency-topology requires --json".to_string());
        }
        print!(
            "{}",
            render_rust_project_harness_dependency_topology_json(&project_root)?
        );
        return Ok(ExitCode::SUCCESS);
    }
    if options.view == "dependency-topology-metadata" {
        if !options.json {
            return Err("search dependency-topology-metadata requires --json".to_string());
        }
        print!(
            "{}",
            render_rust_project_harness_dependency_topology_metadata_json(&project_root)?
        );
        return Ok(ExitCode::SUCCESS);
    }
    if options.view == "compare" && options.json {
        let query = options
            .query
            .as_deref()
            .ok_or_else(|| "search compare requires a query".to_string())?;
        println!(
            "{}",
            render_rust_project_harness_search_compare_json_with_config(
                &project_root,
                &config,
                query,
                &render_options,
            )?
        );
        return Ok(ExitCode::SUCCESS);
    }
    if options.view == "semantic-facts" {
        if !options.json {
            return Err("search semantic-facts requires --json".to_string());
        }
        let query = options
            .query
            .as_deref()
            .ok_or_else(|| "search semantic-facts requires a query".to_string())?;
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("failed to read search semantic-facts stdin: {error}"))?;
        print!(
            "{}",
            render_rust_project_harness_search_semantic_facts_json(&project_root, query, &input)?
        );
        return Ok(ExitCode::SUCCESS);
    }
    let raw_rendered = if options.view == "ingest" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("failed to read search ingest stdin: {error}"))?;
        render_rust_project_harness_search_ingest_with_config(
            &project_root,
            &config,
            &input,
            &render_options,
        )?
    } else {
        let request = RustSearchViewRequest {
            project_root: &project_root,
            config: &config,
            view: &options.view,
            query: options.query.as_deref(),
            options: &render_options,
        };
        render_rust_project_harness_search_view_with_config(&request)?
    };
    let json_options = options.semantic_json_options();
    let rendered = if options.json {
        String::new()
    } else if options.view == "compare" {
        raw_rendered.clone()
    } else if options.output_view.as_deref() == Some("seeds") {
        let packet_json = render_search_json(&project_root, &json_options, &raw_rendered)?;
        render_compact_graph_seed_packet(&packet_json, options.seeds)?
    } else {
        apply_search_output_controls(
            SearchOutputControls {
                depth: options.depth,
                output_view: options.output_view.as_deref(),
                packet_kind: None,
                seeds: options.seeds,
            },
            &raw_rendered,
        )
    };
    if options.json {
        println!(
            "{}",
            render_search_json(&project_root, &json_options, &raw_rendered)?
        );
    } else {
        if options.trace {
            let trace_options = options.search_trace_options();
            print!("{}", render_search_trace(&trace_options, &raw_rendered));
        }
        if options.explain {
            print!("{}", render_search_plan(options.search_plan_options()));
        }
        print!("{rendered}");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(feature = "search"))]
fn run_search_view(_options: &SearchOptions) -> Result<ExitCode, String> {
    Err("search command requires the `search` feature".to_string())
}

#[derive(Debug, Default)]
pub(super) struct CliOptions {
    pub(super) json: bool,
    pub(super) agent_snapshot: bool,
    pub(super) help: bool,
    pub(super) paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub(super) struct ResolvedCheckTarget {
    pub(super) root: PathBuf,
    pub(super) scope: RustHarnessRunScope,
}

#[derive(Debug, Default)]
struct CheckOptions {
    json: bool,
    help: bool,
    mode: Option<String>,
    paths: Vec<PathBuf>,
}

#[derive(Debug, Default)]
pub(super) struct AgentOptions {
    pub(super) client: Option<String>,
    pub(super) json: bool,
    pub(super) help: bool,
    pub(super) paths: Vec<PathBuf>,
}

#[derive(Debug, Default)]
pub(super) struct SearchOptions {
    pub(super) view: String,
    pub(super) query: Option<String>,
    pub(super) json: bool,
    pub(super) help: bool,
    pub(super) trace: bool,
    pub(super) explain: bool,
    pub(super) output_view: Option<String>,
    pub(super) depth: Option<usize>,
    pub(super) dir: Option<String>,
    pub(super) edges: Vec<String>,
    pub(super) per_owner: Option<usize>,
    pub(super) seeds: Option<usize>,
    pub(super) owners: Option<usize>,
    pub(super) hits: Option<usize>,
    pub(super) package: Option<String>,
    pub(super) owner: Option<String>,
    pub(super) dependency: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) lines: bool,
    pub(super) pipes: Vec<String>,
    pub(super) query_set: Vec<String>,
    pub(super) item_query: Option<String>,
    pub(super) item_projection_metadata: bool,
    pub(super) workspace_root: Option<PathBuf>,
}

#[cfg(test)]
#[path = "../../../tests/unit/cli/runner/dispatch.rs"]
mod tests;

fn resolve_search_workspace_root(
    cwd: &std::path::Path,
    input: &std::path::Path,
) -> Result<PathBuf, String> {
    if !cwd.is_absolute() {
        return Err(format!(
            "search workspace current directory must be absolute: cwd={}",
            cwd.display()
        ));
    }
    if input.as_os_str().is_empty() {
        return Err(format!(
            "search workspace root must not be empty: cwd={}",
            cwd.display()
        ));
    }

    let candidate = if input.is_absolute() {
        input.to_path_buf()
    } else if input == std::path::Path::new(".") || cwd.ends_with(input) {
        cwd.to_path_buf()
    } else {
        cwd.join(input)
    };
    let normalized = normalize_absolute_search_workspace_root(&candidate)?;
    if normalized.exists() {
        std::fs::canonicalize(&normalized).map_err(|error| {
            format!(
                "failed to canonicalize search workspace root input={} cwd={} resolved={}: {error}",
                input.display(),
                cwd.display(),
                normalized.display()
            )
        })
    } else {
        Ok(normalized)
    }
}

fn normalize_absolute_search_workspace_root(path: &std::path::Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!(
            "search workspace root must resolve to an absolute path: {}",
            path.display()
        ));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                normalized.push(std::path::Component::RootDir.as_os_str())
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    return Err(format!(
                        "search workspace root escapes its absolute root: {}",
                        path.display()
                    ));
                }
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    Ok(normalized)
}

impl SearchOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let args = args.into_iter();
        let mut options = Self::default();
        let mut positional_only = false;
        let mut pending_option: Option<String> = None;
        let mut positionals = Vec::<std::ffi::OsString>::new();
        for arg in args {
            if let Some(option) = pending_option.take() {
                let Some(value) = arg.to_str() else {
                    return Err(format!("expected UTF-8 value after {option}"));
                };
                match option.as_str() {
                    "--package" => options.package = Some(value.to_string()),
                    "--owner" => options.owner = Some(value.to_string()),
                    "--dependency" => options.dependency = Some(value.to_string()),
                    "--scope" => options.scope = Some(value.to_string()),
                    "--view" => options.output_view = Some(value.to_string()),
                    "--depth" => options.depth = Some(parse_usize_option(&option, value)?),
                    "--dir" => options.dir = Some(value.to_string()),
                    "--edge" => options.edges.extend(split_csv_values(value)),
                    "--per-owner" => {
                        options.per_owner = Some(parse_usize_option(&option, value)?);
                    }
                    "--seeds" => options.seeds = Some(parse_usize_option(&option, value)?),
                    "--owners" => options.owners = Some(parse_usize_option(&option, value)?),
                    "--hits" => options.hits = Some(parse_usize_option(&option, value)?),
                    "--query-set" => options.query_set.extend(split_csv_values(value)),
                    "--query" => options.item_query = Some(value.to_string()),
                    "--workspace" => {
                        if value.starts_with('-') {
                            return Err("--workspace requires a project root".to_string());
                        }
                        options.workspace_root = Some(PathBuf::from(value));
                    }
                    _ => unreachable!("known pending search option"),
                }
                continue;
            }
            if positional_only {
                positionals.push(arg);
                continue;
            }
            if !matches!(arg.to_str(), Some("--query" | "--query-set"))
                && options.query_set.is_empty()
                && options.item_query.is_none()
                && positionals.len() == 1
                && positionals
                    .first()
                    .and_then(|view| view.to_str())
                    .is_some_and(search_view_requires_query)
            {
                positionals.push(arg);
                continue;
            }
            let Some(value) = arg.to_str() else {
                positionals.push(arg);
                continue;
            };
            match value {
                "--" => positional_only = true,
                "--json" => options.json = true,
                "--help" | "-h" => options.help = true,
                "--lines" => options.lines = true,
                "--trace" => options.trace = true,
                "--explain" => options.explain = true,
                "--item-slice" => options.pipes.push("items".to_string()),
                "--package" | "--owner" | "--dependency" | "--scope" | "--view" | "--depth"
                | "--dir" | "--edge" | "--per-owner" | "--seeds" | "--owners" | "--hits"
                | "--query-set" | "--query" | "--workspace" => {
                    pending_option = Some(value.to_string())
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown search option: {value}"));
                }
                _ => positionals.push(arg),
            }
        }
        if let Some(option) = pending_option {
            return Err(format!("expected value after {option}"));
        }
        if options.help {
            return Ok(options);
        }
        options.apply_positionals(positionals)?;
        if options.view.is_empty() {
            return Err("expected search view: prime".to_string());
        }
        validate_search_options(&options)?;
        Ok(options)
    }

    fn apply_positionals(&mut self, positionals: Vec<std::ffi::OsString>) -> Result<(), String> {
        let mut values = positionals
            .into_iter()
            .map(|value| {
                value
                    .into_string()
                    .map_err(|_| "expected UTF-8 search arguments".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if values.is_empty() {
            return Ok(());
        }
        self.view = values.remove(0);
        if !is_known_search_view(&self.view) {
            return Ok(());
        }
        if search_view_requires_query(&self.view) {
            if !self.query_set.is_empty() {
                self.query = Some(self.query_set.join(","));
            } else if self.query.is_none() {
                if values.is_empty() {
                    return Err(format!("search {} requires a query", self.command_label()));
                }
                self.query = Some(values.remove(0));
            }
        } else if search_view_accepts_optional_query(&self.view)
            && values
                .first()
                .is_some_and(|value| !is_search_pipe(value) && !PathBuf::from(value).exists())
        {
            self.query = Some(values.remove(0));
        }
        for value in values {
            if is_search_pipe(&value) {
                self.pipes.push(value);
            } else {
                self.push_scope(value);
            }
        }
        Ok(())
    }

    fn push_scope(&mut self, value: String) {
        if let Some(scope) = &mut self.scope {
            scope.push(',');
            scope.push_str(&value);
        } else {
            self.scope = Some(value);
        }
    }

    pub(super) fn command_label(&self) -> String {
        self.view.clone()
    }

    #[cfg(feature = "search")]
    fn search_plan_options(&self) -> SearchPlanOptions<'_> {
        SearchPlanOptions {
            view: &self.view,
            query: self.query.as_deref(),
            output_view: self.output_view.as_deref(),
            depth: self.depth,
            dir: self.dir.as_deref(),
            edges: &self.edges,
            pipes: &self.pipes,
        }
    }

    #[cfg(feature = "search")]
    fn semantic_json_options(&self) -> SemanticSearchJsonOptions {
        SemanticSearchJsonOptions {
            view: self.view.clone(),
            query: self.query.clone(),
            command_label: self.command_label(),
            trace: self.trace,
            explain: self.explain,
            output_view: self.output_view.clone(),
            depth: self.depth,
            dir: self.dir.clone(),
            edges: self.edges.clone(),
            per_owner: self.per_owner,
            seeds: self.seeds,
            owners: self.owners,
            hits: self.hits,
            package: self.package.clone(),
            owner: self.owner.clone(),
            dependency: self.dependency.clone(),
            scope: self.scope.clone(),
            lines: self.lines,
            pipes: self.pipes.clone(),
            query_set: self.query_set(),
        }
    }

    #[cfg(feature = "search")]
    fn search_trace_options(&self) -> SearchTraceOptions {
        SearchTraceOptions {
            source: self.command_label(),
            query: self.query.clone(),
            pipes: self.pipes.clone(),
            output_view: self.output_view.clone(),
        }
    }

    #[cfg(feature = "search")]
    fn render_options(&self) -> RustSearchOptions {
        RustSearchOptions {
            package: self.package.clone(),
            owner: self.owner.clone(),
            dependency: self.dependency.clone(),
            scope: self.scope.clone(),
            pipes: self.pipes.clone(),
            lines: self.lines,
            output_view: self.output_view.clone(),
            seeds: self.seeds,
            query_set: self.query_set(),
            item_query: self.item_query.clone(),
            item_projection_metadata: self.item_projection_metadata,
        }
    }

    #[cfg(feature = "search")]
    fn project_root(&self) -> Result<PathBuf, String> {
        let cwd = std::env::current_dir()
            .map_err(|error| format!("failed to resolve provider current directory: {error}"))?;
        let root = match self.workspace_root.as_ref() {
            Some(path) => path.clone(),
            None => discover_rust_project_root()?,
        };
        resolve_search_workspace_root(&cwd, &root)
    }

    fn query_set(&self) -> Vec<String> {
        if !search_view_supports_query_set(&self.view) {
            return Vec::new();
        }
        if !self.query_set.is_empty() {
            return self.query_set.clone();
        }
        let Some(query) = self.query.as_deref() else {
            return Vec::new();
        };
        let terms = split_csv_values(query);
        if terms.len() > 1 { terms } else { Vec::new() }
    }
}

impl CheckOptions {
    fn target(&self) -> Result<ResolvedCheckTarget, String> {
        match self.paths.as_slice() {
            [path] => Ok(ResolvedCheckTarget {
                root: rust_package_root_for_path(path)?,
                scope: RustHarnessRunScope::Package,
            }),
            [] => {
                let current = env::current_dir()
                    .map_err(|error| format!("failed to read current dir: {error}"))?;
                Ok(ResolvedCheckTarget {
                    root: rust_project_root_for_path(&current)?,
                    scope: RustHarnessRunScope::ProjectWorkspace,
                })
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }

    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut positional_only = false;
        for arg in args {
            if positional_only {
                options.paths.push(PathBuf::from(arg));
                continue;
            }
            let Some(value) = arg.to_str() else {
                options.paths.push(PathBuf::from(arg));
                continue;
            };
            match value {
                "--" => positional_only = true,
                "--json" => options.json = true,
                "--help" | "-h" => options.help = true,
                "--changed" => options.set_mode("changed")?,
                "--full" => options.set_mode("full")?,
                value if value.starts_with('-') => {
                    return Err(format!("unknown check option: {value}"));
                }
                _ => options.paths.push(PathBuf::from(arg)),
            }
        }
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        Ok(options)
    }

    fn set_mode(&mut self, mode: &str) -> Result<(), String> {
        if self.mode.is_some() {
            return Err("expected only one check mode: --changed or --full".to_string());
        }
        self.mode = Some(mode.to_string());
        Ok(())
    }
}

fn validate_search_options(options: &SearchOptions) -> Result<(), String> {
    if !is_known_search_view(&options.view) {
        return Err(format!("unknown search view: {}", options.view));
    }
    if let Some(view) = options.output_view.as_deref()
        && !matches!(view, "graph" | "hits" | "both" | "seeds")
    {
        return Err(format!("unknown search --view mode: {view}"));
    }
    if let Some(dir) = options.dir.as_deref()
        && !matches!(dir, "out" | "in" | "both")
    {
        return Err(format!("unknown search --dir mode: {dir}"));
    }
    if !options.query_set.is_empty() && !search_view_supports_query_set(&options.view) {
        return Err(format!(
            "search {} does not support --query-set",
            options.command_label()
        ));
    }
    Ok(())
}
