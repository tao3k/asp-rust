//! Collection graph emission orchestration.

use std::collections::BTreeSet;
use std::fs;
use std::ops::ControlFlow;

use serde_json::Value;

use super::facts::push_field_graph_facts;
use super::field_extract::collection_fields;
use super::owner_scan::CandidateOwner;

const FIELD_LIMIT: usize = 24;

struct GraphSink<'a> {
    nodes: &'a mut Vec<Value>,
    edges: &'a mut Vec<Value>,
    seen_nodes: &'a mut BTreeSet<String>,
    seen_edges: &'a mut BTreeSet<(String, String, String)>,
}

pub(in crate::search::semantic_facts) fn emit_collection_field_graph_facts(
    query: &str,
    owners: Vec<CandidateOwner>,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let mut sink = GraphSink {
        nodes,
        edges,
        seen_nodes,
        seen_edges,
    };
    let _ = owners
        .into_iter()
        .try_fold(FIELD_LIMIT, |remaining, owner| {
            if remaining == 0 {
                return ControlFlow::Break(());
            }
            let emitted = emit_owner_collection_fields(query, &owner, remaining, &mut sink);
            let next_remaining = remaining.saturating_sub(emitted);
            if next_remaining == 0 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(next_remaining)
            }
        });
}

fn emit_owner_collection_fields(
    query: &str,
    owner: &CandidateOwner,
    limit: usize,
    sink: &mut GraphSink<'_>,
) -> usize {
    let Ok(source) = fs::read_to_string(&owner.absolute) else {
        return 0;
    };
    let Ok(syntax) = crate::parser::parse_rust_source_syntax(&source) else {
        return 0;
    };
    emit_collection_fields_from_syntax(query, owner, &syntax, limit, sink)
}

fn emit_collection_fields_from_syntax(
    query: &str,
    owner: &CandidateOwner,
    syntax: &syn::File,
    limit: usize,
    sink: &mut GraphSink<'_>,
) -> usize {
    let fields = collection_fields(&owner.display, syntax)
        .into_iter()
        .filter(|field| field.matches_query(query))
        .take(limit)
        .collect::<Vec<_>>();
    fields.iter().for_each(|field| {
        push_field_graph_facts(
            query,
            field,
            sink.nodes,
            sink.edges,
            sink.seen_nodes,
            sink.seen_edges,
        );
    });
    fields.len()
}
