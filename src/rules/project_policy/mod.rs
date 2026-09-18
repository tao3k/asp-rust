//! Project-level Rust test policy rules.

mod catalog;
mod config;
mod dev_gate;
mod manifest;
mod pack;
pub(crate) use pack::evaluate_workspace;
mod quality;
mod source_scope;
mod source_tests;
mod support;
mod test_bloat;
mod test_layout;
mod test_targets;
mod verification_integration;

pub use pack::rust_project_policy_rules;
pub(crate) use pack::{
    MAX_INTEGRATION_TEST_EFFECTIVE_LINES, MAX_TEST_SUPPORT_EFFECTIVE_LINES,
    MAX_UNIT_TEST_EFFECTIVE_LINES, MIN_INTEGRATION_TEST_FUNCTIONS, MIN_UNIT_TEST_FUNCTIONS,
    PACK_ID, RUST_PROJ_R001, RUST_PROJ_R002, RUST_PROJ_R003, RUST_PROJ_R004, RUST_PROJ_R005,
    RUST_PROJ_R006, RUST_PROJ_R007, RUST_PROJ_R008, RUST_PROJ_R009, RUST_PROJ_R010, RUST_PROJ_R011,
    RUST_PROJ_R012, RUST_PROJ_R013, RUST_PROJ_R014, RUST_PROJ_R015, RUST_PROJ_R016, RUST_PROJ_R017,
    RUST_PROJ_R018, RUST_PROJ_R019, RUST_PROJ_R020, RUST_PROJ_R021, RUST_PROJ_R022, RUST_PROJ_R023,
    RUST_PROJ_R024, evaluate,
};
