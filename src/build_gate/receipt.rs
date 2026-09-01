//! Snapshot of downstream build-gate policy obligations and evidence.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::verification::{
    RustVerificationPlan, RustVerificationTaskKind, plan_rust_project_verification_with_config,
};

use super::dependency_baseline::AspRustDependencyBaseline;
use super::policy::AspRustDownstreamPolicy;

/// Stable schema id for downstream policy receipt projections.
pub const ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID: &str = "asp-rust.downstream-policy-receipt";
/// Current downstream policy receipt schema version.
pub const ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION: &str = "1";

/// Agent-facing receipt for a downstream build-gate policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspRustDownstreamPolicyReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub gate_label: String,
    pub dependency_baseline_packages: Vec<AspRustDependencyBaselinePackageReceipt>,
    pub active_verification_task_count: usize,
    pub performance_task_count: usize,
    pub stability_task_count: usize,
    pub performance_report_obligation: bool,
    pub stability_report_obligation: bool,
    pub report_obligations: Vec<AspRustReportObligationReceipt>,
}

/// Receipt projection of one dependency baseline package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspRustDependencyBaselinePackageReceipt {
    pub name: String,
    pub version: String,
    pub source_contains: String,
}

/// Receipt projection of one verification report obligation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspRustReportObligationReceipt {
    pub key: String,
    pub renderer: String,
    pub suggested_artifact_name: String,
    pub reason: String,
    pub task_kinds: Vec<String>,
    pub task_fingerprints: Vec<String>,
}

/// Build an agent-facing receipt for a downstream policy without asserting it.
pub fn asp_rust_downstream_policy_receipt(
    project_root: &Path,
    policy: &AspRustDownstreamPolicy,
) -> Result<AspRustDownstreamPolicyReceipt, String> {
    let plan = plan_rust_project_verification_with_config(project_root, policy.config())?;
    Ok(downstream_policy_receipt_from_plan(policy, &plan))
}

pub(crate) fn downstream_policy_receipt_from_plan(
    policy: &AspRustDownstreamPolicy,
    plan: &RustVerificationPlan,
) -> AspRustDownstreamPolicyReceipt {
    AspRustDownstreamPolicyReceipt {
        schema_id: ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID.to_string(),
        schema_version: ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION.to_string(),
        gate_label: policy.gate_label().to_string(),
        dependency_baseline_packages: dependency_baseline_package_receipts(policy),
        active_verification_task_count: plan.active_tasks().len(),
        performance_task_count: active_task_count(plan, RustVerificationTaskKind::Performance),
        stability_task_count: active_task_count(plan, RustVerificationTaskKind::Stability),
        performance_report_obligation: has_report_obligation(plan, "performance_index_json"),
        stability_report_obligation: has_report_obligation(plan, "stability_index_json"),
        report_obligations: plan
            .report_obligations
            .iter()
            .map(|obligation| AspRustReportObligationReceipt {
                key: obligation.key.clone(),
                renderer: obligation.renderer.clone(),
                suggested_artifact_name: obligation.suggested_artifact_name.clone(),
                reason: obligation.reason.clone(),
                task_kinds: obligation
                    .task_kinds
                    .iter()
                    .map(|kind| verification_task_kind_key(*kind).to_string())
                    .collect(),
                task_fingerprints: obligation.task_fingerprints.clone(),
            })
            .collect(),
    }
}

pub(super) fn dependency_baseline_package_receipts(
    policy: &AspRustDownstreamPolicy,
) -> Vec<AspRustDependencyBaselinePackageReceipt> {
    policy
        .dependency_baseline()
        .into_iter()
        .flat_map(AspRustDependencyBaseline::packages)
        .map(|package| AspRustDependencyBaselinePackageReceipt {
            name: package.name().to_string(),
            version: package.version().to_string(),
            source_contains: package.source_contains().to_string(),
        })
        .collect()
}

/// Render a downstream policy receipt as structured JSON for evidence files.
pub fn render_asp_rust_downstream_policy_receipt_json(
    receipt: &AspRustDownstreamPolicyReceipt,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(receipt)
}

fn active_task_count(plan: &RustVerificationPlan, kind: RustVerificationTaskKind) -> usize {
    plan.tasks
        .iter()
        .filter(|task| task.kind == kind && task.is_active())
        .count()
}

pub(super) fn has_report_obligation(plan: &RustVerificationPlan, key: &str) -> bool {
    plan.report_obligations
        .iter()
        .any(|obligation| obligation.key == key)
}

pub(crate) fn verification_task_kind_key(kind: RustVerificationTaskKind) -> &'static str {
    match kind {
        RustVerificationTaskKind::Stress => "stress",
        RustVerificationTaskKind::Performance => "performance",
        RustVerificationTaskKind::Stability => "stability",
        RustVerificationTaskKind::Chaos => "chaos",
        RustVerificationTaskKind::Security => "security",
        RustVerificationTaskKind::Regression => "regression",
        RustVerificationTaskKind::ResponsibilityReview => "responsibility_review",
    }
}
