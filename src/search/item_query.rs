//! Parser-owned item query matching and compact code lines.

use std::path::Path;

use crate::parser::native_syntax::item_projection::RustItemProjectionNodeSyntax;
use crate::parser::{ParsedRustModule, RustTopLevelItemSyntax};

use super::format::render_item_locator_line_with_read;
use super::limits::{SEARCH_ITEM_LIMIT, SEARCH_OWNER_LIMIT};

pub(super) fn owner_item_count(
    matching_modules: &[&ParsedRustModule],
    include_items: bool,
    item_query: Option<&str>,
) -> usize {
    if !include_items {
        return 0;
    }
    matching_modules
        .iter()
        .flat_map(|module| module_items_for_query(module, item_query))
        .count()
}

pub(super) fn module_items_for_query<'a>(
    module: &'a ParsedRustModule,
    item_query: Option<&str>,
) -> Vec<&'a RustTopLevelItemSyntax> {
    let Some(query) = item_query.map(str::trim).filter(|query| !query.is_empty()) else {
        return named_module_items(module);
    };
    let searchable_items = searchable_module_items(module);
    let terms = item_query_terms(query);
    let exact = searchable_items
        .iter()
        .copied()
        .filter(|item| {
            terms
                .iter()
                .any(|term| item_matches_query_exact(item, term))
        })
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return exact;
    }
    searchable_items
        .into_iter()
        .filter(|item| {
            terms
                .iter()
                .any(|term| item_matches_query_fuzzy(item, term))
        })
        .collect()
}

pub(super) fn render_owner_item_lines(
    package_root: &Path,
    matching_modules: &[&ParsedRustModule],
    item_query: Option<&str>,
    _names_only: bool,
    item_projection_metadata: bool,
) -> Vec<String> {
    matching_modules
        .iter()
        .take(SEARCH_OWNER_LIMIT)
        .flat_map(|module| {
            render_module_item_lines(package_root, module, item_query, item_projection_metadata)
        })
        .collect()
}

pub(super) fn render_owner_item_hot_lines(
    package_root: &Path,
    matching_modules: &[&ParsedRustModule],
    item_query: Option<&str>,
) -> Vec<String> {
    let Some(query) = item_query.map(str::trim).filter(|query| !query.is_empty()) else {
        return Vec::new();
    };
    let terms = item_query_terms(query);
    let named_items = matching_modules
        .iter()
        .flat_map(|module| searchable_module_items(module))
        .collect::<Vec<_>>();
    let selected_items = matching_modules
        .iter()
        .flat_map(|module| module_items_for_query(module, Some(query)))
        .collect::<Vec<_>>();
    item_query_term_revisions(&named_items, &selected_items, &terms)
        .into_iter()
        .filter_map(|revision| {
            let (source, target) = revision.split_once("->")?;
            let (module, item) = matching_modules.iter().find_map(|module| {
                find_item_by_query_candidate(module, target).map(|item| (*module, item))
            })?;
            let mut line =
                render_item_locator_line_with_read(package_root, &module.report.path, item)
                    .replacen("|item ", "|hot ", 1);
            line.push_str(" revise=");
            line.push_str(source);
            Some(line)
        })
        .collect()
}

pub(super) fn render_owner_item_frontier_lines(
    package_root: &Path,
    matching_modules: &[&ParsedRustModule],
) -> Vec<String> {
    matching_modules
        .iter()
        .take(SEARCH_OWNER_LIMIT)
        .flat_map(|module| {
            named_module_items(module)
                .into_iter()
                .take(SEARCH_ITEM_LIMIT)
                .map(|item| {
                    render_item_locator_line_with_read(package_root, &module.report.path, item)
                        .replacen("|item ", "|hot ", 1)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(super) fn render_item_query_line(
    matching_modules: &[&ParsedRustModule],
    item_query: Option<&str>,
    names_only: bool,
) -> Option<String> {
    let query = item_query
        .map(str::trim)
        .filter(|query| !query.is_empty())?;
    let summary = item_query_match_summary(matching_modules, query);
    let output = if names_only { " output=names" } else { "" };
    let candidate_field = if summary.candidates.is_empty() {
        String::new()
    } else {
        format!(" candidates={}", summary.candidates.join(","))
    };
    let revision_field = if summary.revisions.is_empty() {
        String::new()
    } else {
        format!(" revise={}", summary.revisions.join(","))
    };
    let candidate_field = format!("{candidate_field}{revision_field}");
    let next = if summary.mode == ItemQueryMatchMode::Miss {
        summary.next.as_deref().unwrap_or("revise-query")
    } else if names_only && summary.item_count > 1 {
        "select-item"
    } else {
        "query-code"
    };
    Some(format!(
        "|query itemQuery={query} status={} match={} item={} reason={}{}{} next={next}",
        summary.mode.status(),
        summary.mode.label(),
        summary.item_count,
        summary.mode.reason(),
        output,
        candidate_field
    ))
}

fn render_module_item_lines(
    package_root: &Path,
    module: &ParsedRustModule,
    item_query: Option<&str>,
    item_projection_metadata: bool,
) -> Vec<String> {
    module_items_for_query(module, item_query)
        .into_iter()
        .take(SEARCH_ITEM_LIMIT)
        .flat_map(|item| render_item_lines(package_root, module, item, item_projection_metadata))
        .collect()
}

fn render_item_lines(
    package_root: &Path,
    module: &ParsedRustModule,
    item: &RustTopLevelItemSyntax,
    item_projection_metadata: bool,
) -> Vec<String> {
    let mut line = render_item_locator_line_with_read(package_root, &module.report.path, item);
    if item_projection_metadata {
        let parser_nodes = projection_node_tokens(&item.projection_nodes);
        if !parser_nodes.is_empty() {
            line.push_str(" nodes=");
            line.push_str(&parser_nodes);
        }
    }
    let mut lines = vec![line];
    lines.extend(item.projection_nodes.iter().filter_map(|node| {
        super::format::render_projection_item_locator_line_with_read(
            package_root,
            &module.report.path,
            node,
        )
    }));
    lines
}

fn projection_node_tokens(nodes: &[RustItemProjectionNodeSyntax]) -> String {
    nodes
        .iter()
        .map(|node| {
            let id = projection_node_id(node);
            let native_id = projection_native_id(node);
            let fingerprint = projection_structural_fingerprint(node);
            let label = encode_projection_node_label(node.label.trim());
            format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}",
                id,
                node.kind,
                node.role,
                node.depth,
                node.line,
                node.end_line,
                native_id,
                fingerprint,
                label
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn encode_projection_node_label(label: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(label.len() * 2);
    for byte in label.as_bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn projection_node_id(node: &RustItemProjectionNodeSyntax) -> String {
    format!(
        "{}-{}-{}-{}",
        safe_projection_id_part(node.kind),
        node.line,
        node.end_line,
        stable_projection_hash(&node.label)
    )
}

fn projection_native_id(node: &RustItemProjectionNodeSyntax) -> String {
    format!(
        "rust-{}-{}-{}-{}",
        safe_projection_id_part(node.kind),
        node.line,
        node.end_line,
        stable_projection_hash(&node.label)
    )
}

fn projection_structural_fingerprint(node: &RustItemProjectionNodeSyntax) -> String {
    format!(
        "{}-{}-{}-{}-{}",
        safe_projection_id_part(node.kind),
        safe_projection_id_part(node.role),
        node.line,
        node.end_line,
        stable_projection_hash(&node.label)
    )
}

fn safe_projection_id_part(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    if normalized.is_empty() {
        "node".to_string()
    } else {
        normalized
    }
}

fn stable_projection_hash(value: &str) -> String {
    let mut hash = 2_166_136_261u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    format!("{hash:x}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemQueryMatchMode {
    Exact,
    FallbackContains,
    Miss,
}

struct ItemQuerySummary {
    mode: ItemQueryMatchMode,
    item_count: usize,
    candidates: Vec<String>,
    revisions: Vec<String>,
    next: Option<String>,
}

impl ItemQueryMatchMode {
    fn status(self) -> &'static str {
        match self {
            Self::Exact | Self::FallbackContains => "hit",
            Self::Miss => "miss",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::FallbackContains => "fallback-contains",
            Self::Miss => "none",
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Self::Exact => "parser-item-exact",
            Self::FallbackContains => "parser-item-fallback",
            Self::Miss => "parser-item-miss",
        }
    }
}

fn item_query_match_summary(
    matching_modules: &[&ParsedRustModule],
    query: &str,
) -> ItemQuerySummary {
    let terms = item_query_terms(query);
    let named_items = matching_modules
        .iter()
        .flat_map(|module| searchable_module_items(module))
        .collect::<Vec<_>>();
    let exact_count = matching_modules
        .iter()
        .flat_map(|module| searchable_module_items(module))
        .filter(|item| {
            terms
                .iter()
                .any(|term| item_matches_query_exact(item, term))
        })
        .count();
    if exact_count > 0 {
        return ItemQuerySummary {
            mode: ItemQueryMatchMode::Exact,
            item_count: exact_count,
            candidates: Vec::new(),
            revisions: Vec::new(),
            next: None,
        };
    }
    let fuzzy_items = named_items
        .iter()
        .filter(|item| {
            terms
                .iter()
                .any(|term| item_matches_query_fuzzy(item, term))
        })
        .copied()
        .collect::<Vec<_>>();
    let fuzzy_count = fuzzy_items.len();
    if fuzzy_count > 0 {
        let revisions = item_query_term_revisions(&named_items, &fuzzy_items, &terms);
        ItemQuerySummary {
            mode: ItemQueryMatchMode::FallbackContains,
            item_count: fuzzy_count,
            candidates: Vec::new(),
            revisions,
            next: None,
        }
    } else {
        let candidates = item_query_miss_candidates(&named_items, &terms);
        let next = candidates
            .first()
            .map(|candidate| format!("query:{candidate}"));
        ItemQuerySummary {
            mode: ItemQueryMatchMode::Miss,
            item_count: 0,
            candidates,
            revisions: Vec::new(),
            next,
        }
    }
}

fn item_query_miss_candidates(items: &[&RustTopLevelItemSyntax], terms: &[&str]) -> Vec<String> {
    let mut scored = items
        .iter()
        .filter_map(|item| {
            let name = item_query_candidate_name(item)?;
            let score = terms
                .iter()
                .map(|term| item_query_candidate_score(name, term))
                .max()
                .unwrap_or(0);
            (score > 0).then(|| (score, name.to_string()))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|(left_score, left_name), (right_score, right_name)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_name.cmp(right_name))
    });
    let mut seen = std::collections::HashSet::new();
    scored
        .into_iter()
        .map(|(_, name)| name)
        .filter(|name| seen.insert(name.clone()))
        .take(5)
        .collect()
}

fn item_query_candidate_name(item: &RustTopLevelItemSyntax) -> Option<&str> {
    item.name
        .as_deref()
        .or(item.impl_target_name.as_deref())
        .or(item.function_name.as_deref())
        .or(item.macro_name.as_deref())
        .or(item.include_target.as_deref())
}

fn item_query_candidate_score(candidate: &str, term: &str) -> usize {
    let candidate = normalize_identifier(candidate);
    let term = normalize_identifier(term);
    if candidate.is_empty() || term.is_empty() {
        return 0;
    }
    if candidate == term {
        return 100;
    }
    if candidate.contains(&term) || term.contains(&candidate) {
        return 80;
    }
    if let Some(prefix) = term.rsplit_once('_').map(|(prefix, _)| prefix)
        && prefix.len() >= 4
        && candidate.starts_with(prefix)
    {
        return 70 + prefix.len().min(20);
    }
    let shared_tokens = shared_identifier_token_count(&candidate, &term);
    if shared_tokens >= 2 {
        return 40 + shared_tokens * 5;
    }
    let common_prefix = common_prefix_len(&candidate, &term);
    if common_prefix >= 4 {
        return 20 + common_prefix.min(20);
    }
    0
}

fn find_item_by_query_candidate<'a>(
    module: &'a ParsedRustModule,
    candidate: &str,
) -> Option<&'a RustTopLevelItemSyntax> {
    searchable_module_items(module)
        .into_iter()
        .find(|item| item_matches_query_exact(item, candidate))
}

fn normalize_identifier(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }
    normalized.trim_matches('_').to_string()
}

fn shared_identifier_token_count(left: &str, right: &str) -> usize {
    let left_tokens = identifier_tokens(left);
    let right_tokens = identifier_tokens(right);
    left_tokens
        .iter()
        .filter(|token| right_tokens.iter().any(|right| right == *token))
        .count()
}

fn identifier_tokens(value: &str) -> Vec<&str> {
    value.split('_').filter(|token| token.len() >= 2).collect()
}

fn common_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn named_module_items(module: &ParsedRustModule) -> Vec<&RustTopLevelItemSyntax> {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.name.is_some())
        .collect()
}

fn searchable_module_items(module: &ParsedRustModule) -> Vec<&RustTopLevelItemSyntax> {
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.name.is_some() || item.impl_target_name.is_some())
        .collect()
}

fn item_query_term_revisions(
    items: &[&RustTopLevelItemSyntax],
    selected_items: &[&RustTopLevelItemSyntax],
    terms: &[&str],
) -> Vec<String> {
    terms
        .iter()
        .filter_map(|term| {
            if selected_items.iter().any(|item| {
                item_matches_query_exact(item, term) || item_matches_query_fuzzy(item, term)
            }) {
                return None;
            }
            let single_term = [*term];
            let mut candidates = item_query_miss_candidates(items, &single_term);
            if candidates.is_empty()
                && let Some(alias) = snake_case_query_alias(term)
            {
                let alias_terms = [alias.as_str()];
                candidates = item_query_miss_candidates(items, &alias_terms);
            }
            candidates
                .into_iter()
                .next()
                .map(|candidate| format!("{term}->{candidate}"))
        })
        .collect()
}

fn snake_case_query_alias(term: &str) -> Option<String> {
    let mut alias = String::new();
    let mut changed = false;
    let mut previous_word = false;
    for character in term.chars() {
        if character.is_ascii_uppercase() {
            if previous_word && !alias.ends_with('_') {
                alias.push('_');
            }
            alias.push(character.to_ascii_lowercase());
            changed = true;
            previous_word = true;
        } else {
            alias.push(character);
            previous_word = character.is_ascii_lowercase() || character.is_ascii_digit();
        }
    }
    if changed && alias != term {
        Some(alias)
    } else {
        None
    }
}

fn item_query_terms(query: &str) -> Vec<&str> {
    query
        .split('|')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .collect()
}

fn item_matches_query_exact(item: &RustTopLevelItemSyntax, query: &str) -> bool {
    item_query_candidates(item)
        .into_iter()
        .flatten()
        .any(|candidate| candidate == query)
        || item_projection_symbols(item).any(|candidate| candidate == query)
}

fn item_matches_query_fuzzy(item: &RustTopLevelItemSyntax, query: &str) -> bool {
    item_query_candidates(item)
        .into_iter()
        .flatten()
        .any(|candidate| candidate.contains(query))
        || item_projection_symbols(item).any(|candidate| candidate.contains(query))
}

fn item_projection_symbols(item: &RustTopLevelItemSyntax) -> impl Iterator<Item = &str> {
    item.projection_nodes.iter().filter_map(|node| {
        node.canonical_item_identity
            .as_ref()
            .map(|identity| identity.symbol.as_str())
    })
}

fn item_query_candidates(item: &RustTopLevelItemSyntax) -> [Option<&str>; 6] {
    [
        item.name.as_deref(),
        item.impl_target_name.as_deref(),
        item.function_name.as_deref(),
        item.macro_name.as_deref(),
        item.include_target.as_deref(),
        Some(item.kind),
    ]
}
