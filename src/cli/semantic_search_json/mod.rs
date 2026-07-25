//! Shared semantic-search JSON envelope for CLI search output.

mod packet;

pub(super) use packet::{SemanticSearchJsonOptions, build_search_packet, render_search_json};
