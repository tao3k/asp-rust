//! Project-level Rust test policy rule catalog and evaluator.

use crate::parser::{
    ParsedRustModule, parse_cargo_manifest, parse_cargo_test_targets, rust_reasoning_tree_facts,
};
use crate::{AspRustConfig, AspRustFinding, AspRustRule, AspRustScope};

use super::build_gate::build_gate_findings;
use super::catalog::rules_by_id;
use super::manifest::manifest_findings;
use super::quality::quality_findings;
use super::source_scope::source_scope_findings;
use super::source_tests::source_test_mount_findings;
use super::test_bloat::test_bloat_findings;
use super::test_layout::test_layout_findings;
use super::test_targets::{
    retired_test_target_gate_findings, test_target_aggregate_findings,
    test_target_module_mount_findings,
};
use super::verification_integration::verification_integration_findings;

pub(crate) const PACK_ID: &str = "rust.project_policy";
pub(crate) const RUST_PROJ_R001: &str = "RUST-AGENT-PROJECT-001";
pub(crate) const RUST_PROJ_R002: &str = "RUST-AGENT-PROJECT-002";
pub(crate) const RUST_PROJ_R003: &str = "RUST-AGENT-PROJECT-003";
pub(crate) const RUST_PROJ_R004: &str = "RUST-AGENT-PROJECT-004";
pub(crate) const RUST_PROJ_R005: &str = "RUST-AGENT-PROJECT-005";
pub(crate) const RUST_PROJ_R006: &str = "RUST-AGENT-PROJECT-006";
pub(crate) const RUST_PROJ_R007: &str = "RUST-AGENT-PROJECT-007";
pub(crate) const RUST_PROJ_R008: &str = "RUST-AGENT-PROJECT-008";
pub(crate) const RUST_PROJ_R009: &str = "RUST-AGENT-PROJECT-009";
pub(crate) const RUST_PROJ_R010: &str = "RUST-AGENT-PROJECT-010";
pub(crate) const RUST_PROJ_R011: &str = "RUST-AGENT-PROJECT-011";
pub(crate) const RUST_PROJ_R012: &str = "RUST-AGENT-PROJECT-012";
pub(crate) const RUST_PROJ_R013: &str = "RUST-AGENT-PROJECT-013";
pub(crate) const RUST_PROJ_R014: &str = "RUST-AGENT-PROJECT-014";
pub(crate) const RUST_PROJ_R015: &str = "RUST-AGENT-PROJECT-015";
pub(crate) const RUST_PROJ_R016: &str = "RUST-AGENT-PROJECT-016";
pub(crate) const RUST_PROJ_R017: &str = "RUST-AGENT-PROJECT-017";
pub(crate) const RUST_PROJ_R018: &str = "RUST-AGENT-PROJECT-018";
pub(crate) const RUST_PROJ_R019: &str = "RUST-AGENT-PROJECT-019";
pub(crate) const RUST_PROJ_R020: &str = "RUST-AGENT-PROJECT-020";
pub(crate) const RUST_PROJ_R021: &str = "RUST-AGENT-PROJECT-021";
pub(crate) const RUST_PROJ_R022: &str = "RUST-AGENT-PROJECT-022";
pub(crate) const RUST_PROJ_R023: &str = "RUST-AGENT-PROJECT-MANIFEST-023";
pub(crate) const RUST_PROJ_R024: &str = "RUST-AGENT-PROJECT-024";

pub(crate) const MAX_UNIT_TEST_EFFECTIVE_LINES: usize = 1000;
pub(crate) const MIN_UNIT_TEST_FUNCTIONS: usize = 8;
pub(crate) const MAX_INTEGRATION_TEST_EFFECTIVE_LINES: usize = 1000;
pub(crate) const MIN_INTEGRATION_TEST_FUNCTIONS: usize = 12;
pub(crate) const MAX_TEST_SUPPORT_EFFECTIVE_LINES: usize = 1000;

/// Return compact metadata for Rust project-policy rules.
#[must_use]
pub fn rust_project_policy_rules() -> Vec<AspRustRule> {
    rules_by_id().into_values().collect()
}

pub(crate) fn evaluate_workspace(
    workspace_root: &std::path::Path,
    package_scopes: &[AspRustScope],
    config: &AspRustConfig,
) -> Vec<AspRustFinding> {
    let rules = rules_by_id();
    super::verification_integration::workspace_performance_verification_findings(
        workspace_root,
        package_scopes,
        config,
        &rules,
    )
}

pub(crate) fn evaluate(
    scope: Option<&AspRustScope>,
    modules: &[ParsedRustModule],
    config: &AspRustConfig,
) -> Vec<AspRustFinding> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let rules = rules_by_id();
    let mut findings = Vec::new();
    let cargo_manifest = parse_cargo_manifest(&scope.project_root);
    let cargo_test_targets = parse_cargo_test_targets(&scope.project_root, &cargo_manifest);
    let reasoning_tree = rust_reasoning_tree_facts(scope, modules);
    findings.extend(test_layout_findings(&scope.project_root, &rules));
    findings.extend(manifest_findings(
        &scope.project_root,
        &cargo_manifest,
        &rules,
    ));
    findings.extend(source_scope_findings(
        &scope.project_root,
        config,
        &cargo_manifest,
        &rules,
    ));
    findings.extend(source_test_mount_findings(scope, modules, &rules));
    findings.extend(test_bloat_findings(
        &scope.project_root,
        modules,
        &reasoning_tree,
        &rules,
    ));
    findings.extend(test_target_aggregate_findings(
        &scope.project_root,
        &cargo_test_targets,
        &rules,
    ));
    findings.extend(test_target_module_mount_findings(
        &scope.project_root,
        &cargo_test_targets,
        &rules,
    ));
    findings.extend(retired_test_target_gate_findings(
        &scope.project_root,
        &cargo_manifest,
        &cargo_test_targets,
        &rules,
    ));
    findings.extend(verification_integration_findings(
        &scope.project_root,
        &reasoning_tree,
        config,
        modules,
        &cargo_manifest,
        &rules,
    ));
    findings.extend(quality_findings(
        &scope.project_root,
        config,
        modules,
        &rules,
    ));
    findings.extend(build_gate_findings(
        &scope.project_root,
        &cargo_manifest,
        modules,
        &rules,
    ));
    findings
}
