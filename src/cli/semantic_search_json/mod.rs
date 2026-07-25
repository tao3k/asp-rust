//! Shared semantic-search JSON envelope for CLI search output.

mod packet;

pub(super) use packet::{SemanticSearchJsonOptions, render_search_json};
