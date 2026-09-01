//! Project scanner that builds verification profile candidates from parser facts.

use std::path::Path;

use crate::model::AspRustConfig;
use crate::verification::RustVerificationPolicy;
use crate::verification::analysis::{
    RustVerificationCargoDependencyAnalysis, analyze_rust_verification_project,
};

use super::collect::{PackageCandidateInput, collect_package_candidates};
use super::model::RustVerificationProfileIndex;

/// Build parser-suggested responsibility profile candidates for a Rust project.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index(
    project_root: &Path,
) -> Result<RustVerificationProfileIndex, String> {
    build_rust_verification_profile_index_with_config(project_root, &AspRustConfig::default())
}

/// Build parser-suggested responsibility profile candidates with config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index_with_config(
    project_root: &Path,
    config: &AspRustConfig,
) -> Result<RustVerificationProfileIndex, String> {
    build_rust_verification_profile_index_with_policy(
        project_root,
        config,
        &config.verification_policy,
    )
}

/// Build parser-suggested responsibility profile candidates with policy.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_profile_index_with_policy(
    project_root: &Path,
    config: &AspRustConfig,
    policy: &RustVerificationPolicy,
) -> Result<RustVerificationProfileIndex, String> {
    let analysis = analyze_rust_verification_project(
        project_root,
        config,
        RustVerificationCargoDependencyAnalysis::Parse,
    )?;
    let mut candidates = Vec::new();
    for package_analysis in &analysis.package_analyses {
        let reasoning_tree = &package_analysis.reasoning_tree;
        collect_package_candidates(
            PackageCandidateInput {
                project_root,
                package_root: &reasoning_tree.package_root,
                modules: &reasoning_tree.modules,
                branches: &reasoning_tree.owner_branches,
                cargo_dependencies: &package_analysis.cargo_dependencies,
                policy,
            },
            &mut candidates,
        );
    }
    candidates.sort_by(|left, right| {
        left.package_root
            .cmp(&right.package_root)
            .then_with(|| left.owner_path.cmp(&right.owner_path))
    });
    Ok(RustVerificationProfileIndex {
        project_root: project_root.to_path_buf(),
        candidates,
        configured_profile_hint_count: policy.profile_hints.len(),
    })
}
