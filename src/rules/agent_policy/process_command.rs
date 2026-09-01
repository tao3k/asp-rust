//! Agent policy rules for process command execution boundaries.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, path_line_location, source_line};
use crate::rules::display_path;
use crate::{AspRustFinding, AspRustRule};

use super::RUST_AGENT_POLICY_PROCESS_COMMAND_PROBE_V1;
use super::doc_boundary::documented_agent_boundary;

pub(super) fn process_command_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_PROCESS_COMMAND_PROBE_V1];
    module
        .syntax_facts
        .process_command_executions
        .iter()
        .filter(|execution| !execution.is_test_context)
        .filter_map(|execution| {
            if !is_process_command_boundary_context(
                module.report.path.to_string_lossy().as_ref(),
                &execution.function_name,
                &execution.command_expr,
            ) {
                return None;
            }
            if documented_agent_boundary(
                &module.source,
                execution.function_line,
                &[
                    "process command probe",
                    "process command boundary",
                    "command execution boundary",
                    "process runner",
                    "typed command receipt",
                ],
            ) {
                return None;
            }
            let mut boundary_parts = Vec::new();
            if execution.has_current_dir {
                boundary_parts.push("cwd");
            }
            if execution.has_env {
                boundary_parts.push("env");
            }
            let boundary = if boundary_parts.is_empty() {
                "without cwd/env shaping".to_owned()
            } else {
                format!("with {} shaping", boundary_parts.join("+"))
            };
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} executes process command `{}` in `{}` via `{}` {}.",
                    display_path(&module.report.path),
                    execution.command_expr,
                    execution.function_name,
                    execution.terminal_operation,
                    boundary
                ),
                path_line_location(&module.report.path, execution.line),
                source_line(&module.source, execution.line),
                "split command construction, execution, and receipt parsing into named owners or document the process runner boundary",
            ))
        })
        .collect()
}

fn is_process_command_boundary_context(
    path: &str,
    function_name: &str,
    command_expr: &str,
) -> bool {
    let haystack = format!("{path} {function_name} {command_expr}").to_ascii_lowercase();
    [
        "binding",
        "process command",
        "process-command",
        "process_command",
        "probe",
    ]
    .iter()
    .any(|needle| haystack.contains(needle))
}
