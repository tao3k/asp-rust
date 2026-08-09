//! Compact search protocol renderers for agent exploration.

mod api;
mod cargo;
mod compare;
mod context;
pub(crate) mod density;
mod dependency;
mod format;
pub(crate) mod guide;
mod hits;
mod ingest;
mod item_query;
mod limits;
mod namespace;
mod owner;
mod owner_seed_view;
mod owner_view;
pub(crate) mod policy;
mod prime;
mod prime_support;
mod query;
mod recency;
mod scope;
#[cfg(feature = "cli")]
mod semantic_facts;
mod syntax_query;
mod version;

pub(in crate::search) use crate::parser::{
    CargoDependencyFacts, ParsedRustModule, parse_rust_file,
};
pub(in crate::search) use crate::{discover_rust_files, rust_project_harness_scope};
pub use api::{
    RustSearchOptions, RustSearchViewRequest, render_rust_project_harness_search_view_with_config,
};
pub use compare::render_search_compare_json as render_rust_project_harness_search_compare_json_with_config;
pub(in crate::search) use context::{
    PackageSearchContext, search_contexts, search_contexts_for_path_query,
};
pub(in crate::search) use dependency::render_search_dependency;
pub(in crate::search) use format::{
    append_block, compact_locations, display_project_path, package_label, package_roots_for_request,
};
pub(in crate::search) use hits::{
    SearchHit, import_hits, matching_dependencies, sort_search_hits_by_recency, symbol_calls,
    symbol_definitions,
};
pub use ingest::render_rust_project_harness_search_ingest_with_config;
pub(in crate::search) use limits::SEARCH_HIT_LIMIT;
pub(in crate::search) use owner_view::render_search_owner;
pub use prime::{
    render_rust_project_harness_search_prime, render_rust_project_harness_search_prime_with_config,
};
pub(in crate::search) use recency::compare_paths_by_recency;
pub(in crate::search) use scope::{module_allowed, path_allowed_by_scope};
#[cfg(feature = "cli")]
pub use semantic_facts::{
    render_rust_project_harness_dependency_topology_json,
    render_rust_project_harness_dependency_topology_metadata_json,
    render_rust_project_harness_search_semantic_facts_json,
};
pub(in crate::search) use version::version_requirement_matches_request;
