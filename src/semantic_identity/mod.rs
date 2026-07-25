//! Provider-local typed identities for schema-backed exact selectors.
//!
//! The Rust harness owns these value objects locally so the standalone provider
//! does not acquire a reverse filesystem dependency on its ASP host repository.

pub(crate) mod canonical_item_identity;
pub(crate) mod exact_selector_merkle;
pub(crate) mod exact_selector_projection_packet;
pub(crate) mod structural_selector;
