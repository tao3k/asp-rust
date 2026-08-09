//! Workspace-level evidence graph receipts for multi-crate build gates.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::build_gate::{
    RustProjectHarnessDownstreamPolicy, RustProjectHarnessDownstreamPolicyReceipt,
    downstream_policy_receipt_from_plan, verification_task_kind_key,
};
use crate::verification::{
    RustVerificationPlan, RustVerificationTaskKind, plan_rust_project_verification_with_config,
};
use crate::{RustHarnessConfig, RustHarnessReport};

/// Stable schema id for multi-crate workspace evidence graph receipts.
pub const RUST_PROJECT_HARNESS_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_ID: &str =
    "rust-lang-project-harness.workspace-evidence-graph-receipt";

/// Current workspace evidence graph receipt schema version.
pub const RUST_PROJECT_HARNESS_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_VERSION: &str = "1";

/// Input for one member crate in a workspace evidence graph receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectHarnessWorkspaceEvidenceGraphMemberInput {
    crate_label: String,
    project_root: PathBuf,
    policy: RustProjectHarnessDownstreamPolicy,
}

impl RustProjectHarnessWorkspaceEvidenceGraphMemberInput {
    /// Create one member crate input for a workspace evidence graph receipt.
    #[must_use]
    pub fn new(
        crate_label: impl Into<String>,
        project_root: impl Into<PathBuf>,
        policy: RustProjectHarnessDownstreamPolicy,
    ) -> Self {
        Self {
            crate_label: crate_label.into(),
            project_root: project_root.into(),
            policy,
        }
    }

    /// Human-readable member label.
    #[must_use]
    pub fn crate_label(&self) -> &str {
        &self.crate_label
    }

    /// Member crate root used for parser-owned verification planning.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        self.project_root.as_path()
    }

    /// Downstream policy asserted by the member crate build gate.
    #[must_use]
    pub fn policy(&self) -> &RustProjectHarnessDownstreamPolicy {
        &self.policy
    }
}

/// Result of explicitly composing package-atomic downstream gates for a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectHarnessWorkspaceRunReport {
    /// Cargo workspace root whose manifest admitted every selected member.
    pub workspace_root: PathBuf,
    /// Independently evaluated package reports.
    pub members: Vec<RustProjectHarnessWorkspaceMemberRunReport>,
}

/// One package atom in an explicit workspace run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustProjectHarnessWorkspaceMemberRunReport {
    /// Caller-owned member label.
    pub crate_label: String,
    /// Cargo package root selected for this atom.
    pub project_root: PathBuf,
    /// Package-scoped harness report.
    pub report: RustHarnessReport,
}

/// Assert explicitly selected workspace members as independent package atoms.
///
/// The Cargo manifest graph admits member roots before any package analysis.
/// Each admitted member then runs through the ordinary downstream package API;
/// no workspace-wide source traversal or shared member policy is introduced.
///
/// # Panics
///
/// Panics when a requested root is absent from the Cargo package graph, a root
/// is selected twice, or one member policy fails.
#[track_caller]
pub fn assert_rust_workspace_harness_downstream_policies(
    workspace_root: &Path,
    workspace_config: &RustHarnessConfig,
    members: impl IntoIterator<Item = RustProjectHarnessWorkspaceEvidenceGraphMemberInput>,
) -> RustProjectHarnessWorkspaceRunReport {
    let admitted_roots = crate::discovery::discover_cargo_package_roots(
        workspace_root,
        &workspace_config.ignored_dir_names,
        &workspace_config.include_hidden_dir_names,
    )
    .into_iter()
    .map(|root| crate::path::normalize_lexical_path(&root))
    .collect::<BTreeSet<_>>();
    let mut selected_roots = BTreeSet::new();
    let mut reports = Vec::new();
    for member in members {
        let project_root = crate::path::normalize_lexical_path(member.project_root());
        assert!(
            admitted_roots.contains(&project_root),
            "workspace member `{}` is not admitted by Cargo graph rooted at {}: {}",
            member.crate_label(),
            workspace_root.display(),
            project_root.display()
        );
        assert!(
            selected_roots.insert(project_root.clone()),
            "workspace package selected more than once: {}",
            project_root.display()
        );
        let report = crate::build_gate::assert_rust_project_harness_downstream_policy(
            &project_root,
            member.policy(),
        );
        assert!(
            report
                .root_paths
                .iter()
                .all(|path| path.starts_with(&project_root)),
            "workspace member gate escaped package atom {}",
            project_root.display()
        );
        reports.push(RustProjectHarnessWorkspaceMemberRunReport {
            crate_label: member.crate_label().to_string(),
            project_root,
            report,
        });
    }
    RustProjectHarnessWorkspaceRunReport {
        workspace_root: crate::path::normalize_lexical_path(workspace_root),
        members: reports,
    }
}

/// Agent-facing multi-crate evidence graph receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessWorkspaceEvidenceGraphReceipt {
    /// Stable receipt schema id.
    pub schema_id: String,
    /// Stable receipt schema version.
    pub schema_version: String,
    /// Human-readable workspace label.
    pub workspace_label: String,
    /// Workspace root used by the downstream project.
    pub workspace_root: String,
    /// Aggregated graph summary.
    pub summary: RustProjectHarnessWorkspaceEvidenceGraphSummaryReceipt,
    /// Member policy receipts included in this graph.
    pub members: Vec<RustProjectHarnessWorkspaceEvidenceGraphMemberReceipt>,
    /// Evidence graph nodes for workspace, members, dependencies, reports, and task kinds.
    pub nodes: Vec<RustProjectHarnessWorkspaceEvidenceGraphNodeReceipt>,
    /// Directed evidence graph edges connecting policy obligations.
    pub edges: Vec<RustProjectHarnessWorkspaceEvidenceGraphEdgeReceipt>,
    /// Trust loop steps an agent should close before treating the workspace as reliable.
    pub trust_loop_steps: Vec<RustProjectHarnessWorkspaceTrustLoopStepReceipt>,
}

/// Aggregated evidence graph summary for a multi-crate workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessWorkspaceEvidenceGraphSummaryReceipt {
    /// Number of member crate policies projected into this graph.
    pub member_crate_count: usize,
    /// Number of dependency baseline package requirements across member gates.
    pub dependency_baseline_package_count: usize,
    /// Number of active verification tasks across all members.
    pub active_verification_task_count: usize,
    /// Number of active performance verification tasks across all members.
    pub performance_task_count: usize,
    /// Number of active stability verification tasks across all members.
    pub stability_task_count: usize,
    /// Number of active security verification tasks across all members.
    pub security_task_count: usize,
    /// Number of required report obligations across all members.
    pub report_obligation_count: usize,
}

/// Evidence graph projection for one member crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessWorkspaceEvidenceGraphMemberReceipt {
    /// Member label supplied by the workspace policy owner.
    pub crate_label: String,
    /// Member crate root used for verification planning.
    pub project_root: String,
    /// Downstream policy receipt for this member.
    pub policy_receipt: RustProjectHarnessDownstreamPolicyReceipt,
    /// Active task counts by verification kind.
    pub active_task_kind_counts: Vec<RustProjectHarnessVerificationTaskKindCountReceipt>,
}

/// Count of active verification tasks for one task kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessVerificationTaskKindCountReceipt {
    /// Stable verification task kind.
    pub kind: RustVerificationTaskKind,
    /// Active task count.
    pub count: usize,
}

/// Node kind in the workspace evidence graph receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustProjectHarnessWorkspaceEvidenceGraphNodeKind {
    /// Workspace policy root.
    Workspace,
    /// Member crate build-gate policy.
    MemberCrate,
    /// Active verification task family.
    VerificationTaskKind,
    /// Dependency baseline requirement.
    DependencyBaselinePackage,
    /// Required report artifact.
    ReportObligation,
}

/// One node in the workspace evidence graph receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessWorkspaceEvidenceGraphNodeReceipt {
    /// Stable node id within the receipt.
    pub id: String,
    /// Typed node kind.
    pub kind: RustProjectHarnessWorkspaceEvidenceGraphNodeKind,
    /// Human-readable node label.
    pub label: String,
}

/// Edge kind in the workspace evidence graph receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustProjectHarnessWorkspaceEvidenceGraphEdgeKind {
    /// Workspace contains a member crate gate.
    Contains,
    /// Member requires active verification coverage.
    RequiresVerification,
    /// Member requires dependency baseline evidence.
    RequiresDependencyBaseline,
    /// Member requires a report artifact.
    RequiresReport,
    /// Report covers a verification task kind.
    Covers,
}

/// One directed edge in the workspace evidence graph receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessWorkspaceEvidenceGraphEdgeReceipt {
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Typed edge kind.
    pub kind: RustProjectHarnessWorkspaceEvidenceGraphEdgeKind,
}

/// Trust loop status in the workspace evidence graph receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustProjectHarnessWorkspaceTrustLoopStepStatus {
    /// Policy or evidence is configured.
    Configured,
    /// No member crate inputs were supplied.
    MissingMembers,
    /// Evidence is required by the gate.
    Required,
    /// Evidence is optional or not yet configured.
    NotConfigured,
    /// Verification is active.
    Active,
    /// Verification has no active tasks.
    MissingActiveTasks,
    /// Required evidence is incomplete.
    Incomplete,
    /// Build gate is enforced by member crates.
    Enforced,
    /// Build gate is not enforceable from the receipt.
    NotEnforced,
}

/// One trust-loop step projected from the workspace evidence graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustProjectHarnessWorkspaceTrustLoopStepReceipt {
    /// Stable step key.
    pub key: String,
    /// Typed step status for agent triage.
    pub status: RustProjectHarnessWorkspaceTrustLoopStepStatus,
    /// Evidence node ids that justify the step status.
    pub evidence_node_ids: Vec<String>,
    /// Agent action for closing or preserving this step.
    pub agent_action: String,
}

/// Build a multi-crate workspace evidence graph receipt from member policies.
///
/// The graph is intentionally projected at the build-gate boundary: downstream
/// workspaces can persist it from `build.rs` or CI without reimplementing the
/// parser-owned verification planner.
pub fn rust_project_harness_workspace_evidence_graph_receipt(
    workspace_root: &Path,
    workspace_label: impl Into<String>,
    members: impl IntoIterator<Item = RustProjectHarnessWorkspaceEvidenceGraphMemberInput>,
) -> Result<RustProjectHarnessWorkspaceEvidenceGraphReceipt, String> {
    let workspace_label = workspace_label.into();
    let member_receipts = members
        .into_iter()
        .map(workspace_evidence_graph_member_receipt)
        .collect::<Result<Vec<_>, _>>()?;
    let summary = summarize_workspace_evidence_graph(&member_receipts);
    let (nodes, edges) = build_workspace_evidence_graph_edges(&workspace_label, &member_receipts);
    let trust_loop_steps = build_workspace_trust_loop_steps(&summary, &nodes);

    Ok(RustProjectHarnessWorkspaceEvidenceGraphReceipt {
        schema_id: RUST_PROJECT_HARNESS_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_ID.to_string(),
        schema_version: RUST_PROJECT_HARNESS_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_VERSION
            .to_string(),
        workspace_label,
        workspace_root: workspace_root.display().to_string(),
        summary,
        members: member_receipts,
        nodes,
        edges,
        trust_loop_steps,
    })
}

/// Render a workspace evidence graph receipt as structured JSON for evidence files.
///
/// # Errors
///
/// Returns a serialization error if the receipt cannot be encoded as JSON.
pub fn render_rust_project_harness_workspace_evidence_graph_receipt_json(
    receipt: &RustProjectHarnessWorkspaceEvidenceGraphReceipt,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(receipt)
}

fn active_task_kind_counts(
    plan: &RustVerificationPlan,
) -> Vec<RustProjectHarnessVerificationTaskKindCountReceipt> {
    let mut counts = BTreeMap::<RustVerificationTaskKind, usize>::new();
    for task in plan.active_tasks() {
        *counts.entry(task.kind).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(kind, count)| RustProjectHarnessVerificationTaskKindCountReceipt { kind, count })
        .collect()
}

fn task_kind_count(
    counts: &[RustProjectHarnessVerificationTaskKindCountReceipt],
    kind: RustVerificationTaskKind,
) -> usize {
    counts
        .iter()
        .find(|count| count.kind == kind)
        .map(|count| count.count)
        .unwrap_or(0)
}

fn workspace_evidence_graph_member_receipt(
    member: RustProjectHarnessWorkspaceEvidenceGraphMemberInput,
) -> Result<RustProjectHarnessWorkspaceEvidenceGraphMemberReceipt, String> {
    let plan =
        plan_rust_project_verification_with_config(member.project_root(), member.policy().config())
            .map_err(|error| format!("{} verification plan: {error}", member.crate_label()))?;
    let policy_receipt = downstream_policy_receipt_from_plan(member.policy(), &plan);
    Ok(RustProjectHarnessWorkspaceEvidenceGraphMemberReceipt {
        crate_label: member.crate_label,
        project_root: member.project_root.display().to_string(),
        policy_receipt,
        active_task_kind_counts: active_task_kind_counts(&plan),
    })
}

fn summarize_workspace_evidence_graph(
    members: &[RustProjectHarnessWorkspaceEvidenceGraphMemberReceipt],
) -> RustProjectHarnessWorkspaceEvidenceGraphSummaryReceipt {
    RustProjectHarnessWorkspaceEvidenceGraphSummaryReceipt {
        member_crate_count: members.len(),
        dependency_baseline_package_count: members
            .iter()
            .map(|member| member.policy_receipt.dependency_baseline_packages.len())
            .sum(),
        active_verification_task_count: members
            .iter()
            .map(|member| member.policy_receipt.active_verification_task_count)
            .sum(),
        performance_task_count: members
            .iter()
            .map(|member| member.policy_receipt.performance_task_count)
            .sum(),
        stability_task_count: members
            .iter()
            .map(|member| member.policy_receipt.stability_task_count)
            .sum(),
        security_task_count: members
            .iter()
            .map(|member| {
                task_kind_count(
                    &member.active_task_kind_counts,
                    RustVerificationTaskKind::Security,
                )
            })
            .sum(),
        report_obligation_count: members
            .iter()
            .map(|member| member.policy_receipt.report_obligations.len())
            .sum(),
    }
}

fn build_workspace_evidence_graph_edges(
    workspace_label: &str,
    members: &[RustProjectHarnessWorkspaceEvidenceGraphMemberReceipt],
) -> (
    Vec<RustProjectHarnessWorkspaceEvidenceGraphNodeReceipt>,
    Vec<RustProjectHarnessWorkspaceEvidenceGraphEdgeReceipt>,
) {
    let workspace_node_id = workspace_evidence_node_id("workspace", workspace_label);
    let mut nodes = vec![workspace_evidence_node(
        &workspace_node_id,
        RustProjectHarnessWorkspaceEvidenceGraphNodeKind::Workspace,
        workspace_label,
    )];
    let mut edges = Vec::new();

    for member in members {
        let member_node_id =
            workspace_evidence_node_id("member_crate", &member.policy_receipt.gate_label);
        nodes.push(workspace_evidence_node(
            &member_node_id,
            RustProjectHarnessWorkspaceEvidenceGraphNodeKind::MemberCrate,
            &member.policy_receipt.gate_label,
        ));
        edges.push(workspace_evidence_edge(
            &workspace_node_id,
            &member_node_id,
            RustProjectHarnessWorkspaceEvidenceGraphEdgeKind::Contains,
        ));

        for count in &member.active_task_kind_counts {
            let task_kind = verification_task_kind_key(count.kind);
            let task_node_id = workspace_evidence_node_id(
                "verification_task_kind",
                &format!("{}:{task_kind}", member.policy_receipt.gate_label),
            );
            nodes.push(workspace_evidence_node(
                &task_node_id,
                RustProjectHarnessWorkspaceEvidenceGraphNodeKind::VerificationTaskKind,
                &format!("{} active {task_kind} tasks", count.count),
            ));
            edges.push(workspace_evidence_edge(
                &member_node_id,
                &task_node_id,
                RustProjectHarnessWorkspaceEvidenceGraphEdgeKind::RequiresVerification,
            ));
        }

        for package in &member.policy_receipt.dependency_baseline_packages {
            let package_node_id = workspace_evidence_node_id(
                "dependency_baseline_package",
                &format!(
                    "{}:{}:{}:{}",
                    member.policy_receipt.gate_label,
                    package.name,
                    package.version,
                    package.source_contains
                ),
            );
            nodes.push(workspace_evidence_node(
                &package_node_id,
                RustProjectHarnessWorkspaceEvidenceGraphNodeKind::DependencyBaselinePackage,
                &format!(
                    "{} {} {}",
                    package.name, package.version, package.source_contains
                ),
            ));
            edges.push(workspace_evidence_edge(
                &member_node_id,
                &package_node_id,
                RustProjectHarnessWorkspaceEvidenceGraphEdgeKind::RequiresDependencyBaseline,
            ));
        }

        for obligation in &member.policy_receipt.report_obligations {
            let report_node_id = workspace_evidence_node_id(
                "report_obligation",
                &format!("{}:{}", member.policy_receipt.gate_label, obligation.key),
            );
            nodes.push(workspace_evidence_node(
                &report_node_id,
                RustProjectHarnessWorkspaceEvidenceGraphNodeKind::ReportObligation,
                &obligation.key,
            ));
            edges.push(workspace_evidence_edge(
                &member_node_id,
                &report_node_id,
                RustProjectHarnessWorkspaceEvidenceGraphEdgeKind::RequiresReport,
            ));
            for kind in &obligation.task_kinds {
                let task_node_id = workspace_evidence_node_id(
                    "verification_task_kind",
                    &format!("{}:{kind}", member.policy_receipt.gate_label),
                );
                edges.push(workspace_evidence_edge(
                    &report_node_id,
                    &task_node_id,
                    RustProjectHarnessWorkspaceEvidenceGraphEdgeKind::Covers,
                ));
            }
        }
    }

    (nodes, edges)
}

fn build_workspace_trust_loop_steps(
    summary: &RustProjectHarnessWorkspaceEvidenceGraphSummaryReceipt,
    nodes: &[RustProjectHarnessWorkspaceEvidenceGraphNodeReceipt],
) -> Vec<RustProjectHarnessWorkspaceTrustLoopStepReceipt> {
    vec![
        workspace_trust_loop_step(
            "workspace_policy",
            if summary.member_crate_count > 0 {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::Configured
            } else {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::MissingMembers
            },
            nodes,
            &[
                RustProjectHarnessWorkspaceEvidenceGraphNodeKind::Workspace,
                RustProjectHarnessWorkspaceEvidenceGraphNodeKind::MemberCrate,
            ],
            "derive every member build.rs gate from the shared workspace policy",
        ),
        workspace_trust_loop_step(
            "dependency_baseline",
            if summary.dependency_baseline_package_count > 0 {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::Required
            } else {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::NotConfigured
            },
            nodes,
            &[RustProjectHarnessWorkspaceEvidenceGraphNodeKind::DependencyBaselinePackage],
            "pin critical git/version dependencies and keep Cargo.lock drift visible",
        ),
        workspace_trust_loop_step(
            "verification_plan",
            if summary.active_verification_task_count > 0 {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::Active
            } else {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::MissingActiveTasks
            },
            nodes,
            &[RustProjectHarnessWorkspaceEvidenceGraphNodeKind::VerificationTaskKind],
            "keep parser-owned verification tasks active for each member crate",
        ),
        workspace_trust_loop_step(
            "performance_stability_reports",
            if summary.performance_task_count > 0
                && summary.stability_task_count > 0
                && summary.report_obligation_count > 0
            {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::Required
            } else {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::Incomplete
            },
            nodes,
            &[RustProjectHarnessWorkspaceEvidenceGraphNodeKind::ReportObligation],
            "persist performance and stability report artifacts for regression comparison",
        ),
        workspace_trust_loop_step(
            "security_review",
            if summary.security_task_count > 0 {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::Active
            } else {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::NotConfigured
            },
            nodes,
            &[RustProjectHarnessWorkspaceEvidenceGraphNodeKind::VerificationTaskKind],
            "add security verification owners for security-critical APIs and dependency boundaries",
        ),
        workspace_trust_loop_step(
            "build_gate",
            if summary.member_crate_count > 0 {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::Enforced
            } else {
                RustProjectHarnessWorkspaceTrustLoopStepStatus::NotEnforced
            },
            nodes,
            &[RustProjectHarnessWorkspaceEvidenceGraphNodeKind::MemberCrate],
            "run member cargo test/check so each build.rs gate closes the loop before merge",
        ),
    ]
}

fn workspace_trust_loop_step(
    key: &str,
    status: RustProjectHarnessWorkspaceTrustLoopStepStatus,
    nodes: &[RustProjectHarnessWorkspaceEvidenceGraphNodeReceipt],
    evidence_kinds: &[RustProjectHarnessWorkspaceEvidenceGraphNodeKind],
    agent_action: &str,
) -> RustProjectHarnessWorkspaceTrustLoopStepReceipt {
    RustProjectHarnessWorkspaceTrustLoopStepReceipt {
        key: key.to_string(),
        status,
        evidence_node_ids: nodes
            .iter()
            .filter(|node| evidence_kinds.contains(&node.kind))
            .map(|node| node.id.clone())
            .collect(),
        agent_action: agent_action.to_string(),
    }
}

fn workspace_evidence_node(
    id: &str,
    kind: RustProjectHarnessWorkspaceEvidenceGraphNodeKind,
    label: &str,
) -> RustProjectHarnessWorkspaceEvidenceGraphNodeReceipt {
    RustProjectHarnessWorkspaceEvidenceGraphNodeReceipt {
        id: id.to_string(),
        kind,
        label: label.to_string(),
    }
}

fn workspace_evidence_edge(
    source: &str,
    target: &str,
    kind: RustProjectHarnessWorkspaceEvidenceGraphEdgeKind,
) -> RustProjectHarnessWorkspaceEvidenceGraphEdgeReceipt {
    RustProjectHarnessWorkspaceEvidenceGraphEdgeReceipt {
        source: source.to_string(),
        target: target.to_string(),
        kind,
    }
}

fn workspace_evidence_node_id(kind: &str, label: &str) -> String {
    format!("{kind}:{}", label.replace([' ', '/', '\\'], "_"))
}
