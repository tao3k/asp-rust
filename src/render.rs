//! Agent-first compact text rendering for Rust harness diagnostics.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use crate::{RustDiagnosticSeverity, RustHarnessFinding, RustHarnessReport};

const FAILURE_FRONTIER_CONTEXT_LINES: usize = 12;

/// Render a compact diagnostic report with advice enabled.
#[must_use]
pub fn render_rust_project_harness(report: &RustHarnessReport) -> String {
    render_rust_project_harness_with_options(report, None, true)
}

/// Render a structured JSON diagnostic report for tool consumers.
///
/// This is the library equivalent of the CLI `--json` output mode.
///
/// # Errors
///
/// Returns a serialization error if the report cannot be encoded as JSON.
pub fn render_rust_project_harness_json(
    report: &RustHarnessReport,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(report)
}

/// Render a compact failure frontier for the next exact source reads.
#[must_use]
pub fn render_rust_project_harness_failure_frontier(
    report: &RustHarnessReport,
    project_root: &Path,
    max_hot_blocks: usize,
) -> String {
    let hot_blocks = failure_frontier_hot_blocks(report, project_root, max_hot_blocks);
    if hot_blocks.is_empty() {
        return String::new();
    }

    let mut rendered = String::new();
    let blocking_findings = report.blocking_findings(None);
    let advisory_findings = report.advisory_findings();
    let _ = writeln!(
        rendered,
        "[fail] rust blockingFindings={} advisoryFindings={} changedInvariants={} hotBlocks={}",
        blocking_findings.len(),
        advisory_findings.len(),
        report.invariant_candidates.len(),
        hot_blocks.len()
    );
    let _ = writeln!(
        rendered,
        "|failureFrontier status=ready source=rust-check hotBlocks={} directSourceReadCode<={} changedInvariants={} findings={}",
        hot_blocks.len(),
        hot_blocks.len(),
        report.invariant_candidates.len(),
        report.findings.len()
    );
    for hot_block in hot_blocks {
        let _ = writeln!(
            rendered,
            "|hotBlock selector={} source={} rule={} line={}",
            hot_block.selector, hot_block.source, hot_block.rule_id, hot_block.line
        );
        let _ = writeln!(
            rendered,
            "|next asp rust query --from-hook direct-source-read --code --workspace . --selector {}",
            shell_single_quote(&hot_block.selector)
        );
    }
    rendered
}

/// Render only non-blocking agent advice.
#[must_use]
pub fn render_rust_project_harness_advice(report: &RustHarnessReport) -> String {
    let severities = BTreeSet::from([RustDiagnosticSeverity::Info]);
    render_finding_list(&report.blocking_findings(Some(&severities)))
}

/// Render a compact diagnostic report with explicit severity and advice options.
#[must_use]
pub fn render_rust_project_harness_with_options(
    report: &RustHarnessReport,
    severities: Option<&BTreeSet<RustDiagnosticSeverity>>,
    include_advice: bool,
) -> String {
    let blocking_findings = report.blocking_findings(severities);
    let advice = if include_advice {
        deduplicate_advice_findings(&report.advisory_findings(), &blocking_findings)
    } else {
        Vec::new()
    };
    let findings = blocking_findings
        .iter()
        .chain(advice.iter())
        .copied()
        .collect::<Vec<_>>();
    if findings.is_empty() {
        return "[ok] rust\n".to_string();
    }
    render_finding_list(&findings)
}

fn render_finding(finding: &RustHarnessFinding) -> String {
    let path = finding
        .location
        .path
        .as_ref()
        .map_or_else(|| "<memory>".to_string(), |path| display_path(path));
    let display_column = finding.location.column + 1;
    let mut rendered = format!(
        "[{}] {}: {}\n@ {path}:{}:{display_column}\n",
        finding.rule_id,
        title_case(finding.severity.as_str()),
        finding.title,
        finding.location.line
    );
    let _ = writeln!(rendered, "fix: {}", finding.label);
    if let Some(source_line) = &finding.source_line {
        let _ = writeln!(
            rendered,
            "line: {} | {}",
            finding.location.line, source_line
        );
    }
    let _ = writeln!(rendered, "Help: {}", finding.summary);
    let _ = writeln!(rendered, "Contract: {}", finding.requirement);
    rendered
}

fn deduplicate_advice_findings<'a>(
    advice_findings: &[&'a RustHarnessFinding],
    blocking_findings: &[&RustHarnessFinding],
) -> Vec<&'a RustHarnessFinding> {
    let blocking_keys = blocking_findings
        .iter()
        .map(|finding| finding_key(finding))
        .collect::<BTreeSet<_>>();
    advice_findings
        .iter()
        .copied()
        .filter(|finding| !blocking_keys.contains(&finding_key(finding)))
        .collect()
}

fn finding_key(finding: &RustHarnessFinding) -> (String, Option<String>, usize, usize) {
    (
        finding.rule_id.clone(),
        finding
            .location
            .path
            .as_ref()
            .map(|path| display_path(path)),
        finding.location.line,
        finding.location.column,
    )
}

fn render_finding_list(findings: &[&RustHarnessFinding]) -> String {
    findings
        .iter()
        .map(|finding| render_finding(finding))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FailureFrontierHotBlock {
    selector: String,
    source: &'static str,
    rule_id: String,
    line: usize,
}

fn failure_frontier_hot_blocks(
    report: &RustHarnessReport,
    project_root: &Path,
    max_hot_blocks: usize,
) -> Vec<FailureFrontierHotBlock> {
    if max_hot_blocks == 0 {
        return Vec::new();
    }

    let mut hot_blocks = Vec::new();
    let mut seen_paths = BTreeSet::new();
    for candidate in &report.invariant_candidates {
        push_failure_frontier_hot_block(
            &mut hot_blocks,
            &mut seen_paths,
            project_root,
            candidate.location.path.as_deref(),
            candidate.location.line,
            candidate.source_rule_id.as_str(),
            "invariant",
        );
        if hot_blocks.len() >= max_hot_blocks {
            return hot_blocks;
        }
    }
    if !hot_blocks.is_empty() {
        return hot_blocks;
    }
    for finding in report.blocking_findings(None) {
        push_failure_frontier_hot_block(
            &mut hot_blocks,
            &mut seen_paths,
            project_root,
            finding.location.path.as_deref(),
            finding.location.line,
            &finding.rule_id,
            "finding",
        );
        if hot_blocks.len() >= max_hot_blocks {
            return hot_blocks;
        }
    }
    for finding in report.advisory_findings() {
        push_failure_frontier_hot_block(
            &mut hot_blocks,
            &mut seen_paths,
            project_root,
            finding.location.path.as_deref(),
            finding.location.line,
            &finding.rule_id,
            "advice",
        );
        if hot_blocks.len() >= max_hot_blocks {
            return hot_blocks;
        }
    }
    hot_blocks
}

fn push_failure_frontier_hot_block(
    hot_blocks: &mut Vec<FailureFrontierHotBlock>,
    seen_paths: &mut BTreeSet<String>,
    project_root: &Path,
    path: Option<&Path>,
    line: usize,
    rule_id: &str,
    source: &'static str,
) {
    let Some(path) = path else {
        return;
    };
    let display_path = project_relative_display_path(project_root, path);
    if !seen_paths.insert(display_path.clone()) {
        return;
    }
    let line = line.max(1);
    let start_line = line.saturating_sub(FAILURE_FRONTIER_CONTEXT_LINES).max(1);
    let end_line = line + FAILURE_FRONTIER_CONTEXT_LINES;
    hot_blocks.push(FailureFrontierHotBlock {
        selector: format!("{display_path}:{start_line}-{end_line}"),
        source,
        rule_id: rule_id.to_string(),
        line,
    });
}

fn project_relative_display_path(project_root: &Path, path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(project_root)
        && !relative.as_os_str().is_empty()
    {
        return display_path(relative);
    }
    display_path(path)
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().collect::<String>() + chars.as_str()
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
