//! Semantic verification coverage required by build gates.

use std::path::Path;

use crate::model::AspRustConfig;
use crate::verification::{RustVerificationTaskKind, plan_rust_project_verification_with_config};

use super::guidance::downstream_build_gate_agent_guidance;
use super::receipt::has_report_obligation;
use super::support::cargo_manifest_dir;

/// Assert that a cargo-check build gate has active semantic verification tasks.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, the verification plan cannot be
/// built, or the configured plan lacks active verification tasks/reports.
#[track_caller]
pub fn assert_asp_rust_verification_from_env_with_config(config: &AspRustConfig, gate_label: &str) {
    let root = cargo_manifest_dir();
    assert_asp_rust_verification_with_config(&root, config, gate_label);
}

/// Assert that a cargo-check build gate has active semantic verification tasks.
///
/// This mirrors a Clippy-style build-script gate: downstream crates pass their
/// harness config through `build.rs`, and Cargo surfaces missing semantic
/// verification coverage during `cargo check`/`cargo test` compilation.
///
/// # Panics
///
/// Panics when the verification plan cannot be built, or the configured plan
/// lacks active verification tasks/reports.
#[track_caller]
pub fn assert_asp_rust_verification_with_config(
    project_root: &Path,
    config: &AspRustConfig,
    gate_label: &str,
) {
    let performance = configured_verification_task_kind(
        &config.verification_policy,
        &RustVerificationTaskKind::Performance,
    );
    let stability = configured_verification_task_kind(
        &config.verification_policy,
        &RustVerificationTaskKind::Stability,
    );
    if !performance && !stability {
        return;
    }
    let plan =
        plan_rust_project_verification_with_config(project_root, config).unwrap_or_else(|error| {
            panic!(
                "{gate_label} verification plan: {error}\n{}",
                downstream_build_gate_agent_guidance(gate_label)
            )
        });
    assert_asp_rust_verification_plan(&plan, &config.verification_policy, gate_label);
}

pub(super) fn assert_asp_rust_verification_plan(
    plan: &crate::RustVerificationPlan,
    policy: &crate::RustVerificationPolicy,
    gate_label: &str,
) {
    let performance =
        configured_verification_task_kind(policy, &RustVerificationTaskKind::Performance);
    let stability = configured_verification_task_kind(policy, &RustVerificationTaskKind::Stability);
    if !performance && !stability {
        return;
    }
    if performance {
        assert_active_verification_task(plan, gate_label, RustVerificationTaskKind::Performance);
        assert_verification_report_obligation(plan, gate_label, "performance_index_json");
    }
    if stability {
        assert_active_verification_task(plan, gate_label, RustVerificationTaskKind::Stability);
        assert_verification_report_obligation(plan, gate_label, "stability_index_json");
    }
}

pub(super) fn configured_verification_task_kind(
    policy: &crate::RustVerificationPolicy,
    task_kind: &RustVerificationTaskKind,
) -> bool {
    if policy.disabled_task_kinds.contains(task_kind) {
        return false;
    }

    policy.skill_bindings.contains_key(task_kind)
        || policy.task_contract_overrides.contains_key(task_kind)
        || policy
            .responsibility_task_overrides
            .values()
            .any(|task_kinds| task_kinds.contains(task_kind))
        || (task_kind == &RustVerificationTaskKind::Stability && policy.stability_picture.is_some())
}

fn assert_active_verification_task(
    plan: &crate::verification::RustVerificationPlan,
    gate_label: &str,
    kind: RustVerificationTaskKind,
) {
    if !plan
        .tasks
        .iter()
        .any(|task| task.kind == kind && task.is_active())
    {
        panic!(
            "{gate_label} build gate must configure active {kind:?} verification tasks\n{}",
            downstream_build_gate_agent_guidance(gate_label)
        );
    }
}

fn assert_verification_report_obligation(
    plan: &crate::verification::RustVerificationPlan,
    gate_label: &str,
    key: &str,
) {
    if !has_report_obligation(plan, key) {
        panic!(
            "{gate_label} build gate must require a {key} report\n{}",
            downstream_build_gate_agent_guidance(gate_label)
        );
    }
}
