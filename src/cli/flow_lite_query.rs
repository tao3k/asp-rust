//! Native ASP flow-lite query catalog.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use super::query::print_query_help;

const FLOW_LITE_CATALOG_ID: &str = "flow-lite";
const FLOW_LITE_FLOW_KIND: &str = "local-source-sink";
const MAX_FLOW_HOT_PATH_SCAN_FILES: usize = 64;
const MAX_FLOW_SOURCE_FILES: usize = 1200;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlowLiteWhere {
    source_call: String,
    sink_constructs: String,
    scope_fn: String,
    owner_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlowLiteOccurrence {
    handle: String,
    kind: &'static str,
    value: String,
    path: String,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlowLiteResult {
    owner_path: String,
    function_start: usize,
    function_end: usize,
    source: Option<FlowLiteOccurrence>,
    sink: Option<FlowLiteOccurrence>,
    scanned_files: usize,
    reason: Option<&'static str>,
}

pub(super) fn run_flow_lite_query_catalog(args: &[OsString]) -> Result<Option<ExitCode>, String> {
    if !has_flow_lite_catalog(args)? {
        return Ok(None);
    }

    let mut catalog_id = None::<String>;
    let mut where_expr = None::<String>;
    let mut json_output = false;
    let mut workspace_root = None::<PathBuf>;
    let mut positionals = Vec::<PathBuf>::new();
    let mut pending_option = None::<String>;

    for arg in args {
        let value = arg
            .to_str()
            .ok_or_else(|| format!("query argument is not valid UTF-8: {arg:?}"))?;
        if let Some(option) = pending_option.take() {
            match option.as_str() {
                "--catalog" => {
                    catalog_id = Some(value.to_string());
                }
                "--where" => {
                    where_expr = Some(value.to_string());
                }
                "--workspace" => {
                    workspace_root = Some(PathBuf::from(value));
                }
                _ => unreachable!("unsupported pending flow-lite option: {option}"),
            }
            continue;
        }

        match value {
            "--catalog" | "--where" | "--workspace" => pending_option = Some(value.to_string()),
            "--json" => json_output = true,
            "--help" | "-h" => {
                print_query_help();
                return Ok(Some(ExitCode::SUCCESS));
            }
            option if option.starts_with('-') => {
                return Err(format!("unsupported flow-lite query option: {option}"));
            }
            other => positionals.push(PathBuf::from(other)),
        }
    }

    if let Some(option) = pending_option {
        return Err(format!("missing value for flow-lite query option {option}"));
    }
    if catalog_id.as_deref() != Some(FLOW_LITE_CATALOG_ID) {
        return Err("query flow-lite dispatch requires --catalog flow-lite".to_string());
    }

    let where_expr = where_expr
        .as_deref()
        .ok_or_else(|| "query --catalog flow-lite requires --where".to_string())?;
    let constraints = parse_flow_lite_where(where_expr)?;
    let project_root = match (workspace_root, positionals.as_slice()) {
        (Some(root), []) => root,
        (None, []) => PathBuf::from("."),
        (None, [root]) => root.clone(),
        (Some(_), [_]) => {
            return Err(
                "query accepts either --workspace <WORKSPACE> or one positional WORKSPACE, not both"
                    .to_string(),
            );
        }
        (_, _) => return Err("query accepts at most one positional WORKSPACE".to_string()),
    };
    let project_root = absolute_project_root(&project_root);
    let result = evaluate_flow_lite_query(&project_root, &constraints)?;
    if json_output {
        print_flow_lite_json(&project_root, &constraints, &result)?;
    } else {
        print_flow_lite_frontier(&project_root, &constraints, &result);
    }
    Ok(Some(ExitCode::SUCCESS))
}

fn has_flow_lite_catalog(args: &[OsString]) -> Result<bool, String> {
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg.to_str() == Some("--catalog") {
            let Some(value) = args.next() else {
                return Err("missing value for query catalog option --catalog".to_string());
            };
            return Ok(value.to_str() == Some(FLOW_LITE_CATALOG_ID));
        }
    }
    Ok(false)
}

fn parse_flow_lite_where(value: &str) -> Result<FlowLiteWhere, String> {
    let mut source_call = None::<String>;
    let mut sink_constructs = None::<String>;
    let mut scope_fn = None::<String>;
    let mut owner_path = None::<String>;
    for constraint in value.split_whitespace() {
        let (key, raw_value) = constraint
            .split_once('=')
            .ok_or_else(|| format!("invalid flow-lite --where constraint `{constraint}`"))?;
        if raw_value.trim().is_empty() {
            return Err(format!("flow-lite --where key `{key}` has an empty value"));
        }
        let target = match key {
            "source.call" => &mut source_call,
            "sink.constructs" => &mut sink_constructs,
            "scope.fn" => &mut scope_fn,
            "owner.path" => &mut owner_path,
            _ => {
                return Err(format!(
                    "unsupported flow-lite --where key `{key}`; supported keys are source.call,sink.constructs,scope.fn,owner.path"
                ));
            }
        };
        if target.is_some() {
            return Err(format!("duplicate flow-lite --where key `{key}`"));
        }
        *target = Some(raw_value.to_string());
    }

    Ok(FlowLiteWhere {
        source_call: source_call
            .ok_or_else(|| "flow-lite --where requires source.call".to_string())?,
        sink_constructs: sink_constructs
            .ok_or_else(|| "flow-lite --where requires sink.constructs".to_string())?,
        scope_fn: scope_fn.ok_or_else(|| "flow-lite --where requires scope.fn".to_string())?,
        owner_path,
    })
}

fn evaluate_flow_lite_query(
    project_root: &Path,
    constraints: &FlowLiteWhere,
) -> Result<FlowLiteResult, String> {
    let source_files = flow_lite_source_files(project_root, constraints)?;
    let scanned_files = source_files.len();
    if constraints.owner_path.is_none() && scanned_files > MAX_FLOW_HOT_PATH_SCAN_FILES {
        return Ok(FlowLiteResult {
            owner_path: ".".to_string(),
            function_start: 1,
            function_end: 1,
            source: None,
            sink: None,
            scanned_files,
            reason: Some("scope-not-narrowed"),
        });
    }
    let mut fallback = FlowLiteResult {
        owner_path: ".".to_string(),
        function_start: 1,
        function_end: 1,
        source: None,
        sink: None,
        scanned_files,
        reason: None,
    };
    for path in source_files {
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let syntax = match crate::parser::parse_rust_source_syntax(&source) {
            Ok(syntax) => syntax,
            Err(_) => continue,
        };
        let normalized_path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let relative_path = relative_project_path(project_root, &normalized_path);
        if let Some(result) = find_flow_lite_in_items(&syntax.items, &relative_path, constraints) {
            return Ok(FlowLiteResult {
                scanned_files,
                reason: None,
                ..result
            });
        }
        if fallback.owner_path == "." {
            fallback.owner_path = relative_path;
        }
    }
    Ok(fallback)
}

fn find_flow_lite_in_items(
    items: &[syn::Item],
    relative_path: &str,
    constraints: &FlowLiteWhere,
) -> Option<FlowLiteResult> {
    for item in items {
        match item {
            syn::Item::Fn(function) if function.sig.ident == constraints.scope_fn.as_str() => {
                let span = function.span();
                let mut collector = FunctionFlowLiteCollector {
                    constraints,
                    relative_path,
                    source: None,
                    sink: None,
                };
                collector.visit_block(&function.block);
                return Some(FlowLiteResult {
                    owner_path: relative_path.to_string(),
                    function_start: span.start().line.max(1),
                    function_end: span.end().line.max(span.start().line.max(1)),
                    source: collector.source,
                    sink: collector.sink,
                    scanned_files: 0,
                    reason: None,
                });
            }
            syn::Item::Mod(module) => {
                if let Some((_, nested_items)) = &module.content
                    && let Some(result) =
                        find_flow_lite_in_items(nested_items, relative_path, constraints)
                {
                    return Some(result);
                }
            }
            _ => {}
        }
    }
    None
}

struct FunctionFlowLiteCollector<'a> {
    constraints: &'a FlowLiteWhere,
    relative_path: &'a str,
    source: Option<FlowLiteOccurrence>,
    sink: Option<FlowLiteOccurrence>,
}

impl<'ast> Visit<'ast> for FunctionFlowLiteCollector<'_> {
    fn visit_expr_call(&mut self, expr_call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(expr_path) = expr_call.func.as_ref() {
            if self.source.is_none()
                && path_terminal_matches(&expr_path.path, &self.constraints.source_call)
            {
                self.source = Some(self.occurrence(
                    "call",
                    &self.constraints.source_call,
                    expr_call.span().start().line,
                ));
            }
            if self.sink.is_none()
                && path_contains(&expr_path.path, &self.constraints.sink_constructs)
            {
                self.sink = Some(self.occurrence(
                    "constructs",
                    &self.constraints.sink_constructs,
                    expr_call.span().start().line,
                ));
            }
        }
        visit::visit_expr_call(self, expr_call);
    }

    fn visit_expr_struct(&mut self, expr_struct: &'ast syn::ExprStruct) {
        if self.sink.is_none()
            && path_contains(&expr_struct.path, &self.constraints.sink_constructs)
        {
            self.sink = Some(self.occurrence(
                "constructs",
                &self.constraints.sink_constructs,
                expr_struct.span().start().line,
            ));
        }
        visit::visit_expr_struct(self, expr_struct);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.sink.is_none()
            && token_stream_contains_ident(mac.tokens.clone(), &self.constraints.sink_constructs)
        {
            self.sink = Some(self.occurrence(
                "constructs",
                &self.constraints.sink_constructs,
                mac.span().start().line,
            ));
        }
        visit::visit_macro(self, mac);
    }
}

impl FunctionFlowLiteCollector<'_> {
    fn occurrence(
        &self,
        handle_kind: &'static str,
        value: &str,
        line: usize,
    ) -> FlowLiteOccurrence {
        let line = line.max(1);
        FlowLiteOccurrence {
            handle: format!("{handle_kind}:{value}@{}:{}", self.relative_path, line),
            kind: handle_kind,
            value: value.to_string(),
            path: self.relative_path.to_string(),
            line,
        }
    }
}

fn print_flow_lite_frontier(
    project_root: &Path,
    constraints: &FlowLiteWhere,
    result: &FlowLiteResult,
) {
    println!(
        "[query-flow-lite] root={} lang=rust catalog=flow-lite flow={} scope=fn({}) alg=native-flow-lite",
        project_root.display(),
        FLOW_LITE_FLOW_KIND,
        constraints.scope_fn
    );
    println!("legend: ID=kind:role(value)!next; edge SRC>{{DST:rel}}; frontier ID.next");
    println!("aliases=G:query,F:flow,S:source,K:sink,P:path");
    println!();
    println!("F=flow:local-source-sink(fn:{})!flow", constraints.scope_fn);
    if let Some(source) = &result.source {
        println!(
            "S=source:{}({})@{}:{}!code",
            source.kind, source.value, source.path, source.line
        );
    }
    if let Some(sink) = &result.sink {
        println!(
            "K=sink:{}({})@{}:{}!code",
            sink.kind, sink.value, sink.path, sink.line
        );
    }
    if result.source.is_some() && result.sink.is_some() {
        println!("P=path:bounded(S->K)!flow");
    } else {
        println!("P=path:unavailable(fn:{})!flow", constraints.scope_fn);
    }
    println!();
    println!("G>{{F:selects}}");
    match (result.source.is_some(), result.sink.is_some()) {
        (true, true) => {
            println!("F>{{S:source,K:sink,P:flows-to}}");
            println!("S>{{K:flows-to}}");
        }
        (true, false) => println!("F>{{S:source,P:unavailable}}"),
        (false, true) => println!("F>{{K:sink,P:unavailable}}"),
        (false, false) => println!("F>{{P:unavailable}}"),
    }
    println!();
    println!(
        "confidence={} sourceAuthority=native-parser executionBackend=native-parser adapterMode=native-projection owner={} range={}:{} scannedFiles={}{}",
        flow_lite_confidence(result),
        result.owner_path,
        result.function_start,
        result.function_end,
        result.scanned_files,
        result
            .reason
            .map(|reason| format!(" reason={reason}"))
            .unwrap_or_default()
    );
    println!("rank={}", flow_lite_rank(result));
    println!("frontier={}", flow_lite_frontier(result));
    println!("omit=code,full-path-ast,raw-source");
    println!("avoid=codeql-hot-path,raw-read,inline-code");
}

fn print_flow_lite_json(
    project_root: &Path,
    constraints: &FlowLiteWhere,
    result: &FlowLiteResult,
) -> Result<(), String> {
    let source_handle = result
        .source
        .as_ref()
        .map(|source| source.handle.clone())
        .unwrap_or_else(|| format!("call:{}", constraints.source_call));
    let sink_handle = result
        .sink
        .as_ref()
        .map(|sink| sink.handle.clone())
        .unwrap_or_else(|| format!("constructs:{}", constraints.sink_constructs));
    let mut path = Vec::new();
    if let Some(source) = &result.source {
        path.push(flow_lite_path_step(
            "step.1",
            &source.handle,
            "source",
            source,
        ));
    }
    if let Some(sink) = &result.sink {
        path.push(flow_lite_path_step("step.2", &sink.handle, "sink", sink));
    }
    if let (Some(source), Some(sink)) = (&result.source, &result.sink) {
        path.push(serde_json::json!({
            "id": "step.3",
            "handle": sink.handle.as_str(),
            "relation": "flows-to",
            "location": {
                "path": sink.path.as_str(),
                "lineRange": line_range(sink.line),
            },
            "evidenceRefs": ["native-flow-lite.1"],
            "fields": {
                "from": source.handle.as_str(),
                "to": sink.handle.as_str(),
                "scopeFn": constraints.scope_fn.as_str()
            }
        }));
    }
    let omissions = flow_lite_omissions(constraints, result);
    let packet = serde_json::json!({
        "schemaId": "agent.semantic-protocols.semantic-flow-lite",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "languageId": "rust",
        "providerId": "asp-rust",
        "projectRoot": project_root.display().to_string(),
        "flowId": format!("flow-lite:{}:{}:{}:{}", result.owner_path, constraints.scope_fn, constraints.source_call, constraints.sink_constructs),
        "flowKind": FLOW_LITE_FLOW_KIND,
        "scope": "function",
        "ownerPath": result.owner_path.as_str(),
        "sourceAuthority": "native-parser",
        "executionBackend": "native-parser",
        "adapterMode": "native-projection",
        "sourceHandle": source_handle,
        "sinkHandle": sink_handle,
        "path": path,
        "guards": [],
        "effects": [],
        "artifacts": [],
        "confidence": flow_lite_confidence(result),
        "omissions": omissions,
        "fields": {
            "catalog": FLOW_LITE_CATALOG_ID,
            "where": {
                "source.call": constraints.source_call.as_str(),
                "sink.constructs": constraints.sink_constructs.as_str(),
                "scope.fn": constraints.scope_fn.as_str(),
                "owner.path": constraints.owner_path.as_deref(),
            },
            "scannedFiles": result.scanned_files as i64,
            "rawSourceStored": false
        }
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&packet)
            .map_err(|error| format!("failed to serialize semantic flow-lite packet: {error}"))?
    );
    Ok(())
}

fn flow_lite_path_step(
    id: &str,
    handle: &str,
    relation: &str,
    occurrence: &FlowLiteOccurrence,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "handle": handle,
        "relation": relation,
        "location": {
            "path": occurrence.path.as_str(),
            "lineRange": line_range(occurrence.line),
        },
        "evidenceRefs": ["native-flow-lite.1"],
        "fields": {
            "value": occurrence.value.as_str(),
            "kind": occurrence.kind
        }
    })
}

fn flow_lite_omissions(
    constraints: &FlowLiteWhere,
    result: &FlowLiteResult,
) -> Vec<serde_json::Value> {
    let mut omissions = Vec::new();
    if result.reason == Some("scope-not-narrowed") {
        omissions.push(serde_json::json!({
            "kind": "too-expensive",
            "message": format!("flow-lite scope.fn `{}` matched a large project without owner.path; narrow with search owner or add owner.path", constraints.scope_fn),
            "target": "owner.path"
        }));
        return omissions;
    }
    if result.owner_path == "." {
        omissions.push(serde_json::json!({
            "kind": "unavailable",
            "message": format!("scope.fn `{}` was not found", constraints.scope_fn),
            "target": "scope.fn"
        }));
        return omissions;
    }
    if result.source.is_none() {
        omissions.push(serde_json::json!({
            "kind": "unavailable",
            "message": format!("source.call `{}` was not found in scope.fn `{}`", constraints.source_call, constraints.scope_fn),
            "target": "source.call"
        }));
    }
    if result.sink.is_none() {
        omissions.push(serde_json::json!({
            "kind": "unavailable",
            "message": format!("sink.constructs `{}` was not found in scope.fn `{}`", constraints.sink_constructs, constraints.scope_fn),
            "target": "sink.constructs"
        }));
    }
    omissions
}

fn flow_lite_confidence(result: &FlowLiteResult) -> &'static str {
    if result.source.is_some() && result.sink.is_some() {
        "bounded"
    } else if result.source.is_some() || result.sink.is_some() {
        "partial"
    } else {
        "unavailable"
    }
}

fn flow_lite_rank(result: &FlowLiteResult) -> &'static str {
    match (result.source.is_some(), result.sink.is_some()) {
        (true, true) => "S,K,F",
        (true, false) => "S,F",
        (false, true) => "K,F",
        (false, false) => "F",
    }
}

fn flow_lite_frontier(result: &FlowLiteResult) -> &'static str {
    match (result.source.is_some(), result.sink.is_some()) {
        (true, true) => "S.code,K.code",
        (true, false) => "S.code",
        (false, true) => "K.code",
        (false, false) => "F.flow",
    }
}

fn path_terminal_matches(path: &syn::Path, expected: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == expected)
}

fn path_contains(path: &syn::Path, needle: &str) -> bool {
    path.segments.iter().any(|segment| segment.ident == needle)
}

fn token_stream_contains_ident(tokens: TokenStream, needle: &str) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == needle,
        TokenTree::Group(group) => token_stream_contains_ident(group.stream(), needle),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn line_range(line: usize) -> String {
    let line = line.max(1);
    format!("{line}:{line}")
}

fn flow_lite_source_files(
    project_root: &Path,
    constraints: &FlowLiteWhere,
) -> Result<Vec<PathBuf>, String> {
    let Some(owner_path) = constraints.owner_path.as_deref() else {
        return rust_source_files(project_root);
    };
    let path = Path::new(owner_path);
    let source_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    Ok(vec![source_path])
}

fn rust_source_files(project_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for root in rust_query_source_roots(project_root) {
        collect_rust_source_files(&root, &mut files)?;
    }
    files.sort();
    files.dedup();
    if files.len() > MAX_FLOW_SOURCE_FILES {
        files.truncate(MAX_FLOW_SOURCE_FILES);
    }
    Ok(files)
}

fn rust_query_source_roots(project_root: &Path) -> Vec<PathBuf> {
    if project_root.is_file() {
        return vec![project_root.to_path_buf()];
    }
    let mut roots = ["src", "tests", "benches", "examples"]
        .iter()
        .map(|name| project_root.join(name))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    let build_script = project_root.join("build.rs");
    if build_script.is_file() {
        roots.push(build_script);
    }
    roots.extend(workspace_member_source_roots(project_root));
    roots.sort();
    roots.dedup();
    if roots.is_empty() {
        roots.push(project_root.to_path_buf());
    }
    roots
}

fn workspace_member_source_roots(project_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for namespace in ["crates", "languages"] {
        let namespace_root = project_root.join(namespace);
        let Ok(entries) = fs::read_dir(&namespace_root) else {
            continue;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if should_skip_query_dir(&entry_path) {
                continue;
            }
            let src = entry_path.join("src");
            if src.is_dir() {
                roots.push(src);
            }
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn collect_rust_source_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|error| {
        format!(
            "failed to read flow-lite query project root {}: {error}",
            path.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read flow-lite query project entry under {}: {error}",
                path.display()
            )
        })?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if should_skip_query_dir(&entry_path) {
                continue;
            }
            collect_rust_source_files(&entry_path, files)?;
        } else if entry_path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("rs")
        {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn should_skip_query_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with('.')
        || matches!(
            name,
            "node_modules" | "target" | "vendor" | "dist" | "build" | "result"
        )
}

fn absolute_project_root(project_root: &Path) -> PathBuf {
    let absolute = if project_root.is_absolute() {
        project_root.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(project_root)
    };
    fs::canonicalize(&absolute).unwrap_or(absolute)
}

fn relative_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}
