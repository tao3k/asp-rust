//! Runtime-owned HTTP/JSON server surface for the Rust language provider.
//!
//! This module is intentionally the only executable surface of `asp-rust`.
//! Search, query, policy, and projection commands are ASP Server operations;
//! they are not provider-local command-line authorities.

mod contract;
mod http_json;
mod project_resolution;
mod projection;
mod runtime;
mod syntax_query;

pub use runtime::run_provider_server_from_env;
