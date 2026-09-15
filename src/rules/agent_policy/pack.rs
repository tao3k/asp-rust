//! Agent-oriented Rust policy rule catalog and evaluator.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, rust_reasoning_tree_facts};
use crate::{AspRustFinding, AspRustRule, AspRustScope, RustDiagnosticSeverity};

use super::{
    algorithm_shape, api_shape, data_shape, dependency_graph, native_abi, process_command,
    source_surface, tokio_runtime,
};
use crate::rules::labels;

const PACK_ID: &str = "rust.agent_policy";
pub(super) const RUST_AGENT_POLICY_DOCS_MODULE_INTENT_V1: &str = "RUST-AGENT-DOCS-MODULE-001";
pub(super) const RUST_AGENT_POLICY_DOCS_PUBLIC_ITEM_V1: &str = "RUST-AGENT-DOCS-PUBLIC-002";
pub(super) const RUST_AGENT_POLICY_SOURCE_NAMESPACE_REPEAT_V1: &str =
    "RUST-AGENT-SOURCE-NAMESPACE-003";
pub(super) const RUST_AGENT_POLICY_API_PUBLIC_NAME_CONFLICT_V1: &str = "RUST-AGENT-API-NAME-004";
pub(super) const RUST_AGENT_POLICY_API_FACADE_EXPORT_GROUPS_V1: &str = "RUST-AGENT-API-FACADE-005";
pub(super) const RUST_AGENT_POLICY_SOURCE_PUBLIC_MODULE_NAME_V1: &str =
    "RUST-AGENT-SOURCE-MODULE-006";
pub(super) const RUST_AGENT_POLICY_SOURCE_MODULE_PATH_NAME_V1: &str = "RUST-AGENT-SOURCE-PATH-007";
pub(super) const RUST_AGENT_POLICY_DOCS_BRANCH_INTENT_V1: &str = "RUST-AGENT-DOCS-BRANCH-008";
pub(super) const RUST_AGENT_POLICY_OWNER_DEPENDENCY_CYCLE_V1: &str = "RUST-AGENT-OWNER-GRAPH-009";
pub(super) const RUST_AGENT_POLICY_OWNER_LEAF_IMPORT_V1: &str = "RUST-AGENT-OWNER-BOUNDARY-010";
pub(super) const RUST_AGENT_POLICY_DOCS_OWNER_FAN_OUT_V1: &str = "RUST-AGENT-DOCS-OWNER-FANOUT-011";
pub(super) const RUST_AGENT_POLICY_API_SEMANTIC_IDENTIFIER_TYPE_V1: &str =
    "RUST-AGENT-API-TYPE-012";
pub(super) const RUST_AGENT_POLICY_API_ERROR_BOUNDARY_V1: &str = "RUST-AGENT-API-ERROR-013";
pub(super) const RUST_AGENT_POLICY_TEST_SUPPORT_REEXPORT_V1: &str = "RUST-AGENT-TEST-SUPPORT-014";
pub(super) const RUST_AGENT_POLICY_CFG_PUBLIC_NESTED_FLOW_V1: &str = "RUST-AGENT-CFG-PUBLIC-015";
pub(super) const RUST_AGENT_POLICY_CFG_PUBLIC_BROAD_SURFACE_V1: &str = "RUST-AGENT-CFG-PUBLIC-016";
pub(super) const RUST_AGENT_POLICY_ITER_PUBLIC_MANUAL_TRANSFORM_V1: &str =
    "RUST-AGENT-ITER-PUBLIC-017";
pub(super) const RUST_AGENT_POLICY_API_FLAG_PARAMETER_SURFACE_V1: &str = "RUST-AGENT-API-FLAGS-018";
pub(super) const RUST_AGENT_POLICY_API_POSITIONAL_PARAMETER_SURFACE_V1: &str =
    "RUST-AGENT-API-PARAMETERS-019";
pub(super) const RUST_AGENT_POLICY_DATA_PRIMITIVE_FIELD_V1: &str = "RUST-AGENT-DATA-FIELD-020";
pub(super) const RUST_AGENT_POLICY_DATA_ENUM_PRIMITIVE_PAYLOAD_V1: &str =
    "RUST-AGENT-DATA-ENUM-PAYLOAD-021";
pub(super) const RUST_AGENT_POLICY_DATA_DERIVABLE_BOUNDS_V1: &str = "RUST-AGENT-DATA-BOUNDS-022";
pub(super) const RUST_AGENT_POLICY_PUBLIC_TUPLE_API_SURFACE_V1: &str = "RUST-AGENT-API-SHAPE-023";
pub(super) const RUST_AGENT_POLICY_DATA_ENUM_TUPLE_PAYLOAD_V1: &str =
    "RUST-AGENT-DATA-ENUM-TUPLE-024";
pub(super) const RUST_AGENT_POLICY_CFG_IMPL_NESTED_TRAVERSAL_V1: &str = "RUST-AGENT-CFG-IMPL-025";
pub(super) const RUST_AGENT_POLICY_ITER_IMPL_MANUAL_TRANSFORM_V1: &str = "RUST-AGENT-ITER-IMPL-026";
pub(super) const RUST_AGENT_POLICY_API_PRIMITIVE_TYPE_ALIAS_V1: &str =
    "RUST-AGENT-API-TYPE-ALIAS-027";
pub(super) const RUST_AGENT_POLICY_DATA_STRINGLY_STATE_FIELD_V1: &str = "RUST-AGENT-DATA-STATE-028";
pub(super) const RUST_AGENT_POLICY_DATA_LINEAR_MEMBERSHIP_SCAN_V1: &str =
    "RUST-AGENT-DATA-MEMBERSHIP-029";
pub(super) const RUST_AGENT_POLICY_ASYNC_BLOCKING_BOUNDARY_V1: &str =
    "RUST-AGENT-ASYNC-BLOCKING-030";
pub(super) const RUST_AGENT_POLICY_ASYNC_SYNC_LOCK_BOUNDARY_V1: &str =
    "RUST-AGENT-ASYNC-SYNC-LOCK-031";
pub(super) const RUST_AGENT_POLICY_ASYNC_BACKPRESSURE_BOUNDARY_V1: &str =
    "RUST-AGENT-ASYNC-BACKPRESSURE-032";
pub(super) const RUST_AGENT_POLICY_ASYNC_SELECT_CANCEL_SAFETY_V1: &str =
    "RUST-AGENT-ASYNC-CANCEL-SAFETY-033";
pub(super) const RUST_AGENT_POLICY_ASYNC_TIMEOUT_CANCEL_SAFETY_V1: &str =
    "RUST-AGENT-ASYNC-CANCEL-SAFETY-034";
pub(super) const RUST_AGENT_POLICY_ASYNC_TASK_LIFECYCLE_V1: &str =
    "RUST-AGENT-ASYNC-TASK-LIFECYCLE-001";
pub(super) const RUST_AGENT_POLICY_PUBLIC_DYNAMIC_JSON_API_BOUNDARY_V1: &str =
    "RUST-AGENT-API-SHAPE-036";
pub(super) const RUST_AGENT_POLICY_PROCESS_COMMAND_PROBE_V1: &str = "RUST-AGENT-PROC-001";
pub(super) const RUST_AGENT_POLICY_TOKIO_RUNTIME_BOUNDARY_V1: &str = "RUST-AGENT-TOKIO-RUNTIME-002";
pub(super) const RUST_AGENT_POLICY_NATIVE_ABI_CONTRACT_V1: &str = "RUST-AGENT-NATIVE-ABI-001";

/// Scenario coverage required for an agent policy rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RustPolicyScenarioRequirement {
    /// Agent policy rule id that must stay scenario-backed.
    pub rule_id: &'static str,
    /// Stable scenario id expected in `scenario.toml`.
    pub scenario_id: &'static str,
    /// Policy id expected in `scenario.toml` `policy_ids`.
    pub policy_id: &'static str,
    /// Crate-relative scenario root.
    pub scenario_root: &'static str,
}

/// Return compact metadata for agent-oriented Rust policy rules.
#[must_use]
pub fn rust_agent_policy_rules() -> Vec<AspRustRule> {
    rules_by_id().into_values().collect()
}

/// Return Rust policy rules that require scenario benchmark coverage.
#[must_use]
pub(crate) fn rust_agent_policy_scenario_requirements() -> &'static [RustPolicyScenarioRequirement]
{
    super::scenario_requirements::rust_agent_policy_scenario_requirements()
}

pub(crate) fn evaluate(
    scope: Option<&AspRustScope>,
    modules: &[ParsedRustModule],
) -> Vec<AspRustFinding> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let rules = rules_by_id();
    let mut findings = Vec::new();
    let reasoning_tree = rust_reasoning_tree_facts(scope, modules);
    let module_by_path = modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let source_modules = modules
        .iter()
        .filter(|module| {
            reasoning_tree
                .module(&module.report.path)
                .is_some_and(|module_facts| module_facts.is_source_module)
        })
        .collect::<Vec<_>>();
    for module in modules {
        if !module.report.is_valid {
            continue;
        };
        findings.extend(process_command::process_command_findings(module, &rules));
    }
    for module in &source_modules {
        if !module.report.is_valid {
            continue;
        };
        findings.extend(source_surface::source_module_findings(
            &reasoning_tree,
            module,
            &rules,
        ));
        findings.extend(data_shape::data_shape_findings(module, &rules));
        findings.extend(api_shape::api_shape_findings(module, &rules));
        findings.extend(tokio_runtime::tokio_runtime_boundary_findings(
            module, &rules,
        ));
        findings.extend(native_abi::native_abi_contract_findings(module, &rules));
    }
    findings.extend(source_surface::repeated_namespace_findings(
        &reasoning_tree,
        modules,
        &rules,
    ));
    findings.extend(source_surface::generic_module_path_findings(
        &reasoning_tree,
        &source_modules,
        &rules,
    ));
    findings.extend(source_surface::public_name_conflict_findings(
        &source_modules,
        &rules,
    ));
    findings.extend(source_surface::test_support_reexport_findings(
        &reasoning_tree,
        modules,
        &rules,
    ));
    findings.extend(algorithm_shape::algorithm_shape_findings(
        &source_modules,
        &rules,
    ));
    findings.extend(algorithm_shape::native_iterator_idiom_findings(
        &source_modules,
        &rules,
    ));
    findings.extend(dependency_graph::dependency_graph_findings(
        &reasoning_tree,
        &module_by_path,
        &rules,
    ));
    findings
}

fn rules_by_id() -> BTreeMap<&'static str, AspRustRule> {
    [
        AspRustRule::new(
            RUST_AGENT_POLICY_DOCS_MODULE_INTENT_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public module lacks an intent doc",
            "Add a concise `//!` module-level intent doc using `clippy::doc_markdown` style, with technical identifiers in backticks.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DOCS_PUBLIC_ITEM_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public item lacks a doc comment",
            "Document public Rust boundaries using `clippy::doc_markdown` style so agents can reason from native syntax without guessing intent.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_SOURCE_NAMESPACE_REPEAT_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Namespace path repeats a segment",
            "Keep Rust module namespaces branch-unique, including file stems; rename repeated path segments so agents see one clear ownership path.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_API_PUBLIC_NAME_CONFLICT_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public item name conflicts across namespaces",
            "Give project-level public items unambiguous names or move them behind a clear domain namespace.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_API_FACADE_EXPORT_GROUPS_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Facade exports too many public groups",
            "Keep facade exports grouped by owner so agents can identify the right repair surface quickly.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_SOURCE_PUBLIC_MODULE_NAME_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public module name is generic",
            "Name public Rust modules after the domain they own; avoid generic buckets such as utils, common, helpers, or shared.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_SOURCE_MODULE_PATH_NAME_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Module path segment is generic",
            "Avoid generic Rust module file or directory names in source roots; name paths after the owner responsibility.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DOCS_BRANCH_INTENT_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Branch module lacks reasoning-tree intent doc",
            "Document source modules that own multiple resolved child edges with a `//!` intent doc in `clippy::doc_markdown` style.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_OWNER_DEPENDENCY_CYCLE_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Owner dependency cycle crosses branches",
            "Keep owner dependency edges acyclic so agents can follow the reasoning tree without circular repair ownership.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_OWNER_LEAF_IMPORT_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Owner branch imports another owner leaf",
            "Depend on another owner through its branch boundary instead of importing leaf implementation modules directly.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DOCS_OWNER_FAN_OUT_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Owner fan-out lacks intent doc",
            "Document branch modules that coordinate several owner dependencies with a `//!` intent doc in `clippy::doc_markdown` style.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_API_SEMANTIC_IDENTIFIER_TYPE_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public semantic identifier uses a primitive type",
            "Give public semantic identifiers a named domain type so agents can preserve invariants without guessing from parameter names.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_API_ERROR_BOUNDARY_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public error boundary uses an application error type",
            "Keep public library error boundaries typed so agents and callers can handle recovery without inspecting application error context.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_TEST_SUPPORT_REEXPORT_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Test support re-export is unused",
            "Keep test support facades narrow; re-export only names consumed through the same support surface or used by support helpers.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_CFG_PUBLIC_NESTED_FLOW_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function hides algorithm behind nested control flow",
            "Expose public Rust algorithm shape through guard clauses, `match`, typed dispatch, or small named pipeline steps so agents can reason about the branch structure before editing.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_CFG_PUBLIC_BROAD_SURFACE_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function owns a broad linear algorithm surface",
            "Split broad public Rust functions into small named helpers or pipeline steps so agents can edit one algorithm responsibility at a time.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_ITER_PUBLIC_MANUAL_TRANSFORM_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function manually spells an iterator transform",
            "Use Rust iterator adapters and consumers such as `map`, `filter`, `filter_map`, `collect`, `sum`, `count`, `any`, `all`, or a named iterator pipeline helper when loops only map, filter, collect, count, sum, answer a predicate, or repeatedly scan the same iterator source.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_API_FLAG_PARAMETER_SURFACE_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function exposes multiple flag parameters",
            "Replace multiple public `bool` or `Option<bool>` parameters with a named enum, newtype, or config struct so agents can preserve mode semantics without reading every branch.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_API_POSITIONAL_PARAMETER_SURFACE_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public function exposes a broad positional parameter surface",
            "Replace broad public positional parameter lists with a named config, request type, or builder surface so agents can preserve constructor semantics without re-reading every call site.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DATA_PRIMITIVE_FIELD_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public data struct exposes primitive semantic fields",
            "Wrap repeated public semantic primitive fields in named domain types so agents preserve data invariants instead of extending stringly typed state.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DATA_ENUM_PRIMITIVE_PAYLOAD_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public enum variant exposes primitive semantic payload fields",
            "Move broad public enum variant payloads into named domain types or a named payload struct so agents preserve event and command invariants instead of extending raw primitive state.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DATA_DERIVABLE_BOUNDS_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public generic data type carries duplicated derivable bounds",
            "Move derivable or formatting trait bounds off public data type definitions and onto the impl or methods that need them so agents do not over-constrain the API contract.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_PUBLIC_TUPLE_API_SURFACE_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public API exposes an anonymous primitive tuple",
            "Replace public tuple parameter or return bundles of primitive semantic values with named structs, enums, or newtypes so agents can preserve field intent.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DATA_ENUM_TUPLE_PAYLOAD_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public enum tuple variant exposes anonymous primitive payload",
            "Replace public enum tuple variant payloads that bundle primitive semantic values with named fields, named payload structs, or domain newtypes so agents preserve event and command intent.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_CFG_IMPL_NESTED_TRAVERSAL_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Implementation function nests traversal scaffolding",
            "Extract nested internal traversals into named iterator, predicate, or receipt-processing helpers so agents can see the algorithm boundary instead of extending raw loop and guard scaffolding.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_ITER_IMPL_MANUAL_TRANSFORM_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Implementation function manually spells an iterator transform",
            "Use Rust iterator adapters or a named iterator helper when internal loops only map, filter, collect, count, sum, answer a predicate, or repeatedly scan the same iterator source.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_API_PRIMITIVE_TYPE_ALIAS_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public semantic type alias hides a primitive carrier",
            "Use a public newtype or named struct instead of a primitive type alias for semantic identifiers, tokens, paths, durations, byte sizes, or flags so agents preserve invariants across call sites.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DATA_STRINGLY_STATE_FIELD_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public data model exposes a stringly state field",
            "Use a public enum, newtype, or typed catalog boundary instead of `String` or `Option<String>` for public state, status, kind, mode, phase, type, tag, or category fields.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_DATA_LINEAR_MEMBERSHIP_SCAN_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Function performs a linear membership scan inside a loop",
            "Build a `HashSet`, `BTreeSet`, `HashMap`, or `BTreeMap` index before the loop, or document why the nested linear scan is bounded, so agents preserve the algorithmic complexity contract.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_ASYNC_BLOCKING_BOUNDARY_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Async task performs blocking work on the runtime",
            "Move blocking I/O, sleeps, or CPU-heavy loops behind `spawn_blocking`, a dedicated worker, or an explicit sync boundary so agents preserve Tokio runtime responsiveness.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_ASYNC_SYNC_LOCK_BOUNDARY_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Async task holds a sync lock guard across await",
            "Drop `std::sync` or `parking_lot` lock guards before `.await`, switch the boundary to `tokio::sync`, or isolate the critical section so agents preserve runtime progress and cancellation behavior.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_ASYNC_BACKPRESSURE_BOUNDARY_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Async queue lacks a backpressure boundary",
            "Use a bounded channel, expose `poll_ready`/`try_send`/`reserve`, or guard an unbounded channel with an explicit capacity or semaphore boundary so agents preserve async backpressure and memory behavior.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_ASYNC_SELECT_CANCEL_SAFETY_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Tokio select branch uses cancellation-unsafe I/O",
            "Keep `tokio::select!` branches cancellation-safe; avoid `read_exact`, `read_to_end`, `read_to_string`, `write_all`, or `write_all_buf` inside select branches unless the partial-progress contract is explicit.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_ASYNC_TIMEOUT_CANCEL_SAFETY_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Tokio timeout wraps cancellation-unsafe I/O",
            "Keep `tokio::time::timeout` boundaries cancellation-safe; avoid wrapping `read_exact`, `read_to_end`, `read_to_string`, `write_all`, or `write_all_buf` unless partial progress is owned outside the timed future.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_ASYNC_TASK_LIFECYCLE_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Tokio task handle is discarded",
            "Keep spawned Tokio tasks behind an explicit lifecycle contract: return, store, await, abort, or supervise the `JoinHandle`, or isolate intentionally detached work behind a named boundary.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_PUBLIC_DYNAMIC_JSON_API_BOUNDARY_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Public API exposes dynamic JSON",
            "Replace public `serde_json::Value` parameters or returns with named request, response, enum, or documented boundary types so agents preserve payload contracts without re-reading untyped JSON shape.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_PROCESS_COMMAND_PROBE_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Process command probe mixes execution and receipt ownership",
            "Keep process command probes split into command construction, execution, environment/path shaping, and typed receipt parsing owners so agents can edit runtime behavior without coupling all effects.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_TOKIO_RUNTIME_BOUNDARY_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Tokio runtime operation bypasses a runtime owner",
            "Route Tokio spawn, blocking work, and runtime construction through a typed runtime facade so agents preserve task tracking, shutdown, cancellation, thread model, and observability behavior.",
            labels("agent-policy"),
        ),
        AspRustRule::new(
            RUST_AGENT_POLICY_NATIVE_ABI_CONTRACT_V1,
            PACK_ID,
            RustDiagnosticSeverity::Info,
            "Native ABI layout lacks a co-located contract",
            "Keep public `repr(C)` native ABI layouts beside ABI version, ABI id, header path, and header source constants so agents preserve Rust, C, and projection compatibility.",
            labels("agent-policy"),
        ),
    ]
    .into_iter()
    .map(|rule| (rule.rule_id, rule))
    .collect()
}
