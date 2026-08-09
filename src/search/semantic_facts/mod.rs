//! Provider-owned bounded semantic graph facts for ASP search pipe enrichment.

mod cargo_graph;
mod collection_graph;
mod contract;
#[cfg(feature = "cli")]
mod dependency_topology;
mod graph_helpers;
mod render;

#[cfg(feature = "cli")]
pub use dependency_topology::{
    render_rust_project_harness_dependency_topology_json,
    render_rust_project_harness_dependency_topology_metadata_json,
};
pub use render::render_rust_project_harness_search_semantic_facts_json;
