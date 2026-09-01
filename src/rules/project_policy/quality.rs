//! Project-policy quality checks for waiver and wrapper surfaces.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{ParsedRustModule, file_location, path_line_location, source_line};
use crate::{AspRustConfig, AspRustFinding, AspRustRule};

use super::support::display_project_path;
use super::{
    RUST_PROJ_R017, RUST_PROJ_R018, RUST_PROJ_R019, RUST_PROJ_R020, RUST_PROJ_R021, RUST_PROJ_R022,
};

pub(super) fn quality_findings(
    project_root: &Path,
    config: &AspRustConfig,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut findings = Vec::new();
    findings.extend(weak_advice_explanation_findings(
        project_root,
        config,
        rules,
    ));
    findings.extend(fake_identity_fallback_findings(
        project_root,
        modules,
        rules,
    ));
    findings.extend(redundant_workspace_wrapper_findings(
        project_root,
        modules,
        rules,
    ));
    findings.extend(silent_evidence_default_findings(
        project_root,
        modules,
        rules,
    ));
    findings.extend(source_location_sentinel_findings(
        project_root,
        modules,
        rules,
    ));
    findings.extend(candidate_loop_telemetry_findings(
        project_root,
        modules,
        rules,
    ));
    findings
}

fn weak_advice_explanation_findings(
    project_root: &Path,
    config: &AspRustConfig,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    [
        (
            "agent_advice_allow_explanation",
            config.agent_advice_allow_explanation.as_deref(),
        ),
        (
            "cargo_check_advice_allow_explanation",
            config.cargo_check_advice_allow_explanation.as_deref(),
        ),
        (
            "cargo_test_advice_allow_explanation",
            config.cargo_test_advice_allow_explanation.as_deref(),
        ),
    ]
    .into_iter()
    .filter_map(|(field_name, explanation)| {
        let explanation = explanation?;
        if advice_explanation_is_structured(explanation) {
            return None;
        }
        let rule = &rules[RUST_PROJ_R017];
        Some(AspRustFinding::from_rule(
            rule,
            format!(
                "{field_name} is configured without a structured allowance contract."
            ),
            file_location(build_script_or_manifest(project_root)),
            None,
            "rewrite the explanation with scope=..., owner=..., finding_category=..., why_safe_now=..., and cleanup_trigger=...",
        ))
    })
    .collect()
}

fn advice_explanation_is_structured(explanation: &str) -> bool {
    let lower = explanation.to_ascii_lowercase();
    [
        "scope=",
        "owner=",
        "finding_category=",
        "why_safe_now=",
        "cleanup_trigger=",
    ]
    .into_iter()
    .all(|marker| lower.contains(marker))
}

fn fake_identity_fallback_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_PROJ_R018];
    modules
        .iter()
        .filter(|module| module.source.contains("CARGO_PKG_NAME"))
        .filter_map(|module| {
            let line = module
                .source
                .lines()
                .position(line_has_fake_identity_fallback)?;
            let line = line + 1;
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} synthesizes a fake Cargo package identity fallback.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "fail closed when CARGO_PKG_NAME is missing or empty, or pass the crate label explicitly from the caller",
            ))
        })
        .collect()
}

fn line_has_fake_identity_fallback(line: &str) -> bool {
    [
        "\"unknown\"",
        "\"unknown-cargo-package\"",
        "\"unknown-crate\"",
        "\"unknown-package\"",
        "\"default-crate\"",
        "\"todo\"",
    ]
    .into_iter()
    .any(|sentinel| line.contains(sentinel))
}

fn redundant_workspace_wrapper_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_PROJ_R019];
    modules
        .iter()
        .flat_map(|module| {
            module
                .syntax_facts
                .top_level_items
                .iter()
                .filter_map(move |item| {
                    let function_name = item.function_name.as_deref()?;
                    if !item.is_public
                        || !is_redundant_member_build_gate_alias(function_name)
                        || !item_range_contains_harness_alias_target(module, item.line, item.end_line)
                    {
                        return None;
                    }
                    Some(AspRustFinding::from_rule(
                        rule,
                        format!(
                            "{} exposes redundant public build-gate alias `{function_name}`.",
                            display_project_path(project_root, &module.report.path)
                        ),
                        path_line_location(&module.report.path, item.line),
                        source_line(&module.source, item.line),
                        "remove the alias, or add deprecation metadata plus a bounded migration deadline",
                    ))
                })
        })
        .collect()
}

fn is_redundant_member_build_gate_alias(function_name: &str) -> bool {
    function_name.starts_with("assert_member_")
        && function_name.contains("build_gate")
        && function_name.contains("_from_env")
        && !function_name.contains("harness")
}

fn item_range_contains_harness_alias_target(
    module: &ParsedRustModule,
    start_line: usize,
    end_line: usize,
) -> bool {
    module
        .source
        .lines()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line).saturating_add(1))
        .any(|line| line.contains("assert_member_harness_build_gate_from_env"))
}

fn silent_evidence_default_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_PROJ_R020];
    modules
        .iter()
        .filter(|module| !is_test_module(module))
        .filter(|module| is_quality_sensitive_module(module))
        .filter_map(|module| {
            let line = module
                .source
                .lines()
                .position(line_has_silent_evidence_default)?;
            let line = line + 1;
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} silently defaults missing evidence-bearing data.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "return Option/Result, skip with typed telemetry, or provide an explicit non-empty fallback reason",
            ))
        })
        .collect()
}

fn line_has_silent_evidence_default(line: &str) -> bool {
    line.contains("unwrap_or_default")
        && [
            "anchor",
            "candidate",
            "canonical",
            "entity",
            "evidence",
            "identity",
            "lineage",
            "path",
            "policy",
            "semantic",
        ]
        .into_iter()
        .any(|term| line.to_ascii_lowercase().contains(term))
}

fn source_location_sentinel_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_PROJ_R021];
    modules
        .iter()
        .filter(|module| !is_test_module(module))
        .filter(|module| is_quality_sensitive_module(module))
        .filter_map(|module| {
            let line = module
                .source
                .lines()
                .position(line_has_source_location_sentinel)?;
            let line = line + 1;
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} uses a sentinel source location value.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "propagate a decode error or drop the candidate with explicit telemetry instead of inventing a source location",
            ))
        })
        .collect()
}

fn line_has_source_location_sentinel(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('"') || trimmed.contains("line.contains(") {
        return false;
    }
    line.contains("usize::MAX")
        && ["line", "column", "range", "location"]
            .into_iter()
            .any(|term| line.to_ascii_lowercase().contains(term))
}

fn candidate_loop_telemetry_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_PROJ_R022];
    modules
        .iter()
        .filter(|module| !is_test_module(module))
        .filter(|module| is_candidate_lookup_module(module))
        .filter(|module| module.source.contains("observe_batch"))
        .filter(|module| module.source.contains("observe_match"))
        .filter(|module| !module_records_candidate_rejection(module))
        .filter_map(|module| {
            let line = first_score_continue_line(&module.source)?;
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} skips candidate rows without rejection telemetry.",
                    display_project_path(project_root, &module.report.path)
                ),
                path_line_location(&module.report.path, line),
                source_line(&module.source, line),
                "record skipped/filtered candidate counts or route the loop through a shared telemetry-aware candidate collector",
            ))
        })
        .collect()
}

fn module_records_candidate_rejection(module: &ParsedRustModule) -> bool {
    [
        "observe_skip",
        "observe_reject",
        "observe_filtered",
        "observe_miss",
    ]
    .into_iter()
    .any(|needle| module.source.contains(needle))
}

fn first_score_continue_line(source: &str) -> Option<usize> {
    let lines = source.lines().collect::<Vec<_>>();
    lines.iter().enumerate().find_map(|(index, line)| {
        let lower = line.to_ascii_lowercase();
        if !(lower.contains("score <= 0.0") || lower.contains("score == 0.0")) {
            return None;
        }
        let has_continue = lines
            .iter()
            .skip(index)
            .take(4)
            .any(|candidate| candidate.contains("continue;"));
        has_continue.then_some(index + 1)
    })
}

fn is_candidate_lookup_module(module: &ParsedRustModule) -> bool {
    let path = module.report.path.to_string_lossy().to_ascii_lowercase();
    path.contains("candidate") || (path.contains("query") && path.contains("lookup"))
}

fn is_quality_sensitive_module(module: &ParsedRustModule) -> bool {
    let path = module.report.path.to_string_lossy().to_ascii_lowercase();
    [
        "candidate",
        "dedup",
        "evidence",
        "graph",
        "lookup",
        "orchestrate",
        "policy",
        "query",
        "search",
    ]
    .into_iter()
    .any(|segment| path.contains(segment))
}

fn is_test_module(module: &ParsedRustModule) -> bool {
    let path = module.report.path.to_string_lossy();
    path.contains("/tests/") || path.contains("\\tests\\")
}

fn build_script_or_manifest(project_root: &Path) -> std::path::PathBuf {
    let build_script = project_root.join("build.rs");
    if build_script.exists() {
        build_script
    } else {
        project_root.join("Cargo.toml")
    }
}
