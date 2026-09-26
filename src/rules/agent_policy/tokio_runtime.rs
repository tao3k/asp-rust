//! Agent policy rules for Tokio runtime ownership boundaries.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, path_line_location, source_line};
use crate::rules::display_path;
use crate::{AspRustFinding, AspRustRule};

use super::RUST_AGENT_POLICY_TOKIO_RUNTIME_BOUNDARY_V1;
use super::doc_boundary::documented_agent_boundary;

pub(super) fn tokio_runtime_boundary_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_TOKIO_RUNTIME_BOUNDARY_V1];
    module
        .syntax_facts
        .tokio_runtime_operations
        .iter()
        .filter(|operation| !operation.is_test_context)
        .filter_map(|operation| {
            let module_path = module.report.path.to_string_lossy();
            if is_runtime_owner_context(&module_path, &operation.function_name) {
                return None;
            }
            if documented_agent_boundary(
                &module.source,
                operation.function_line,
                &[
                    "tokio runtime boundary",
                    "runtime task boundary",
                    "runtime owner",
                    "runtime facade",
                    "task lifecycle boundary",
                    "blocking task boundary",
                ],
            ) {
                return None;
            }
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} calls Tokio runtime operation `{}` in `{}` without a runtime owner boundary.",
                    display_path(&module.report.path),
                    operation.call_expr,
                    operation.function_name,
                ),
                path_line_location(&module.report.path, operation.line),
                source_line(&module.source, operation.line),
                "route Tokio spawn/blocking/runtime construction through a typed runtime facade with task tracking, shutdown, cancellation, and observability",
            ))
        })
        .collect()
}

fn is_runtime_owner_context(path: &str, function_name: &str) -> bool {
    let normalized_path = path.replace('\\', "/").to_ascii_lowercase();
    let normalized_function = function_name.to_ascii_lowercase();
    (normalized_path.contains("/runtime/")
        || normalized_path.contains("tokio_runtime")
        || normalized_path.contains("runtime.rs"))
        && (normalized_function.contains("spawn")
            || normalized_function.contains("runtime")
            || normalized_function.contains("blocking")
            || normalized_function.contains("task"))
}
