//! Agent policy rules derived from parser-owned function algorithm facts.

use std::collections::BTreeMap;

use crate::parser::{
    ParsedRustModule, RustFunctionControlFlowSyntax, path_line_location, source_line,
};
use crate::{AspRustFinding, AspRustRule};

use crate::rules::display_path;

use super::{
    RUST_AGENT_POLICY_ASYNC_BACKPRESSURE_BOUNDARY_V1, RUST_AGENT_POLICY_ASYNC_BLOCKING_BOUNDARY_V1,
    RUST_AGENT_POLICY_ASYNC_SELECT_CANCEL_SAFETY_V1, RUST_AGENT_POLICY_ASYNC_SYNC_LOCK_BOUNDARY_V1,
    RUST_AGENT_POLICY_ASYNC_TASK_LIFECYCLE_V1, RUST_AGENT_POLICY_ASYNC_TIMEOUT_CANCEL_SAFETY_V1,
    RUST_AGENT_POLICY_CFG_IMPL_NESTED_TRAVERSAL_V1, RUST_AGENT_POLICY_CFG_PUBLIC_BROAD_SURFACE_V1,
    RUST_AGENT_POLICY_CFG_PUBLIC_NESTED_FLOW_V1, RUST_AGENT_POLICY_DATA_LINEAR_MEMBERSHIP_SCAN_V1,
    RUST_AGENT_POLICY_ITER_IMPL_MANUAL_TRANSFORM_V1,
    RUST_AGENT_POLICY_ITER_PUBLIC_MANUAL_TRANSFORM_V1,
};

const MAX_LINEAR_NESTING_DEPTH: usize = 2;
const MAX_NATIVE_IDIOM_NESTING_DEPTH: usize = 2;
const MIN_BROAD_FUNCTION_LINES: usize = 72;
const MIN_BROAD_FUNCTION_STATEMENTS: usize = 22;
const MIN_LINEAR_BLOCK_STATEMENTS: usize = 14;
const MIN_INTERNAL_TRAVERSAL_NESTING_DEPTH: usize = 4;
const MIN_INTERNAL_TRAVERSAL_LOOP_NESTING_DEPTH: usize = 2;
const MIN_INTERNAL_TRAVERSAL_BRANCH_COUNT: usize = 2;
const SOFTWARE_CRITERIA_LABEL: &str = "softwareCriteria";
const SOFTWARE_CRITERION_COMPACT_PREFIX: &str = "software-criterion/";
const CONTROL_FLOW_BROAD_LINEAR_PHASE: &str = "control-flow.broad-linear-phase";
const CONTROL_FLOW_DECISION_STACK: &str = "control-flow.decision-stack";
const CONTROL_FLOW_LITERAL_DISPATCH_CHAIN: &str = "control-flow.literal-dispatch-chain";
const CONTROL_FLOW_TRAVERSAL_KNOT: &str = "control-flow.traversal-knot";
const DATA_STRUCTURE_LINEAR_MEMBERSHIP_SCAN: &str = "data-structure.linear-membership-scan";
const NATIVE_IDIOM_MANUAL_TRANSFORM_LOOP: &str = "native-idiom.manual-transform-loop";
const ASYNC_BLOCKING_BOUNDARY: &str = "async.blocking-boundary";
const ASYNC_SYNC_LOCK_ACROSS_AWAIT: &str = "async.sync-lock-across-await";
const ASYNC_UNBOUNDED_QUEUE_BACKPRESSURE: &str = "async.unbounded-queue-backpressure";
const ASYNC_SELECT_CANCELLATION_SAFETY: &str = "async.select-cancellation-safety";
const ASYNC_TIMEOUT_CANCELLATION_SAFETY: &str = "async.timeout-cancellation-safety";
const ASYNC_TASK_LIFECYCLE_BOUNDARY: &str = "async.task-lifecycle-boundary";

fn compact_software_criteria(criterion_ids: &[&'static str]) -> String {
    criterion_ids
        .iter()
        .map(|criterion_id| compact_software_criterion(criterion_id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn compact_software_criterion(criterion_id: &str) -> String {
    if criterion_id.starts_with(SOFTWARE_CRITERION_COMPACT_PREFIX) {
        return criterion_id.to_string();
    }
    format!("{SOFTWARE_CRITERION_COMPACT_PREFIX}{criterion_id}")
}

pub(super) fn algorithm_shape_findings(
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    modules
        .iter()
        .flat_map(|module| {
            let mut findings = Vec::new();
            findings.extend(nested_algorithm_findings(module, rules));
            findings.extend(broad_linear_algorithm_findings(module, rules));
            findings.extend(implementation_traversal_findings(module, rules));
            findings.extend(implementation_iterator_idiom_findings(module, rules));
            findings.extend(linear_membership_scan_findings(module, rules));
            findings.extend(async_blocking_boundary_findings(module, rules));
            findings.extend(async_sync_lock_across_await_findings(module, rules));
            findings.extend(async_unbounded_queue_backpressure_findings(module, rules));
            findings.extend(async_select_cancellation_safety_findings(module, rules));
            findings.extend(async_timeout_cancellation_safety_findings(module, rules));
            findings.extend(async_task_lifecycle_boundary_findings(module, rules));
            findings
        })
        .collect()
}

pub(super) fn native_iterator_idiom_findings(
    modules: &[&ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_ITER_PUBLIC_MANUAL_TRANSFORM_V1];
    modules
        .iter()
        .flat_map(|module| {
            module
                .syntax_facts
                .public_function_control_flows
                .iter()
                .filter(|control_flow| !control_flow.is_test_context)
                .filter_map(|control_flow| {
            let profile = native_iterator_idiom_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
                    Some(with_software_criteria(
                        AspRustFinding::from_rule(
                            rule,
                            format!(
                                "{} public function `{}` manually spells iterator boilerplate. Criteria: {}.",
                                display_path(&module.report.path),
                                control_flow.function_name,
                                NATIVE_IDIOM_MANUAL_TRANSFORM_LOOP
                            ),
                            path_line_location(&module.report.path, control_flow.line),
                            source_line(&module.source, control_flow.line),
                            "replace this boilerplate loop with a Rust iterator idiom",
                        ),
                        &[NATIVE_IDIOM_MANUAL_TRANSFORM_LOOP],
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn nested_algorithm_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_CFG_PUBLIC_NESTED_FLOW_V1];
    module
        .syntax_facts
        .public_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter_map(|control_flow| {
            let profile = nested_algorithm_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
            let criterion_ids = nested_algorithm_software_criteria(&profile);
            Some(with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} public function `{}` hides algorithm shape. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        compact_software_criteria(&criterion_ids)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "make this public algorithm shape explicit",
                ),
                &criterion_ids,
            ))
        })
        .collect()
}

fn broad_linear_algorithm_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_CFG_PUBLIC_BROAD_SURFACE_V1];
    module
        .syntax_facts
        .public_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter_map(|control_flow| {
            let profile = broad_linear_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
            Some(with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} public function `{}` spans {} lines with {} statements and a {}-statement block. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        control_flow.line_span,
                        control_flow.statement_count,
                        control_flow.max_block_statement_count,
                        compact_software_criterion(CONTROL_FLOW_BROAD_LINEAR_PHASE)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "split this broad function into named algorithm steps",
                ),
                &[CONTROL_FLOW_BROAD_LINEAR_PHASE],
            ))
        })
        .collect()
}

fn implementation_traversal_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_CFG_IMPL_NESTED_TRAVERSAL_V1];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_public)
        .filter(|control_flow| !control_flow.is_test_context)
        .filter_map(|control_flow| {
            let profile = implementation_traversal_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
            Some(with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} implementation function `{}` nests traversal scaffolding. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        compact_software_criterion(CONTROL_FLOW_TRAVERSAL_KNOT)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "extract this traversal into named iterator, predicate, or receipt-processing helpers",
                ),
                &[CONTROL_FLOW_TRAVERSAL_KNOT],
            ))
        })
        .collect()
}

fn implementation_iterator_idiom_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_ITER_IMPL_MANUAL_TRANSFORM_V1];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_public)
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| implementation_traversal_profile(control_flow).is_empty())
        .filter_map(|control_flow| {
            let profile = native_iterator_idiom_profile(control_flow);
            if profile.is_empty() {
                return None;
            }
            Some(with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} implementation function `{}` manually spells iterator boilerplate. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        compact_software_criterion(NATIVE_IDIOM_MANUAL_TRANSFORM_LOOP)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "extract this loop into Rust iterator adapters or a named helper",
                ),
                &[NATIVE_IDIOM_MANUAL_TRANSFORM_LOOP],
            ))
        })
        .collect()
}

fn linear_membership_scan_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_DATA_LINEAR_MEMBERSHIP_SCAN_V1];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| control_flow.linear_membership_scan_loop_count > 0)
        .map(|control_flow| {
            with_software_criteria(
            AspRustFinding::from_rule(
                rule,
                format!(
                    "{} function `{}` performs {} loop-local linear membership scan(s). Criteria: {}.",
                    display_path(&module.report.path),
                    control_flow.function_name,
                    control_flow.linear_membership_scan_loop_count,
                    compact_software_criterion(DATA_STRUCTURE_LINEAR_MEMBERSHIP_SCAN)
                ),
                path_line_location(&module.report.path, control_flow.line),
                source_line(&module.source, control_flow.line),
                "build a set or map index before the loop, or document why this scan is bounded",
            ),
            &[DATA_STRUCTURE_LINEAR_MEMBERSHIP_SCAN],
        )
        })
        .collect()
}

fn async_blocking_boundary_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_ASYNC_BLOCKING_BOUNDARY_V1];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| control_flow.is_async)
        .filter(|control_flow| control_flow.blocking_call_count > 0)
        .map(|control_flow| {
            with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} async function `{}` performs {} blocking call(s) without an explicit runtime boundary. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        control_flow.blocking_call_count,
                        compact_software_criterion(ASYNC_BLOCKING_BOUNDARY)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "move blocking work behind spawn_blocking, block_in_place, or a dedicated synchronous boundary",
                ),
                &[ASYNC_BLOCKING_BOUNDARY],
            )
        })
        .collect()
}

fn async_sync_lock_across_await_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_ASYNC_SYNC_LOCK_BOUNDARY_V1];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| control_flow.is_async)
        .filter(|control_flow| control_flow.sync_lock_guard_across_await_count > 0)
        .map(|control_flow| {
            with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} async function `{}` holds {} sync lock guard(s) across `.await`. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        control_flow.sync_lock_guard_across_await_count,
                        compact_software_criterion(ASYNC_SYNC_LOCK_ACROSS_AWAIT)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "drop sync lock guards before `.await`, use `tokio::sync` primitives, or move the critical section behind a synchronous boundary",
                ),
                &[ASYNC_SYNC_LOCK_ACROSS_AWAIT],
            )
        })
        .collect()
}

fn async_unbounded_queue_backpressure_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_ASYNC_BACKPRESSURE_BOUNDARY_V1];
    let has_backpressure_boundary = module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .any(|control_flow| control_flow.backpressure_boundary_signal_count > 0);
    if has_backpressure_boundary {
        return Vec::new();
    }
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| control_flow.unbounded_async_queue_call_count > 0)
        .map(|control_flow| {
            with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} function `{}` creates {} unbounded async queue(s) without a readiness or capacity boundary. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        control_flow.unbounded_async_queue_call_count,
                        compact_software_criterion(ASYNC_UNBOUNDED_QUEUE_BACKPRESSURE)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "use a bounded channel or add an explicit poll_ready, try_send, reserve, or semaphore backpressure boundary",
                ),
                &[ASYNC_UNBOUNDED_QUEUE_BACKPRESSURE],
            )
        })
        .collect()
}

fn async_select_cancellation_safety_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_ASYNC_SELECT_CANCEL_SAFETY_V1];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| control_flow.tokio_select_cancel_unsafe_io_count > 0)
        .map(|control_flow| {
            with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} function `{}` places {} cancellation-unsafe I/O future(s) inside `tokio::select!`. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        control_flow.tokio_select_cancel_unsafe_io_count,
                        compact_software_criterion(ASYNC_SELECT_CANCELLATION_SAFETY)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "split exact read/write progress out of the select branch or use cancellation-safe read/write polling",
                ),
                &[ASYNC_SELECT_CANCELLATION_SAFETY],
            )
        })
        .collect()
}

fn async_timeout_cancellation_safety_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_ASYNC_TIMEOUT_CANCEL_SAFETY_V1];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| control_flow.tokio_timeout_cancel_unsafe_io_count > 0)
        .map(|control_flow| {
            with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} function `{}` wraps {} cancellation-unsafe I/O future(s) in `tokio::time::timeout`. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        control_flow.tokio_timeout_cancel_unsafe_io_count,
                        compact_software_criterion(ASYNC_TIMEOUT_CANCELLATION_SAFETY)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "move exact read/write progress outside the timeout future or use a cancellation-safe polling loop with explicit partial-progress state",
                ),
                &[ASYNC_TIMEOUT_CANCELLATION_SAFETY],
            )
        })
        .collect()
}

fn async_task_lifecycle_boundary_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_ASYNC_TASK_LIFECYCLE_V1];
    module
        .syntax_facts
        .all_function_control_flows
        .iter()
        .filter(|control_flow| !control_flow.is_test_context)
        .filter(|control_flow| control_flow.discarded_tokio_spawn_count > 0)
        .map(|control_flow| {
            with_software_criteria(
                AspRustFinding::from_rule(
                    rule,
                    format!(
                        "{} function `{}` discards {} Tokio task handle(s) without a lifecycle boundary. Criteria: {}.",
                        display_path(&module.report.path),
                        control_flow.function_name,
                        control_flow.discarded_tokio_spawn_count,
                        compact_software_criterion(ASYNC_TASK_LIFECYCLE_BOUNDARY)
                    ),
                    path_line_location(&module.report.path, control_flow.line),
                    source_line(&module.source, control_flow.line),
                    "return, store, await, abort, or supervise the JoinHandle, or isolate the task behind an explicit detached-task boundary",
                ),
                &[ASYNC_TASK_LIFECYCLE_BOUNDARY],
            )
        })
        .collect()
}

fn with_software_criteria(
    mut finding: AspRustFinding,
    criterion_ids: &[&'static str],
) -> AspRustFinding {
    finding
        .labels
        .insert(SOFTWARE_CRITERIA_LABEL.to_string(), criterion_ids.join(","));
    finding
}

fn nested_algorithm_software_criteria(profile: &[&str]) -> Vec<&'static str> {
    let mut criteria = Vec::new();
    for indicator in profile {
        match *indicator {
            "deep control-flow nesting" | "large branch surface without match" => {
                push_signal(&mut criteria, CONTROL_FLOW_DECISION_STACK);
            }
            "nested loops mixed with branches" => {
                push_signal(&mut criteria, CONTROL_FLOW_TRAVERSAL_KNOT);
            }
            "literal dispatch chain without match" => {
                push_signal(&mut criteria, CONTROL_FLOW_LITERAL_DISPATCH_CHAIN);
            }
            _ => {}
        }
    }
    criteria
}

fn push_signal(criteria: &mut Vec<&'static str>, signal: &'static str) {
    if !criteria.contains(&signal) {
        criteria.push(signal);
    }
}

fn nested_algorithm_profile(control_flow: &RustFunctionControlFlowSyntax) -> Vec<&'static str> {
    let mut criteria = Vec::new();
    if control_flow.max_nesting_depth >= 4 {
        criteria.push("deep control-flow nesting");
    }
    if control_flow.max_loop_nesting_depth >= 2 && control_flow.branch_count >= 3 {
        criteria.push("nested loops mixed with branches");
    }
    if control_flow.literal_dispatch_chain_count > 0 {
        criteria.push("literal dispatch chain without match");
    }
    if control_flow.branch_count >= 8 && control_flow.match_count == 0 {
        criteria.push("large branch surface without match");
    }
    criteria
}

fn implementation_traversal_profile(
    control_flow: &RustFunctionControlFlowSyntax,
) -> Vec<&'static str> {
    let mut criteria = Vec::new();
    if control_flow.max_nesting_depth >= MIN_INTERNAL_TRAVERSAL_NESTING_DEPTH
        && control_flow.max_loop_nesting_depth >= MIN_INTERNAL_TRAVERSAL_LOOP_NESTING_DEPTH
        && control_flow.branch_count >= MIN_INTERNAL_TRAVERSAL_BRANCH_COUNT
    {
        criteria.push("nested loops guarded by branches");
    }
    if control_flow.repeated_iterator_source_loop_count > 0 && control_flow.branch_count >= 2 {
        criteria.push("repeated guarded scans over one source");
    }
    criteria
}

fn broad_linear_profile(control_flow: &RustFunctionControlFlowSyntax) -> Vec<&'static str> {
    if control_flow.max_nesting_depth > MAX_LINEAR_NESTING_DEPTH {
        return Vec::new();
    }
    let mut criteria = Vec::new();
    if control_flow.line_span >= MIN_BROAD_FUNCTION_LINES
        && control_flow.statement_count >= MIN_BROAD_FUNCTION_STATEMENTS
    {
        criteria.push("long public function body");
    }
    if control_flow.max_block_statement_count >= MIN_LINEAR_BLOCK_STATEMENTS {
        criteria.push("large linear statement block");
    }
    criteria
}

fn native_iterator_idiom_profile(
    control_flow: &RustFunctionControlFlowSyntax,
) -> Vec<&'static str> {
    if control_flow.max_nesting_depth > MAX_NATIVE_IDIOM_NESTING_DEPTH {
        return Vec::new();
    }
    let mut criteria = Vec::new();
    if control_flow.manual_collection_loop_count > 0 {
        criteria.push("manual collection accumulator loop");
    }
    if control_flow.manual_predicate_loop_count > 0 {
        criteria.push("manual predicate loop");
    }
    if control_flow.manual_count_loop_count > 0 {
        criteria.push("manual count loop");
    }
    if control_flow.manual_numeric_accumulator_loop_count > 0 {
        criteria.push("manual numeric accumulator loop");
    }
    if control_flow.repeated_iterator_source_loop_count > 0 {
        criteria.push("repeated pass over the same iterator source");
    }
    criteria
}
