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
    RustProjectHarnessDependencyBaseline, RustProjectHarnessDependencyBaselinePackage,
    assert_rust_project_harness_dependency_baseline,
};
#[cfg(test)]
pub(crate) use downstream::assert_rust_project_harness_downstream_policy_with_state_home;
pub use downstream::{
    assert_rust_project_harness_downstream_policy,
    assert_rust_project_harness_downstream_policy_from_env,
};
pub use policy::{RustProjectHarnessDownstreamPolicy, RustProjectHarnessWorkspacePolicy};
pub(crate) use receipt::downstream_policy_receipt_from_plan;
pub(crate) use receipt::verification_task_kind_key;
pub use receipt::{
    RUST_PROJECT_HARNESS_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID,
    RUST_PROJECT_HARNESS_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION,
    RustProjectHarnessDependencyBaselinePackageReceipt, RustProjectHarnessDownstreamPolicyReceipt,
    RustProjectHarnessReportObligationReceipt,
    render_rust_project_harness_downstream_policy_receipt_json,
    rust_project_harness_downstream_policy_receipt,
};
pub use verification_gate::{
    assert_rust_project_harness_verification_from_env_with_config,
    assert_rust_project_harness_verification_with_config,
};
