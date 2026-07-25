//! Command-line execution for the Rust project harness binary.

mod agent_registry;
mod ast_patch;
mod behavior_snapshot;
mod determinism_readiness;
mod dev_command_log;
mod evidence_graph;
mod execution_receipt;
mod flow_lite_query;
mod formal_proof_pilot;
#[cfg(feature = "search")]
mod language_projection;
mod query;
mod query_options;
mod review_packet;
mod runner;
mod runner_support;
#[cfg(feature = "search")]
mod search_output;
#[cfg(feature = "search")]
mod search_plan;
#[cfg(feature = "search")]
mod search_trace;
#[cfg(feature = "search")]
mod semantic_search_json;
#[cfg(feature = "search")]
mod semantic_search_json_canonical;
#[cfg(feature = "search")]
mod semantic_search_json_fields;
#[cfg(feature = "search")]
mod semantic_search_synthesis_json;
#[cfg(feature = "search")]
mod semantic_syntax_refs;

pub(in crate::cli) use agent_registry::print_agent_registry;
pub(in crate::cli) use ast_patch::run_ast_patch;
pub(in crate::cli) use behavior_snapshot::run_behavior;
pub(in crate::cli) use determinism_readiness::run_determinism;
pub(in crate::cli) use dev_command_log::DevCommandLog;
pub(in crate::cli) use evidence_graph::run_evidence;
pub(in crate::cli) use execution_receipt::run_receipt;
pub(in crate::cli) use flow_lite_query::run_flow_lite_query_catalog;
pub(in crate::cli) use formal_proof_pilot::run_proof;
#[cfg(feature = "search")]
pub(in crate::cli) use language_projection::run_language_projection;
pub(in crate::cli) use query::{
    ExactSourceQuery, QueryCommand, parse_query, print_query_guide, print_query_help,
    query_guide_kind,
};
pub(in crate::cli) use review_packet::run_review;
pub(in crate::cli) use runner_support::{
    discover_rust_project_root, is_command, is_known_search_view, is_search_pipe,
    moved_agent_action, parse_usize_option, print_agent_doctor, print_agent_help, print_check_help,
    print_guide, print_help, print_search_help, rust_package_root_for_path,
    rust_project_root_for_path, search_view_accepts_optional_query, search_view_requires_query,
    search_view_supports_query_set, split_csv_values,
};
#[cfg(feature = "search")]
pub(in crate::cli) use search_output::{SearchOutputControls, apply_search_output_controls};
#[cfg(feature = "search")]
pub(in crate::cli) use search_plan::{SearchPlanOptions, render_search_plan};
#[cfg(feature = "search")]
pub(in crate::cli) use search_trace::{SearchTraceOptions, render_search_trace};
#[cfg(feature = "search")]
pub(in crate::cli) use semantic_search_json::{SemanticSearchJsonOptions, render_search_json};
#[cfg(feature = "search")]
pub(in crate::cli) use semantic_search_json_canonical::{
    canonical_owner_path, canonical_query_set_terms, canonicalize_read_field,
};
#[cfg(feature = "search")]
pub(in crate::cli) use semantic_search_json_fields::{
    bool_field, display_path, header_package, input_detection_from_header, insert_if_some,
    insert_if_usize, location, location_from_node, next_field, parse_edge_kind, parse_fields,
    parse_next_actions, string_field,
};
#[cfg(feature = "search")]
pub(in crate::cli) use semantic_search_synthesis_json::{
    graph_seed_fragment, merge_seed_fragment_search_synthesis, push_synthesis,
    query_set_search_synthesis,
};
#[cfg(feature = "search")]
pub(in crate::cli) use semantic_syntax_refs::attach_syntax_refs_to_search_items;

pub use runner::run_cli_from_env;
