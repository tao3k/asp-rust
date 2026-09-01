use std::fmt::Write as _;

use super::api::RustSearchOptions;
use crate::{
    model::{AspRustConfig, AspRustRule, RustDiagnosticSeverity},
    rules::{rust_agent_policy_rules, rust_project_policy_rules},
};

const PROJECT_POLICY_OWNER: &str = "src/rules/project_policy/pack.rs";
const AGENT_POLICY_OWNER: &str = "src/rules/agent_policy/pack.rs";

const PROJECT_POLICY_TESTS: &[&str] = &[
    "tests/unit/rule_catalog.rs",
    "tests/unit/path_policy/project.rs",
    "tests/unit/path_policy/project/build_gate.rs",
    "tests/unit/path_policy/project/retired_gate.rs",
    "tests/unit/path_policy/project/verification_integration.rs",
];

const AGENT_POLICY_TESTS: &[&str] = &[
    "tests/unit/rule_catalog.rs",
    "tests/unit/policy_contract.rs",
    "tests/unit/agent_policy_snapshot.rs",
    "tests/unit/agent_policy_snapshot/algorithm_shape.rs",
    "tests/unit/agent_policy_snapshot/error_boundary.rs",
    "tests/unit/agent_policy_snapshot/primitive_api.rs",
];

pub(super) fn render_search_policy(
    _project_root: &std::path::Path,
    _config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let handles = policy_handles(query);
    let owner_paths = handle_owner_paths(&handles);
    let test_paths = handle_test_paths(&handles);
    let mut rendered = format!(
        "[search-policy] q={} handle={} owner={} tests={} pipes={}\n",
        compact_value(query),
        handles.len(),
        owner_paths.len(),
        test_paths.len(),
        compact_list(&options.pipes.iter().map(String::as_str).collect::<Vec<_>>())
    );
    let _ = writeln!(
        rendered,
        "|query {} status={} hit={} selected={} owner={}",
        compact_value(query),
        if handles.is_empty() { "miss" } else { "hit" },
        handles.len(),
        handles.len(),
        compact_list(&owner_paths)
    );
    for handle in &handles {
        let _ = writeln!(
            rendered,
            "|handle {} kind=policy-rule source=provider-policy title={} owner={} tests={} packId={} severity={}",
            handle.id,
            compact_value(handle.title),
            handle.owner_path,
            compact_list(handle.test_paths),
            handle.pack_id,
            handle.severity,
        );
    }
    let summary = if handles.is_empty() {
        "no-provider-policy-handle-matched-query"
    } else {
        "resolved-provider-owned-policy-handles"
    };
    let _ = writeln!(
        rendered,
        "|synthesis algorithm=policy-handle-catalog scope=policy summary={} selectedOwners={} testFrontier={}",
        summary,
        owner_paths.len(),
        compact_list(&test_paths)
    );
    if handles.is_empty() {
        let _ = writeln!(
            rendered,
            "|note kind=policy-not-found message={}",
            compact_value(query)
        );
    } else {
        let mut next = owner_paths
            .iter()
            .map(|path| format!("owner:{path}"))
            .collect::<Vec<_>>();
        next.extend(test_paths.iter().map(|path| format!("tests:{path}")));
        let _ = writeln!(rendered, "|next {}", next.join(","));
    }
    Ok(rendered)
}

#[derive(Clone)]
struct PolicyHandle {
    id: &'static str,
    title: &'static str,
    requirement: &'static str,
    pack_id: &'static str,
    severity: &'static str,
    owner_path: &'static str,
    test_paths: &'static [&'static str],
    aliases: Vec<String>,
    labels: Vec<String>,
    query_terms: Vec<String>,
}

fn policy_handles(query: &str) -> Vec<PolicyHandle> {
    all_policy_handles()
        .into_iter()
        .filter(|handle| handle_matches_query(handle, query))
        .collect()
}

fn all_policy_handles() -> Vec<PolicyHandle> {
    rust_project_policy_rules()
        .into_iter()
        .map(|rule| {
            rule_handle(
                rule,
                PROJECT_POLICY_OWNER,
                PROJECT_POLICY_TESTS,
                "project-policy",
            )
        })
        .chain(
            rust_agent_policy_rules().into_iter().map(|rule| {
                rule_handle(rule, AGENT_POLICY_OWNER, AGENT_POLICY_TESTS, "agent-policy")
            }),
        )
        .collect()
}

fn rule_handle(
    rule: AspRustRule,
    owner_path: &'static str,
    test_paths: &'static [&'static str],
    domain: &'static str,
) -> PolicyHandle {
    let aliases = rule_aliases(&rule);
    let labels = rule_labels(&rule, domain);
    let query_terms = rule_query_terms(&rule);
    PolicyHandle {
        id: rule.rule_id,
        title: rule.title,
        requirement: rule.requirement,
        pack_id: rule.pack_id,
        severity: severity_label(rule.severity),
        owner_path,
        test_paths,
        aliases,
        labels,
        query_terms,
    }
}

fn rule_aliases(rule: &AspRustRule) -> Vec<String> {
    let mut aliases = vec![
        rule.rule_id.to_ascii_lowercase(),
        rule.rule_id.replace('-', "_"),
        rule.rule_id.to_ascii_lowercase().replace('-', "_"),
        rule.pack_id.to_string(),
    ];
    if let Some(domain) = rule.labels.get("domain") {
        aliases.push((*domain).to_string());
    }
    dedupe_sorted(aliases)
}

fn rule_labels(rule: &AspRustRule, domain: &str) -> Vec<String> {
    dedupe_sorted(
        std::iter::once(domain.to_string())
            .chain(rule.labels.values().map(|value| (*value).to_string()))
            .collect(),
    )
}

fn rule_query_terms(rule: &AspRustRule) -> Vec<String> {
    dedupe_sorted(
        [
            rule.rule_id.to_string(),
            rule.rule_id.to_ascii_lowercase(),
            rule.rule_id.replace('-', "_"),
            rule.pack_id.to_string(),
            rule.title.to_string(),
            rule.requirement.to_string(),
        ]
        .into_iter()
        .chain(rule.labels.values().map(|value| (*value).to_string()))
        .collect(),
    )
}

fn handle_matches_query(handle: &PolicyHandle, query: &str) -> bool {
    let needle = normalized_match_text(query);
    if needle.is_empty() {
        return true;
    }
    [
        handle.id,
        handle.title,
        handle.requirement,
        handle.pack_id,
        handle.owner_path,
    ]
    .into_iter()
    .any(|value| normalized_match_text(value).contains(&needle))
        || handle
            .aliases
            .iter()
            .chain(handle.labels.iter())
            .chain(handle.query_terms.iter())
            .any(|value| normalized_match_text(value).contains(&needle))
}

fn normalized_match_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn handle_owner_paths(handles: &[PolicyHandle]) -> Vec<&'static str> {
    dedupe_preserve_order(handles.iter().map(|handle| handle.owner_path))
}

fn handle_test_paths(handles: &[PolicyHandle]) -> Vec<&'static str> {
    dedupe_preserve_order(
        handles
            .iter()
            .flat_map(|handle| handle.test_paths.iter().copied()),
    )
}

fn severity_label(severity: RustDiagnosticSeverity) -> &'static str {
    match severity {
        RustDiagnosticSeverity::Info => "info",
        RustDiagnosticSeverity::Warning => "warning",
        RustDiagnosticSeverity::Error => "error",
    }
}

fn compact_list(values: &[&str]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(",")
    }
}

fn compact_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || matches!(character, '-' | '_' | '.' | '/' | ':' | '@')
            {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn dedupe_sorted(mut values: Vec<String>) -> Vec<String> {
    values.retain(|value| !value.is_empty());
    values.sort();
    values.dedup();
    values
}

fn dedupe_preserve_order(values: impl IntoIterator<Item = &'static str>) -> Vec<&'static str> {
    values.into_iter().fold(Vec::new(), |mut unique, value| {
        if !unique.contains(&value) {
            unique.push(value);
        }
        unique
    })
}
