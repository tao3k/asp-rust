use serde_json::Value;
use std::path::Path;

use crate::parser::parse_rust_source;

pub(super) fn handle_syntax_query_operation_value(payload: &Value) -> Result<Vec<u8>, String> {
    use super::contract::{
        PROVIDER_SYNTAX_QUERY_REQUEST_SCHEMA_ID, PROVIDER_SYNTAX_QUERY_RESPONSE_SCHEMA_ID,
        ProviderSyntaxQueryRequest, ProviderSyntaxQueryResponse,
    };

    let request: ProviderSyntaxQueryRequest = serde_json::from_value(payload.clone())
        .map_err(|error| format!("decode provider syntax-query request: {error}"))?;
    if request.schema_id != PROVIDER_SYNTAX_QUERY_REQUEST_SCHEMA_ID
        || request.schema_version != "1"
        || request.language_id != "rust"
        || request.provider_id != "asp-rust"
        || request.owner_path.is_empty()
        || request.plan.patterns.is_empty()
    {
        return Err("provider syntax-query request identity or plan mismatch".to_string());
    }
    let source_digest = blake3::hash(request.source.as_bytes()).to_hex().to_string();
    if source_digest != request.source_content_digest {
        return Err("provider syntax-query source digest mismatch".to_string());
    }
    let parsed = parse_rust_source(Path::new(&request.owner_path), request.source.clone());
    if !parsed.report.is_valid {
        return Err(format!(
            "native Rust parser rejected syntax-query owner {}: {}",
            request.owner_path,
            parsed
                .report
                .parse_error
                .as_deref()
                .unwrap_or("unknown parse error")
        ));
    }
    let captures = collect_pattern_item_captures(&request, &parsed)?;
    let response = ProviderSyntaxQueryResponse {
        schema_id: PROVIDER_SYNTAX_QUERY_RESPONSE_SCHEMA_ID.to_string(),
        schema_version: "1".to_string(),
        language_id: request.language_id,
        provider_id: request.provider_id,
        owner_path: request.owner_path,
        source_content_digest: request.source_content_digest,
        query_digest: request.query_digest,
        parsed: true,
        captures,
    };
    serde_json::to_vec(&response)
        .map_err(|error| format!("encode provider syntax-query response: {error}"))
}

fn collect_pattern_item_captures(
    request: &super::contract::ProviderSyntaxQueryRequest,
    parsed: &crate::parser::ParsedRustModule,
) -> Result<Vec<super::contract::ProviderSyntaxQueryCapture>, String> {
    let mut captures = Vec::new();
    for pattern in &request.plan.patterns {
        for item in &parsed.syntax_facts.top_level_items {
            if !pattern_matches_item(pattern, item.kind)
                || !predicates_match_item(&request.plan.predicates, pattern, item.name.as_deref())?
            {
                continue;
            }
            let (item_start, item_end) = line_byte_range(&request.source, item.line, item.end_line);
            for capture_name in &pattern.captures {
                let (source_byte_start, source_byte_end) = capture_source_range(
                    &request.source,
                    item_start,
                    item_end,
                    capture_name,
                    item.name.as_deref(),
                );
                captures.push(super::contract::ProviderSyntaxQueryCapture {
                    pattern_index: pattern.index,
                    capture_name: capture_name.clone(),
                    native_fact_ref: format!(
                        "rust:item:{}:{}:{}:{}",
                        request.owner_path,
                        item.line,
                        item.end_line,
                        item.name.as_deref().unwrap_or(item.kind)
                    ),
                    source_byte_start: source_byte_start as u64,
                    source_byte_end: source_byte_end as u64,
                });
            }
        }
    }
    Ok(captures)
}

fn capture_source_range(
    source: &str,
    item_start: usize,
    item_end: usize,
    capture_name: &str,
    item_name: Option<&str>,
) -> (usize, usize) {
    if !capture_name.ends_with("name") {
        return (item_start, item_end);
    }
    item_name
        .and_then(|name| {
            source[item_start..item_end]
                .find(name)
                .map(|offset| (item_start + offset, item_start + offset + name.len()))
        })
        .unwrap_or((item_start, item_end))
}

fn pattern_matches_item(pattern: &super::contract::SyntaxQueryPattern, item_kind: &str) -> bool {
    pattern.node_types.iter().any(|node_type| {
        matches!(
            (node_type.as_str(), item_kind),
            ("function_item", "fn" | "function")
                | ("struct_item", "struct")
                | ("enum_item", "enum")
                | ("trait_item", "trait")
                | ("impl_item", "impl")
                | ("mod_item", "mod")
                | ("use_declaration", "use")
                | ("const_item", "const")
                | ("static_item", "static")
                | ("type_item", "type")
                | ("macro_definition", "macro")
        )
    })
}

fn predicates_match_item(
    predicates: &[super::contract::SyntaxQueryPredicate],
    pattern: &super::contract::SyntaxQueryPattern,
    item_name: Option<&str>,
) -> Result<bool, String> {
    predicates
        .iter()
        .filter(|predicate| pattern.captures.contains(&predicate.capture))
        .try_fold(true, |matched, predicate| {
            if !matched {
                return Ok(false);
            }
            let actual = item_name.unwrap_or_default();
            let values = predicate
                .values
                .iter()
                .map(|value| match value {
                    super::contract::SyntaxQueryPredicateValue::String(value) => value.as_str(),
                    super::contract::SyntaxQueryPredicateValue::Capture(_) => actual,
                })
                .collect::<Vec<_>>();
            let positive = match predicate.op {
                super::contract::SyntaxQueryPredicateOp::Eq
                | super::contract::SyntaxQueryPredicateOp::AnyEq
                | super::contract::SyntaxQueryPredicateOp::AnyOf => values.contains(&actual),
                super::contract::SyntaxQueryPredicateOp::Match
                | super::contract::SyntaxQueryPredicateOp::AnyMatch => values
                    .iter()
                    .map(|value| regex::Regex::new(value).map(|regex| regex.is_match(actual)))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("invalid syntax-query predicate regex: {error}"))?
                    .into_iter()
                    .any(|value| value),
                super::contract::SyntaxQueryPredicateOp::NotEq => {
                    values.iter().all(|value| *value != actual)
                }
                super::contract::SyntaxQueryPredicateOp::NotMatch => values
                    .iter()
                    .map(|value| regex::Regex::new(value).map(|regex| !regex.is_match(actual)))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("invalid syntax-query predicate regex: {error}"))?
                    .into_iter()
                    .all(|value| value),
            };
            Ok(positive)
        })
}

fn line_byte_range(source: &str, start_line: usize, end_line: usize) -> (usize, usize) {
    let line_ends = source
        .split_inclusive('\n')
        .scan(0usize, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, *offset))
        })
        .collect::<Vec<_>>();
    let start = line_ends
        .get(start_line.saturating_sub(1))
        .map_or(0, |(start, _)| *start);
    let end = line_ends
        .get(end_line.saturating_sub(1))
        .map_or(source.len(), |(_, end)| *end);
    (start.min(source.len()), end.min(source.len()))
}
