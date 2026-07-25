//! Compact search output rendering and seed extraction helpers.
mod blocks;
mod core;
mod graph;
mod graph_dependency;
mod package;

pub(super) use core::{SearchOutputControls, apply_search_output_controls};
pub(super) use graph::render_search_graph_packet;
