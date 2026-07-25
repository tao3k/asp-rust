use std::path::{Component, Path};
use std::process::ExitCode;

use serde_json::{Value, json};

use crate::cli::query::TreeSitterQuery;
use crate::parser::parse_rust_file;

pub(super) fn run_tree_sitter_query(options: TreeSitterQuery) -> Result<ExitCode, String> {
    let selector = options.selector.as_deref().ok_or_else(|| {
        if options.code {
            "tree-sitter query --code requires an exact --selector".to_string()
        } else {
            "tree-sitter query requires an exact --selector".to_string()
        }
    })?;
    let owner_path = exact_owner_path(selector)?;
    let source_path = options.workspace_root.join(owner_path);
    if !source_path.is_file() {
        return Err(format!(
            "tree-sitter query selector does not resolve to a source file: {selector}"
        ));
    }
    let query_source = tree_sitter_query_source(&options)?;
    let plan = native_function_query_plan(&options, query_source)?;
    let parsed = parse_rust_file(&source_path);
    if !parsed.report.is_valid {
        return Err(format!(
            "tree-sitter native projection failed to parse {owner_path}: {}",
            parsed
                .report
                .parse_error
                .as_deref()
                .unwrap_or("unknown parser error")
        ));
    }
    let matches = parsed
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| matches!(item.kind, "fn" | "function"))
        .filter(|item| {
            plan.name
                .as_deref()
                .is_none_or(|expected| item.name.as_deref() == Some(expected))
        })
        .collect::<Vec<_>>();

    if options.code {
        let [item] = matches.as_slice() else {
            return Err(format!(
                "tree-sitter query --code requires one unique match; matches={}",
                matches.len()
            ));
        };
        let rendered = source_line_window(&parsed.source, item.line, item.end_line);
        if rendered.is_empty() {
            return Err("tree-sitter query selected an empty source window".to_string());
        }
        if options.json {
            let provider_id = options
                .provider_id
                .as_deref()
                .ok_or_else(|| "missing typed provider identity".to_string())?;
            let parser_identity_digest = canonical_digest_argument(
                options.parser_identity_digest.as_deref(),
                "parser identity",
            )?;
            let query_pack_digest = canonical_digest_argument(
                options.query_pack_digest.as_deref(),
                "query-pack identity",
            )?;
            let normalized_parser_facts = serde_json::to_vec(&json!({
                "kind": item.kind,
                "name": item.name,
                "ownerPath": owner_path,
                "selector": selector,
                "startLine": item.line,
                "endLine": item.end_line,
            }))
            .map_err(|error| format!("failed to encode normalized parser facts: {error}"))?;
            let parser_identity_digest =
                crate::semantic_identity::exact_selector_merkle::parse_content_digest_v1(
                    parser_identity_digest,
                )?;
            let query_pack_digest =
                crate::semantic_identity::exact_selector_merkle::parse_content_digest_v1(
                    query_pack_digest,
                )?;
            let packet_language_id =
                crate::semantic_identity::exact_selector_projection_packet::ProjectionPacketLanguageIdV1::from(
                    "rust",
                );
            let packet_provider_id =
                crate::semantic_identity::exact_selector_projection_packet::ProjectionPacketProviderIdV1::from(
                    provider_id,
                );
            let packet_owner_path =
                crate::semantic_identity::exact_selector_projection_packet::ProjectionPacketOwnerPathV1::from(
                    owner_path,
                );
            let packet_structural_selector =
                crate::semantic_identity::exact_selector_projection_packet::ProjectionPacketStructuralSelectorV1::from(
                    selector,
                );
            let packet = crate::semantic_identity::exact_selector_projection_packet::build_exact_selector_projection_packet_v1(
                crate::semantic_identity::exact_selector_projection_packet::ExactSelectorProjectionPacketV1Input {
                    language_id: &packet_language_id,
                    provider_id: &packet_provider_id,
                    canonical_item_selector: crate::semantic_identity::canonical_item_identity::CanonicalItemSelectorV1::new(
                        crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
                            "rust",
                            "syntax-query",
                            selector,
                        ),
                        selector,
                    ),
                    parser_identity_digest: &parser_identity_digest,
                    query_pack_digest: &query_pack_digest,
                    owner_path: &packet_owner_path,
                    structural_selector: &packet_structural_selector,
                    projection_mode: crate::semantic_identity::exact_selector_merkle::ExactProjectionModeV1::Code,
                    source: parsed.source.as_bytes(),
                    normalized_parser_facts: &normalized_parser_facts,
                    projection: rendered.as_bytes(),
                },
            );
            println!(
                "{}",
                serde_json::to_string(&packet).map_err(|error| {
                    format!("failed to render exact-selector projection packet: {error}")
                })?
            );
            return Ok(ExitCode::SUCCESS);
        }
        print!("{rendered}");
        return Ok(ExitCode::SUCCESS);
    }

    let native_fact_refs = matches
        .iter()
        .map(|item| {
            let name = item.name.as_deref().unwrap_or("<anonymous>");
            format!(
                "rust:item:{owner_path}:{}:{}:{}",
                item.line, item.end_line, name
            )
        })
        .collect::<Vec<_>>();
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schemaId": "agent.semantic-protocols.semantic-tree-sitter-query",
                "schemaVersion": "1",
                "adapterMode": "native-projection",
                "compatibilityLevel": "native-only",
                "selector": owner_path,
                "nativeFactRefs": native_fact_refs,
                "matchCount": matches.len(),
                "cache": { "rawSourceStored": false }
            }))
            .map_err(|error| format!("failed to render tree-sitter query JSON: {error}"))?
        );
        return Ok(ExitCode::SUCCESS);
    }

    println!(
        "[query-treesitter] frontier=I.code omit=code,full-node-list,capture-text ts=identifier/name matches={}",
        matches.len()
    );
    for item in matches {
        let name = item.name.as_deref().unwrap_or("<anonymous>");
        println!(
            "I=item:fn({})@{}:{}:{}!code nativeFactRef=rust:item:{}:{}:{}:{}",
            name, owner_path, item.line, item.end_line, owner_path, item.line, item.end_line, name
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn canonical_digest_argument<'a>(digest: Option<&'a str>, label: &str) -> Result<&'a str, String> {
    let digest = digest.ok_or_else(|| format!("missing {label} digest"))?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(format!("invalid {label} digest"));
    }
    Ok(digest)
}

struct NativeFunctionQueryPlan {
    name: Option<String>,
}

fn native_function_query_plan(
    options: &TreeSitterQuery,
    query_source: &str,
) -> Result<NativeFunctionQueryPlan, String> {
    let node_types = if options.node_types.is_empty() {
        query_source
            .contains("(function_item")
            .then_some(vec!["function_item".to_string()])
            .unwrap_or_default()
    } else {
        options.node_types.clone()
    };
    let captures = if options.captures.is_empty() {
        query_source
            .contains("@function.name")
            .then_some(vec!["function.name".to_string()])
            .unwrap_or_default()
    } else {
        options.captures.clone()
    };
    let fields = if options.fields.is_empty() {
        query_source
            .contains("name:")
            .then_some(vec!["name".to_string()])
            .unwrap_or_default()
    } else {
        options.fields.clone()
    };
    if !node_types.iter().any(|value| value == "function_item")
        || !captures.iter().any(|value| value == "function.name")
        || !fields.iter().any(|value| value == "name")
    {
        return Err(
            "unsupported Rust tree-sitter native projection; expected function_item name/function.name"
                .to_string(),
        );
    }
    let name = options
        .predicates_json
        .as_deref()
        .map(predicate_function_name)
        .transpose()?
        .flatten()
        .or_else(|| inline_eq_predicate(query_source));
    Ok(NativeFunctionQueryPlan { name })
}

fn predicate_function_name(source: &str) -> Result<Option<String>, String> {
    let predicates: Value = serde_json::from_str(source)
        .map_err(|error| format!("invalid syntax-query predicate JSON: {error}"))?;
    let Some(predicates) = predicates.as_array() else {
        return Err("syntax-query predicates must be a JSON array".to_string());
    };
    for predicate in predicates {
        if predicate["capture"] == "function.name" && predicate["op"] == "eq" {
            return Ok(predicate["values"]
                .as_array()
                .and_then(|values| values.first())
                .and_then(|value| value["value"].as_str())
                .map(ToString::to_string));
        }
    }
    Ok(None)
}

fn inline_eq_predicate(source: &str) -> Option<String> {
    let marker = "#eq? @function.name \"";
    let value = source.split_once(marker)?.1;
    Some(value.split('"').next()?.to_string())
}

fn tree_sitter_query_source(options: &TreeSitterQuery) -> Result<&str, String> {
    if let Some(source) = options.source.as_deref() {
        return Ok(source);
    }
    match options.catalog_id.as_deref() {
        Some("declarations") => Ok(include_str!(
            "../../../tree-sitter/tree-sitter-rust/queries/declarations.scm"
        )),
        Some("imports") => Ok(include_str!(
            "../../../tree-sitter/tree-sitter-rust/queries/imports.scm"
        )),
        Some("calls") => Ok(include_str!(
            "../../../tree-sitter/tree-sitter-rust/queries/calls.scm"
        )),
        Some("macros") => Ok(include_str!(
            "../../../tree-sitter/tree-sitter-rust/queries/macros.scm"
        )),
        Some("cfg") => Ok(include_str!(
            "../../../tree-sitter/tree-sitter-rust/queries/cfg.scm"
        )),
        Some(catalog) => Err(format!("unknown Rust tree-sitter query catalog: {catalog}")),
        None => Err("tree-sitter query source is missing".to_string()),
    }
}

fn exact_owner_path(selector: &str) -> Result<&str, String> {
    let selector = selector.strip_prefix("rust://").unwrap_or(selector);
    let path = selector.split('#').next().unwrap_or(selector);
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "tree-sitter query requires a canonical workspace-relative selector: {selector}"
        ));
    }
    path.to_str()
        .ok_or_else(|| "tree-sitter query selector must be UTF-8".to_string())
}

fn source_line_window(source: &str, start_line: usize, end_line: usize) -> String {
    let mut rendered = source
        .lines()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line).saturating_add(1))
        .collect::<Vec<_>>()
        .join("\n");
    if !rendered.is_empty() {
        rendered.push('\n');
    }
    rendered
}

#[cfg(test)]
#[path = "../../../tests/unit/cli/runner/tree_sitter_query.rs"]
mod tests;
