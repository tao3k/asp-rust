//! Provider-facing `ast-patch` receipt rendering and Rust-native mutations.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::ops::Range;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

pub(super) const RECEIPT_SCHEMA_ID: &str = "agent.semantic-protocols.semantic-ast-patch-receipt";
pub(super) const AST_PATCH_PROTOCOL_ID: &str = "agent.semantic-protocols.ast-patch";
pub(super) const SUPPORTED_OPERATIONS: &[&str] = &["replace_item", "split_owner_items"];

pub(in crate::cli) fn run_ast_patch(
    args: impl Iterator<Item = OsString>,
) -> Result<ExitCode, String> {
    let args: Vec<String> = args
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "ast-patch arguments must be valid UTF-8".to_string())
        })
        .collect::<Result<_, _>>()?;
    let Some(mode) = args.first().map(String::as_str) else {
        return Err(
            "usage: asp-rust ast-patch <dry-run|apply> --packet <path|-> [PROJECT_ROOT]"
                .to_string(),
        );
    };
    if mode == "--help" || mode == "-h" {
        print_help();
        return Ok(ExitCode::SUCCESS);
    }
    let mode = AstPatchMode::parse(mode)?;
    let parsed = AstPatchArgs::parse(&args[1..])?;
    let packet_text = if parsed.packet_path == "-" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("failed to read ast-patch packet from stdin: {error}"))?;
        input
    } else {
        std::fs::read_to_string(&parsed.packet_path).map_err(|error| {
            format!(
                "failed to read ast-patch packet {}: {error}",
                parsed.packet_path
            )
        })?
    };
    let project_root = parsed.project_root.unwrap_or_else(|| PathBuf::from("."));
    let receipt = super::receipt::render_ast_patch_receipt(mode, &packet_text, &project_root);
    let quiet_success = matches!(mode, AstPatchMode::Apply)
        && serde_json::from_str::<Value>(&receipt)
            .ok()
            .and_then(|receipt| {
                receipt
                    .get("status")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .as_deref()
            == Some("applied");
    if !quiet_success {
        println!("{receipt}");
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Clone, Copy)]
pub(super) enum AstPatchMode {
    DryRun,
    Apply,
}

impl AstPatchMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "dry-run" => Ok(Self::DryRun),
            "apply" => Ok(Self::Apply),
            _ => Err("expected ast-patch <dry-run|apply>".to_string()),
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Apply => "apply",
        }
    }

    pub(super) fn capability(self) -> &'static str {
        match self {
            Self::DryRun => "provider-ast-dry-run",
            Self::Apply => "provider-ast-apply",
        }
    }

    pub(super) fn success_status(self) -> &'static str {
        match self {
            Self::DryRun => "verified",
            Self::Apply => "applied",
        }
    }

    pub(super) fn mechanical_plan_kind(self) -> &'static str {
        match self {
            Self::DryRun => "provider-dry-run",
            Self::Apply => "provider-apply",
        }
    }

    pub(super) fn mutation_available(self) -> bool {
        matches!(self, Self::Apply)
    }

    pub(super) fn writes_files(self) -> bool {
        matches!(self, Self::Apply)
    }
}

struct AstPatchArgs {
    packet_path: String,
    project_root: Option<PathBuf>,
}

impl AstPatchArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut packet_path = None;
        let mut positionals = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if arg == "--packet" {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--packet requires a path or -".to_string())?;
                if value.starts_with('-') && value != "-" {
                    return Err("--packet requires a path or -".to_string());
                }
                packet_path = Some(value.clone());
                index += 2;
                continue;
            }
            if arg.starts_with('-') {
                return Err(format!("unknown ast-patch option: {arg}"));
            }
            positionals.push(PathBuf::from(arg));
            index += 1;
        }
        if positionals.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        Ok(Self {
            packet_path: packet_path.ok_or_else(|| "--packet requires a path or -".to_string())?,
            project_root: positionals.pop(),
        })
    }
}

#[derive(Clone)]
pub(super) struct SourceRead {
    pub(super) raw: String,
    pub(super) path: String,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
}

impl SourceRead {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        let mut parts = value.rsplitn(3, ':');
        let end = parts
            .next()
            .ok_or_else(|| "target.read must be path:start:end".to_string())?;
        let start = parts
            .next()
            .ok_or_else(|| "target.read must be path:start:end".to_string())?;
        let path = parts
            .next()
            .ok_or_else(|| "target.read must be path:start:end".to_string())?;
        validate_project_path(path, "target.read path")?;
        let start_line = parse_line_number(start, "target.read start line")?;
        let end_line = parse_line_number(end, "target.read end line")?;
        if start_line > end_line {
            return Err("target.read start line must be before end line".to_string());
        }
        Ok(Self {
            raw: value.to_string(),
            path: path.to_string(),
            start_line,
            end_line,
        })
    }
}

fn parse_line_number(value: &str, label: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{label} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{label} must be a positive integer"));
    }
    Ok(parsed)
}

pub(super) fn validate_project_path(value: &str, label: &str) -> Result<(), String> {
    if value == "." {
        return Ok(());
    }
    if value.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(format!("{label} must be project-relative"));
    }
    if value.contains(':') || value.contains('\\') {
        return Err(format!("{label} contains unsupported path characters"));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(format!("{label} must not contain dot path segments")),
        }
    }
    Ok(())
}

pub(super) fn byte_range_for_line_range(
    source: &str,
    start_line: usize,
    end_line: usize,
) -> Result<Range<usize>, String> {
    if source.is_empty() {
        return Err("source file is empty".to_string());
    }
    let line_starts = std::iter::once(0)
        .chain(
            source
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        )
        .collect::<Vec<_>>();
    let available_lines = if source.ends_with('\n') {
        line_starts.len().saturating_sub(1)
    } else {
        line_starts.len()
    };
    if start_line > available_lines || end_line > available_lines {
        return Err(format!(
            "line range {start_line}:{end_line} is outside source with {available_lines} lines"
        ));
    }
    let start = line_starts[start_line - 1];
    let end = if end_line < line_starts.len() {
        line_starts[end_line]
    } else {
        source.len()
    };
    Ok(start..end)
}

pub(super) fn normalized_replacement(snippet: &str) -> String {
    let mut replacement = snippet.trim().to_string();
    replacement.push('\n');
    replacement
}

pub(super) fn validate_item_identity(
    target: &Value,
    existing: &syn::Item,
    replacement: &syn::Item,
) -> Result<(), String> {
    let existing_identity = item_identity(existing);
    let replacement_identity = item_identity(replacement);
    if let (Some(existing), Some(replacement)) = (&existing_identity, &replacement_identity)
        && existing != replacement
    {
        return Err(format!(
            "replacement item identity changed from {}:{} to {}:{}",
            existing.0, existing.1, replacement.0, replacement.1
        ));
    }
    if let Some(expected_name) = target.get("itemName").and_then(Value::as_str) {
        let Some((_, existing_name)) = existing_identity else {
            return Err("target.itemName was provided for an unnamed item".to_string());
        };
        if existing_name != expected_name {
            return Err(format!(
                "target.itemName {expected_name} does not match selected item {existing_name}"
            ));
        }
    }
    if let Some(expected_kind) = target.get("itemKind").and_then(Value::as_str) {
        let Some((existing_kind, _)) = item_identity(existing) else {
            return Err("target.itemKind was provided for an unnamed item".to_string());
        };
        if existing_kind != expected_kind {
            return Err(format!(
                "target.itemKind {expected_kind} does not match selected item kind {existing_kind}"
            ));
        }
    }
    Ok(())
}

pub(super) fn item_identity(item: &syn::Item) -> Option<(&'static str, String)> {
    match item {
        syn::Item::Const(item) => Some(("const", item.ident.to_string())),
        syn::Item::Enum(item) => Some(("enum", item.ident.to_string())),
        syn::Item::Fn(item) => Some(("fn", item.sig.ident.to_string())),
        syn::Item::Impl(item) => type_terminal_name(&item.self_ty).map(|name| ("impl", name)),
        syn::Item::Mod(item) => Some(("mod", item.ident.to_string())),
        syn::Item::Static(item) => Some(("static", item.ident.to_string())),
        syn::Item::Struct(item) => Some(("struct", item.ident.to_string())),
        syn::Item::Trait(item) => Some(("trait", item.ident.to_string())),
        syn::Item::TraitAlias(item) => Some(("trait_alias", item.ident.to_string())),
        syn::Item::Type(item) => Some(("type", item.ident.to_string())),
        syn::Item::Union(item) => Some(("union", item.ident.to_string())),
        _ => None,
    }
}

fn type_terminal_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Type::Reference(reference) => type_terminal_name(&reference.elem),
        syn::Type::Paren(paren) => type_terminal_name(&paren.elem),
        syn::Type::Group(group) => type_terminal_name(&group.elem),
        _ => None,
    }
}

pub(super) fn operation_max_edits(operation: &Value) -> u64 {
    operation
        .get("maxEdits")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

pub(super) fn operation_string_field<'a>(operation: &'a Value, key: &str) -> Option<&'a str> {
    operation
        .get("fields")
        .and_then(Value::as_object)
        .and_then(|fields| fields.get(key))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

pub(super) fn module_name_from_destination(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::to_string)
}

pub(super) fn is_rust_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub(super) fn parse_selected_rust_items(source: &str) -> Result<Vec<syn::Item>, String> {
    if let Ok(item) = syn::parse_str::<syn::Item>(source) {
        return Ok(vec![item]);
    }
    let file = crate::parser::parse_rust_source_syntax(source)
        .map_err(|error| format!("selected target did not parse as Rust items: {error}"))?;
    if file.items.is_empty() {
        return Err("selected target did not contain Rust items".to_string());
    }
    Ok(file.items)
}

pub(super) fn validate_selected_item_identity(
    target: &Value,
    items: &[syn::Item],
) -> Result<(), String> {
    let expected_name = target.get("itemName").and_then(Value::as_str);
    let expected_kind = target.get("itemKind").and_then(Value::as_str);
    if expected_name.is_none() && expected_kind.is_none() {
        return Ok(());
    }
    let matching = items
        .iter()
        .filter(|item| {
            let identity = item_identity(item);
            let name_matches = expected_name.is_none_or(|expected| {
                identity
                    .as_ref()
                    .is_some_and(|(_, actual_name)| actual_name == expected)
            });
            let kind_matches = expected_kind.is_none_or(|expected| {
                identity
                    .as_ref()
                    .is_some_and(|(actual_kind, _)| actual_kind == &expected)
            });
            name_matches && kind_matches
        })
        .count();
    if matching == 0 {
        return Err("selected items did not contain the expected target identity".to_string());
    }
    if items.len() == 1 && matching == 1 {
        return Ok(());
    }
    Ok(())
}

pub(super) fn normalized_split_destination_source(source: &str) -> String {
    let mut destination = source.trim().to_string();
    destination.push('\n');
    destination
}

pub(super) fn insert_module_declaration(source: &mut String, module_name: &str, visibility: &str) {
    if module_declaration_present(source, module_name) {
        return;
    }
    let declaration = if visibility.is_empty() {
        format!("mod {module_name};\n")
    } else {
        format!("{visibility} mod {module_name};\n")
    };
    let insertion_index = module_declaration_insert_index(source);
    source.insert_str(insertion_index, &declaration);
}

fn module_declaration_present(source: &str, module_name: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim();
        line == format!("mod {module_name};")
            || line == format!("pub mod {module_name};")
            || line == format!("pub(crate) mod {module_name};")
            || line == format!("pub(super) mod {module_name};")
    })
}

fn module_declaration_insert_index(source: &str) -> usize {
    source
        .split_inclusive('\n')
        .take_while(|line| {
            let trimmed = line.trim_start();
            trimmed.trim().is_empty() || trimmed.starts_with("//!") || trimmed.starts_with("#![")
        })
        .map(str::len)
        .sum()
}

pub(super) fn write_split_owner_files(
    source_path: &Path,
    owner_source: &str,
    destination_path: &Path,
    destination_source: &str,
) -> Result<(), String> {
    fs::write(destination_path, destination_source).map_err(|error| {
        format!(
            "failed to write destination {}: {error}",
            destination_path.display()
        )
    })?;
    if let Err(error) = fs::write(source_path, owner_source) {
        let remove_result = fs::remove_file(destination_path);
        let rollback = match remove_result {
            Ok(()) => "rolled back destination write".to_string(),
            Err(remove_error) => format!("failed to roll back destination: {remove_error}"),
        };
        return Err(format!(
            "failed to write owner {}; {rollback}: {error}",
            source_path.display()
        ));
    }
    Ok(())
}

pub(super) fn formatter_failure_kind(kind: &'static str) -> &'static str {
    match kind {
        "rustfmt-error" => "formatter-failed",
        "formatter-parse-error" => "formatter-output-reparse-failed",
        other => other,
    }
}

pub(super) fn format_candidate_source(
    source_path: &Path,
    next_source: &str,
) -> Result<String, (&'static str, String)> {
    let temp_path = candidate_format_path(source_path);
    if let Err(error) = fs::write(&temp_path, next_source) {
        return Err((
            "source-write-error",
            format!(
                "failed to write format candidate {}: {error}",
                temp_path.display()
            ),
        ));
    }

    let result = (|| {
        run_rustfmt(&temp_path).map_err(|error| ("rustfmt-error", error))?;
        let formatted = fs::read_to_string(&temp_path).map_err(|error| {
            (
                "source-read-error",
                format!(
                    "failed to read formatted candidate {}: {error}",
                    temp_path.display()
                ),
            )
        })?;
        crate::parser::parse_rust_source_syntax(&formatted).map_err(|error| {
            (
                "formatter-parse-error",
                format!("rustfmt output failed to parse: {error}"),
            )
        })?;
        Ok(formatted)
    })();

    let _ = fs::remove_file(&temp_path);
    result
}

fn candidate_format_path(source_path: &Path) -> PathBuf {
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("ast-patch.rs");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let temp_name = format!(".{file_name}.{}.{}.tmp.rs", std::process::id(), nonce);
    source_path.with_file_name(temp_name)
}

fn run_rustfmt(path: &Path) -> Result<(), String> {
    let output = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg("--config")
        .arg("skip_children=true")
        .arg(path)
        .output()
        .map_err(|error| format!("failed to run rustfmt: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "rustfmt failed with status {}; stdout: {}; stderr: {}",
        output.status,
        stdout.trim(),
        stderr.trim()
    ))
}

pub(super) fn is_schema_operation(value: &str) -> bool {
    matches!(
        value,
        "append_to_block"
            | "insert_before_statement"
            | "insert_after_statement"
            | "replace_statement"
            | "replace_expression"
            | "replace_call_arg"
            | "insert_import"
            | "remove_import"
            | "remove_statement"
            | "remove_item"
            | "replace_item"
            | "split_owner_items"
    )
}

fn print_help() {
    println!(
        "Usage: asp rust ast-patch <dry-run|apply> --packet <semantic-ast-patch.json|-> [project-root]"
    );
    println!();
    println!(
        "Emits a provider AST patch receipt. Rust native apply supports replace_item and split_owner_items."
    );
}
