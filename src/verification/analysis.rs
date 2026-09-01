//! Shared `Verification` analysis facts and scale profiles for Agent receipts.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::AspRustScope;
use crate::model::AspRustConfig;
use crate::parser::{
    CargoDependencyFacts, ParsedRustModule, RustReasoningTreeFacts, parse_cargo_dependency_facts,
    rust_reasoning_tree_facts,
};

pub(super) struct RustVerificationProjectAnalysis {
    pub(super) package_analyses: Vec<RustVerificationPackageAnalysis>,
    pub(super) profile: RustVerificationAnalysisProfile,
}

pub(super) struct RustVerificationPackageAnalysis {
    pub(super) reasoning_tree: RustReasoningTreeFacts,
    pub(super) cargo_dependencies: Vec<CargoDependencyFacts>,
    profile: RustVerificationPackageAnalysisProfile,
}

#[derive(Clone, Copy)]
pub(super) enum RustVerificationCargoDependencyAnalysis {
    Skip,
    Parse,
}

/// Project-scale and runtime profile for one verification analysis pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationAnalysisProfile {
    /// Project root that was analyzed.
    pub project_root: PathBuf,
    /// End-to-end analysis duration in microseconds.
    pub elapsed_micros: u64,
    /// Number of package scopes analyzed.
    pub package_count: usize,
    /// Number of parsed Rust files across package scopes.
    pub rust_file_count: usize,
    /// Number of parser-known source modules across package scopes.
    pub source_module_count: usize,
    /// Number of owner branches derived by the reasoning tree.
    pub owner_branch_count: usize,
    /// Number of Cargo dependency facts parsed for profile inference.
    pub cargo_dependency_count: usize,
    /// Per-package analysis profile.
    pub packages: Vec<RustVerificationPackageAnalysisProfile>,
}

/// Package-scale and runtime profile for one verification analysis pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationPackageAnalysisProfile {
    /// Cargo package root for this analysis scope.
    pub package_root: PathBuf,
    /// Package analysis duration in microseconds.
    pub elapsed_micros: u64,
    /// Number of parsed Rust files in this package scope.
    pub rust_file_count: usize,
    /// Number of parser-known source modules in this package scope.
    pub source_module_count: usize,
    /// Number of owner branches derived by the reasoning tree.
    pub owner_branch_count: usize,
    /// Number of Cargo dependency facts parsed for profile inference.
    pub cargo_dependency_count: usize,
}

#[derive(Default)]
struct RustVerificationAnalysisTotals {
    rust_file_count: usize,
    source_module_count: usize,
    owner_branch_count: usize,
    cargo_dependency_count: usize,
}

impl RustVerificationAnalysisProfile {
    fn from_package_profiles(
        project_root: &Path,
        packages: Vec<RustVerificationPackageAnalysisProfile>,
        elapsed_micros: u64,
    ) -> Self {
        let totals = packages.iter().fold(
            RustVerificationAnalysisTotals::default(),
            |mut totals, package| {
                totals.record_package(package);
                totals
            },
        );
        Self {
            project_root: project_root.to_path_buf(),
            elapsed_micros,
            package_count: packages.len(),
            rust_file_count: totals.rust_file_count,
            source_module_count: totals.source_module_count,
            owner_branch_count: totals.owner_branch_count,
            cargo_dependency_count: totals.cargo_dependency_count,
            packages,
        }
    }
}

impl RustVerificationAnalysisTotals {
    fn record_package(&mut self, package: &RustVerificationPackageAnalysisProfile) {
        self.rust_file_count += package.rust_file_count;
        self.source_module_count += package.source_module_count;
        self.owner_branch_count += package.owner_branch_count;
        self.cargo_dependency_count += package.cargo_dependency_count;
    }
}

impl RustVerificationPackageAnalysisProfile {
    fn from_analysis(
        package_root: &Path,
        parsed_modules: &[ParsedRustModule],
        reasoning_tree: &RustReasoningTreeFacts,
        cargo_dependencies: &[CargoDependencyFacts],
        elapsed_micros: u64,
    ) -> Self {
        Self {
            package_root: package_root.to_path_buf(),
            elapsed_micros,
            rust_file_count: parsed_modules.len(),
            source_module_count: reasoning_tree
                .modules
                .iter()
                .filter(|module| module.is_source_module)
                .count(),
            owner_branch_count: reasoning_tree.owner_branches.len(),
            cargo_dependency_count: cargo_dependencies.len(),
        }
    }
}

/// Build a verification analysis profile with the default harness config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_analysis_profile(
    project_root: &Path,
) -> Result<RustVerificationAnalysisProfile, String> {
    build_rust_verification_analysis_profile_with_config(project_root, &AspRustConfig::default())
}

/// Build a verification analysis profile with Cargo dependency facts enabled.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_verification_analysis_profile_with_config(
    project_root: &Path,
    config: &AspRustConfig,
) -> Result<RustVerificationAnalysisProfile, String> {
    analyze_rust_verification_project(
        project_root,
        config,
        RustVerificationCargoDependencyAnalysis::Parse,
    )
    .map(|analysis| analysis.profile)
}

/// Render a verification analysis profile as structured JSON.
///
/// # Errors
///
/// Returns a serialization error if the profile cannot be encoded as JSON.
pub fn render_rust_verification_analysis_profile_json(
    profile: &RustVerificationAnalysisProfile,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(profile)
}

/// Render a compact verification analysis profile for Agent prompts.
#[must_use]
pub fn render_rust_verification_analysis_profile(
    profile: &RustVerificationAnalysisProfile,
) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "[verify-analysis] packages={} rust_files={} source_modules={} owner_branches={} cargo_dependencies={} elapsed_us={}",
        profile.package_count,
        profile.rust_file_count,
        profile.source_module_count,
        profile.owner_branch_count,
        profile.cargo_dependency_count,
        profile.elapsed_micros
    );
    for package in &profile.packages {
        let _ = writeln!(
            rendered,
            "   |package: {} rust_files={} source_modules={} owner_branches={} cargo_dependencies={} elapsed_us={}",
            compact_package_root(&profile.project_root, &package.package_root),
            package.rust_file_count,
            package.source_module_count,
            package.owner_branch_count,
            package.cargo_dependency_count,
            package.elapsed_micros
        );
    }
    rendered
}

pub(super) fn analyze_rust_verification_project(
    project_root: &Path,
    config: &AspRustConfig,
    cargo_dependency_analysis: RustVerificationCargoDependencyAnalysis,
) -> Result<RustVerificationProjectAnalysis, String> {
    let harness_analysis = crate::runner::analyze_rust_project_once(
        project_root,
        config,
        crate::runner::AspRustRunScope::ProjectWorkspace,
    )?;
    Ok(analyze_rust_verification_from_harness_analysis(
        harness_analysis,
        cargo_dependency_analysis,
    ))
}

pub(super) fn analyze_rust_verification_from_harness_analysis(
    harness_analysis: crate::runner::AspRustAnalysis,
    cargo_dependency_analysis: RustVerificationCargoDependencyAnalysis,
) -> RustVerificationProjectAnalysis {
    let started_at = Instant::now();
    let package_analyses = harness_analysis
        .package_analyses
        .iter()
        .map(|package| {
            let cargo_dependencies = match cargo_dependency_analysis {
                RustVerificationCargoDependencyAnalysis::Skip => Vec::new(),
                RustVerificationCargoDependencyAnalysis::Parse => {
                    parse_cargo_dependency_facts(&package.scope.project_root)
                }
            };
            rust_verification_package_analysis_from_parsed(
                &package.scope,
                &package.parsed_modules,
                cargo_dependencies,
                Instant::now(),
            )
        })
        .collect::<Vec<_>>();
    let package_profiles = package_analyses
        .iter()
        .map(|analysis| analysis.profile.clone())
        .collect();
    let profile = RustVerificationAnalysisProfile::from_package_profiles(
        &harness_analysis.project_root,
        package_profiles,
        elapsed_micros(started_at.elapsed()),
    );
    RustVerificationProjectAnalysis {
        package_analyses,
        profile,
    }
}

fn rust_verification_package_analysis_from_parsed(
    scope: &AspRustScope,
    parsed_modules: &[ParsedRustModule],
    cargo_dependencies: Vec<CargoDependencyFacts>,
    started_at: Instant,
) -> RustVerificationPackageAnalysis {
    let reasoning_tree = rust_reasoning_tree_facts(scope, parsed_modules);
    let profile = RustVerificationPackageAnalysisProfile::from_analysis(
        &reasoning_tree.package_root,
        parsed_modules,
        &reasoning_tree,
        &cargo_dependencies,
        elapsed_micros(started_at.elapsed()),
    );
    RustVerificationPackageAnalysis {
        reasoning_tree,
        cargo_dependencies,
        profile,
    }
}

fn elapsed_micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn compact_package_root(project_root: &Path, package_root: &Path) -> String {
    match package_root.strip_prefix(project_root) {
        Ok(relative_path) if relative_path.as_os_str().is_empty() => ".".to_string(),
        Ok(relative_path) => display_path(relative_path),
        Err(_) => display_path(package_root),
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
