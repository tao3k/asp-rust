//! Compact search output rendering and seed extraction helpers.
mod blocks;
mod compact_graph;
mod core;
mod package;

pub(super) use compact_graph::render_compact_graph_seed_packet;
pub(super) use core::{SearchOutputControls, apply_search_output_controls};
