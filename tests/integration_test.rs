//! Thin root target for CLI integration scenarios.

#[path = "integration/direct_exact_stdin.rs"]
mod direct_exact_stdin;
#[path = "integration/language_projection_impl_identity.rs"]
mod language_projection_impl_identity;
#[path = "integration/owner_search_stdin.rs"]
mod owner_search_stdin;
#[path = "integration/project_resolution_scope.rs"]
mod project_resolution_scope;
#[path = "integration/project_resolution_stdin.rs"]
mod project_resolution_stdin;
#[path = "integration/removed_query_surface.rs"]
mod removed_query_surface;
