//! Workspace-level composition of package-atomic Rust test gates.

use std::path::{Path, PathBuf};

use crate::AspRustReport;
use crate::workspace_build_dag::{AspRustWorkspaceBuildDag, asp_rust_workspace_build_dag};

/// Result of explicitly composing package-atomic downstream gates for a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AspRustWorkspaceRunReport {
    /// Cargo workspace root whose manifest admitted every member.
    pub workspace_root: PathBuf,
    /// Build DAG that admitted the package atoms.
    pub build_dag: AspRustWorkspaceBuildDag,
    /// Independently evaluated package reports.
    pub members: Vec<AspRustWorkspaceMemberRunReport>,
}

/// One package atom in an explicit workspace run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AspRustWorkspaceMemberRunReport {
    /// Caller-owned member label.
    pub crate_label: String,
    /// Cargo package root admitted for this atom.
    pub project_root: PathBuf,
    /// Package-scoped harness report.
    pub report: AspRustReport,
}

/// Assert one Cargo workspace instance as independent package atoms.
///
/// Package membership and ordering come only from parsed Cargo manifests. The
/// shared workspace policy is derived once per Build DAG package; diamond
/// dependencies are evaluated once, never once per incoming edge.
///
/// # Panics
///
/// Panics when Cargo graph construction fails or one Build DAG package fails.
#[track_caller]
pub fn assert_asp_rust_workspace_policy(
    workspace_root: &Path,
    workspace_policy: &crate::dev_gate::AspRustWorkspacePolicy,
) -> AspRustWorkspaceRunReport {
    assert_asp_rust_workspace_policy_with(workspace_root, workspace_policy, |_, config| config)
}

/// Evaluate one Cargo workspace without rejecting advisory or blocking findings.
///
/// This is the reporting half of the workspace Dev Gate. Build Support uses it
/// for `mode = warn`; `mode = deny` uses the corresponding assertion API.
#[must_use]
pub fn evaluate_asp_rust_workspace_policy(
    workspace_root: &Path,
    workspace_policy: &crate::dev_gate::AspRustWorkspacePolicy,
) -> AspRustWorkspaceRunReport {
    evaluate_asp_rust_workspace_policy_with(workspace_root, workspace_policy, |_, config| config)
}

/// Assert one Cargo workspace while applying one package-local config projection.
///
/// Cargo workspace discovery, membership, dependency ordering, and cache
/// ownership stay inside ASP Rust. Downstream Build Support supplies only the
/// declarative package override.
#[track_caller]
pub fn assert_asp_rust_workspace_policy_with<F>(
    workspace_root: &Path,
    workspace_policy: &crate::dev_gate::AspRustWorkspacePolicy,
    configure_member: F,
) -> AspRustWorkspaceRunReport
where
    F: FnMut(&str, crate::AspRustConfig) -> crate::AspRustConfig,
{
    let report =
        evaluate_asp_rust_workspace_policy_with(workspace_root, workspace_policy, configure_member);
    assert_workspace_report_clean(&report, workspace_policy.workspace_label());
    report
}

/// Evaluate one Cargo workspace with a package-local config projection.
#[must_use]
pub fn evaluate_asp_rust_workspace_policy_with<F>(
    workspace_root: &Path,
    workspace_policy: &crate::dev_gate::AspRustWorkspacePolicy,
    configure_member: F,
) -> AspRustWorkspaceRunReport
where
    F: FnMut(&str, crate::AspRustConfig) -> crate::AspRustConfig,
{
    let build_dag = asp_rust_workspace_build_dag(workspace_root, workspace_policy.config())
        .unwrap_or_else(|error| panic!("ASP Rust workspace dependency graph: {error}"));
    evaluate_asp_rust_workspace_build_dag_policy_with(build_dag, workspace_policy, configure_member)
}

/// Assert a pre-derived Cargo Build DAG without rediscovering workspace packages.
///
/// This is the Build Support test boundary: the workspace owner derives the Cargo DAG
/// exactly once, then evaluates every package atom exactly once against that
/// immutable graph. Downstream packages never compile or invoke a second full
/// source scanner.
#[track_caller]
pub fn assert_asp_rust_workspace_build_dag_policy_with<F>(
    build_dag: AspRustWorkspaceBuildDag,
    workspace_policy: &crate::dev_gate::AspRustWorkspacePolicy,
    configure_member: F,
) -> AspRustWorkspaceRunReport
where
    F: FnMut(&str, crate::AspRustConfig) -> crate::AspRustConfig,
{
    let report = evaluate_asp_rust_workspace_build_dag_policy_with(
        build_dag,
        workspace_policy,
        configure_member,
    );
    assert_workspace_report_clean(&report, workspace_policy.workspace_label());
    report
}

/// Evaluate a pre-derived Cargo Build DAG without rejecting its findings.
#[must_use]
pub fn evaluate_asp_rust_workspace_build_dag_policy_with<F>(
    build_dag: AspRustWorkspaceBuildDag,
    workspace_policy: &crate::dev_gate::AspRustWorkspacePolicy,
    mut configure_member: F,
) -> AspRustWorkspaceRunReport
where
    F: FnMut(&str, crate::AspRustConfig) -> crate::AspRustConfig,
{
    let workspace_root = build_dag.workspace_root.clone();
    let mut reports = Vec::new();
    for package in &build_dag.packages {
        let policy = workspace_policy.member_crate_with_config(&package.package_name, |config| {
            configure_member(&package.package_name, config)
        });
        let report =
            crate::dev_gate::evaluate_asp_rust_downstream_policy(&package.package_root, &policy);
        assert!(
            report
                .root_paths
                .iter()
                .all(|path| path.starts_with(&package.package_root)),
            "workspace member gate escaped package atom {}",
            package.package_root.display()
        );
        reports.push(AspRustWorkspaceMemberRunReport {
            crate_label: package.package_name.clone(),
            project_root: package.package_root.clone(),
            report,
        });
    }
    AspRustWorkspaceRunReport {
        workspace_root,
        build_dag,
        members: reports,
    }
}

fn assert_workspace_report_clean(report: &AspRustWorkspaceRunReport, workspace_label: &str) {
    let rejections = report
        .members
        .iter()
        .filter(|member| !member.report.is_clean())
        .map(|member| {
            format!(
                "[{}]\n{}",
                format_args!("{workspace_label}::{}", member.crate_label),
                crate::render_asp_rust(&member.report)
            )
        })
        .collect::<Vec<_>>();
    assert!(
        rejections.is_empty(),
        "ASP Rust workspace policy rejected {} of {} package atoms:\n{}",
        rejections.len(),
        report.members.len(),
        rejections.join("\n\n")
    );
}

/// Assert the single Cargo workspace instance owning `CARGO_MANIFEST_DIR`.
#[track_caller]
pub fn assert_asp_rust_workspace_policy_from_env(
    workspace_policy: &crate::dev_gate::AspRustWorkspacePolicy,
) -> AspRustWorkspaceRunReport {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR is required for ASP Rust workspace policy"));
    let workspace_root = crate::parser::find_required_cargo_workspace_root(&manifest_dir)
        .unwrap_or_else(|error| panic!("resolve ASP Rust workspace instance: {error}"));
    assert_asp_rust_workspace_policy(&workspace_root, workspace_policy)
}

/// Evaluate the Cargo workspace owning `CARGO_MANIFEST_DIR` without rejecting findings.
#[must_use]
pub fn evaluate_asp_rust_workspace_policy_from_env(
    workspace_policy: &crate::dev_gate::AspRustWorkspacePolicy,
) -> AspRustWorkspaceRunReport {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR is required for ASP Rust workspace policy"));
    let workspace_root = crate::parser::find_required_cargo_workspace_root(&manifest_dir)
        .unwrap_or_else(|error| panic!("resolve ASP Rust workspace instance: {error}"));
    evaluate_asp_rust_workspace_policy(&workspace_root, workspace_policy)
}
