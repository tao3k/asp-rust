//! Build-script entrypoints for Cargo-check project harness gates.

mod cache;
mod dependency_baseline;
mod downstream;
mod guidance;
mod policy;
mod receipt;
mod rerun;
mod support;
mod verification_gate;

pub use dependency_baseline::{
    AspRustDependencyBaseline, AspRustDependencyBaselinePackage,
    assert_asp_rust_dependency_baseline,
};
#[cfg(test)]
pub(crate) use downstream::assert_asp_rust_downstream_policy_with_state_home;
pub use downstream::{
    assert_asp_rust_downstream_policy, assert_asp_rust_downstream_policy_from_env,
    assert_asp_rust_downstream_policy_with_authority,
};
pub use policy::{AspRustBuildGateAuthority, AspRustDownstreamPolicy, AspRustWorkspacePolicy};
pub(crate) use receipt::downstream_policy_receipt_from_plan;
pub(crate) use receipt::verification_task_kind_key;
pub use receipt::{
    ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID,
    ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION, AspRustDependencyBaselinePackageReceipt,
    AspRustDownstreamPolicyReceipt, AspRustReportObligationReceipt,
    asp_rust_downstream_policy_receipt, render_asp_rust_downstream_policy_receipt_json,
};
pub use verification_gate::{
    assert_asp_rust_verification_from_env_with_config, assert_asp_rust_verification_with_config,
};
