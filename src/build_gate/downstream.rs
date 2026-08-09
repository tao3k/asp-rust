//! Downstream policy orchestration and receipt projection.

use std::path::Path;

use crate::model::{RustHarnessConfig, RustHarnessReport};

use super::dependency_baseline::assert_rust_project_harness_dependency_baseline;
use super::guidance::downstream_build_gate_agent_guidance;
use super::policy::RustProjectHarnessDownstreamPolicy;
use super::support::{cargo_manifest_dir, has_explanation};

/// Assert a complete downstream policy from `CARGO_MANIFEST_DIR`.
///
/// This is the preferred entrypoint for downstream crates whose policy is too
/// large to live directly in `build.rs`.
///
/// # Panics
///
/// Panics when `CARGO_MANIFEST_DIR` is missing, when the cargo-check policy
/// gate fails, or when semantic verification coverage is incomplete.
#[track_caller]
pub fn assert_rust_project_harness_downstream_policy_from_env(
    policy: &RustProjectHarnessDownstreamPolicy,
) -> RustHarnessReport {
    let root = cargo_manifest_dir();
    assert_rust_project_harness_downstream_policy(&root, policy)
}

/// Assert a complete downstream policy from an explicit project root.
///
/// # Panics
///
/// Panics when the cargo-check policy gate fails, or when semantic
/// verification coverage is incomplete.
#[track_caller]
pub fn assert_rust_project_harness_downstream_policy(
    project_root: &Path,
    policy: &RustProjectHarnessDownstreamPolicy,
) -> RustHarnessReport {
    let cache_root = super::cache::build_gate_cache_root_from_env(project_root);
    run_rust_project_harness_downstream_policy(project_root, policy, cache_root)
}

#[cfg(test)]
pub(crate) fn assert_rust_project_harness_downstream_policy_with_state_home(
    project_root: &Path,
    policy: &RustProjectHarnessDownstreamPolicy,
    state_home: &Path,
) -> RustHarnessReport {
    let cache_root =
        super::cache::build_gate_cache_root(project_root, Some(state_home.as_os_str().to_owned()));
    run_rust_project_harness_downstream_policy(project_root, policy, cache_root)
}

fn run_rust_project_harness_downstream_policy(
    project_root: &Path,
    policy: &RustProjectHarnessDownstreamPolicy,
    cache_root: Option<std::path::PathBuf>,
) -> RustHarnessReport {
    let dependency_baseline_receipts = super::receipt::dependency_baseline_package_receipts(policy);
    let snapshot = super::cache::snapshot_build_gate_inputs(project_root, policy.config())
        .unwrap_or_else(|error| {
            panic!(
                "{} cargo-check build-gate snapshot: {error}\n{}",
                policy.gate_label(),
                downstream_build_gate_agent_guidance(policy.gate_label())
            )
        });
    let cache_key = super::cache::build_gate_cache_key(
        policy.config(),
        crate::runner::RustHarnessRunScope::Package,
        &dependency_baseline_receipts,
        &snapshot,
    )
    .unwrap_or_else(|error| {
        panic!(
            "{} cargo-check build-gate cache key: {error}\n{}",
            policy.gate_label(),
            downstream_build_gate_agent_guidance(policy.gate_label())
        )
    });
    if let Some(record) = cache_root
        .as_deref()
        .and_then(|cache_root| super::cache::load_build_gate_cache(cache_root, &cache_key))
    {
        super::rerun::emit_cargo_rerun_paths(
            project_root,
            record.snapshot.files.iter().map(|file| &file.path),
        );
        assert_build_report_clean_with_agent_guidance(
            &record.report,
            policy.config(),
            policy.gate_label(),
        );
        super::verification_gate::assert_rust_project_harness_verification_plan(
            &record.verification_plan,
            &policy.config().verification_policy,
            policy.gate_label(),
        );
        let expected_receipt =
            super::receipt::downstream_policy_receipt_from_plan(policy, &record.verification_plan);
        assert_eq!(
            record.downstream_policy_receipt,
            expected_receipt,
            "{} cached downstream policy receipt drift",
            policy.gate_label()
        );
        assert_eq!(
            record.dependency_baseline_receipts,
            dependency_baseline_receipts,
            "{} cached dependency baseline receipt drift",
            policy.gate_label()
        );
        if let Some(dependency_baseline) = policy.dependency_baseline() {
            assert_rust_project_harness_dependency_baseline(
                project_root,
                dependency_baseline,
                policy.gate_label(),
            );
        }
        return record.report;
    }

    let analysis = crate::runner::analyze_rust_project_once(
        project_root,
        policy.config(),
        crate::runner::RustHarnessRunScope::Package,
    )
    .unwrap_or_else(|error| {
        panic!(
            "{} cargo-check build gate: {error}\n{}",
            policy.gate_label(),
            downstream_build_gate_agent_guidance(policy.gate_label())
        )
    });
    super::rerun::emit_cargo_rerun_inputs(project_root, &analysis);
    let report = analysis.to_report(policy.config());
    assert_build_report_clean_with_agent_guidance(&report, policy.config(), policy.gate_label());
    let verification_plan = crate::verification::plan_rust_verification_from_harness_analysis(
        analysis,
        &policy.config().verification_policy,
    );
    super::verification_gate::assert_rust_project_harness_verification_plan(
        &verification_plan,
        &policy.config().verification_policy,
        policy.gate_label(),
    );
    if let Some(dependency_baseline) = policy.dependency_baseline() {
        assert_rust_project_harness_dependency_baseline(
            project_root,
            dependency_baseline,
            policy.gate_label(),
        );
    }
    let downstream_policy_receipt =
        super::receipt::downstream_policy_receipt_from_plan(policy, &verification_plan);
    if let Some(cache_root) = cache_root {
        let payload_digest = super::cache::build_gate_cache_payload_digest(
            &report,
            &verification_plan,
            &downstream_policy_receipt,
            &dependency_baseline_receipts,
        )
        .unwrap_or_else(|error| {
            panic!(
                "{} cargo-check build-gate cache payload: {error}\n{}",
                policy.gate_label(),
                downstream_build_gate_agent_guidance(policy.gate_label())
            )
        });
        let record = super::cache::RustProjectHarnessBuildGateCacheRecord {
            schema_id: super::cache::RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_ID.to_string(),
            schema_version: super::cache::RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_VERSION
                .to_string(),
            cache_key,
            snapshot,
            payload_digest,
            report: report.clone(),
            verification_plan,
            downstream_policy_receipt,
            dependency_baseline_receipts,
        };
        super::cache::store_build_gate_cache(&cache_root, &record).unwrap_or_else(|error| {
            panic!(
                "{} cargo-check build-gate cache publish: {error}\n{}",
                policy.gate_label(),
                downstream_build_gate_agent_guidance(policy.gate_label())
            )
        });
    }
    report
}

fn assert_build_report_clean_with_agent_guidance(
    report: &RustHarnessReport,
    config: &RustHarnessConfig,
    gate_label: &str,
) {
    if !report.is_clean() {
        panic!(
            "{}\n{}",
            crate::render_rust_project_harness(report),
            downstream_build_gate_agent_guidance(gate_label)
        );
    }
    if !config_allows_agent_advice(config) {
        let rendered = crate::render_rust_project_harness_advice(report);
        if !rendered.is_empty() {
            panic!(
                "{rendered}\n{}",
                downstream_build_gate_agent_guidance(gate_label)
            );
        }
    }
}

fn config_allows_agent_advice(config: &RustHarnessConfig) -> bool {
    has_explanation(config.cargo_check_advice_allow_explanation.as_deref())
        || has_explanation(config.agent_advice_allow_explanation.as_deref())
}
