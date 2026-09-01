//! Public semantic fact graph renderer.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::json;

use super::cargo_graph::emit_cargo_project_graph_facts;
use super::collection_graph::{emit_collection_field_graph_facts, semantic_fact_owners};
use super::contract::{LANGUAGE_ID, PROVIDER_ID};

/// Render bounded semantic graph facts for collection-field search enrichment.
pub fn render_asp_rust_search_semantic_facts_json(
    project_root: &Path,
    query: &str,
    input: &str,
) -> Result<String, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes = BTreeSet::new();
    let mut seen_edges = BTreeSet::new();
    emit_collection_field_graph_facts(
        query,
        semantic_fact_owners(project_root, input),
        &mut nodes,
        &mut edges,
        &mut seen_nodes,
        &mut seen_edges,
    );
    emit_cargo_project_graph_facts(
        project_root,
        &mut nodes,
        &mut edges,
        &mut seen_nodes,
        &mut seen_edges,
    );
    serde_json::to_string_pretty(&json!({
        "schemaId": "agent.semantic-protocols.semantic-fact-graph",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "languageId": LANGUAGE_ID,
        "providerId": PROVIDER_ID,
        "projectRoot": project_root.display().to_string().replace('\\', "/"),
        "query": query,
        "nodes": nodes,
        "edges": edges,
    }))
    .map(|mut text| {
        text.push('\n');
        text
    })
    .map_err(|error| format!("failed to render semantic fact JSON: {error}"))
}
