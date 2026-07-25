//! Runner API for embedding the Rust project harness in tests and tools.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::discovery::{
    discover_cargo_package_roots, discover_rust_files, rust_project_harness_scope,
};
use crate::invariant_catalog::invariant_candidates_from_findings;
use crate::model::{RustHarnessConfig, RustHarnessReport, RustProjectHarnessScope};
use crate::parser::{ParsedRustModule, parse_rust_file};
use crate::rules::{
    evaluate_default_rule_packs_with_config, evaluate_workspace_rule_packs_with_config,
};

/// Select whether one harness run evaluates only the anchored package or expands project topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustHarnessRunScope {
    Package,
    ProjectWorkspace,
}

#[cfg(test)]
thread_local! {
    static ANALYZE_RUST_PROJECT_CALL_COUNT: std::cell::Cell<usize> = const {
        std::cell::Cell::new(0)
    };
}

#[cfg(test)]
pub(crate) fn reset_analyze_rust_project_call_count() {
    ANALYZE_RUST_PROJECT_CALL_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn analyze_rust_project_call_count() -> usize {
    ANALYZE_RUST_PROJECT_CALL_COUNT.with(std::cell::Cell::get)
}

/// One parser-owned package analysis shared by rule-pack and verification consumers.
pub(crate) struct RustHarnessPackageAnalysis {
    pub(crate) scope: RustProjectHarnessScope,
    pub(crate) parsed_modules: Vec<ParsedRustModule>,
}

/// A single filesystem discovery and parse pass for one requested harness scope.
pub struct RustHarnessAnalysis {
    pub(crate) project_root: PathBuf,
    pub(crate) project_scope: RustProjectHarnessScope,
    pub(crate) package_analyses: Vec<RustHarnessPackageAnalysis>,
    pub(crate) member_scoped: bool,
    pub(crate) parse_pass_count: usize,
}

/// Return the default Rust harness configuration.
#[must_use]
pub fn default_rust_harness_config() -> RustHarnessConfig {
    RustHarnessConfig::default()
}

/// Return the default Rust harness configuration merged with nearest `asp.toml`.
#[must_use]
pub fn rust_harness_config_for_project(project_root: &Path) -> RustHarnessConfig {
    apply_asp_project_config(project_root, RustHarnessConfig::default())
}

/// Run the harness with an explicit package-versus-workspace scope.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn run_rust_project_harness_for_scope(
    project_root: &Path,
    scope: RustHarnessRunScope,
) -> Result<RustHarnessReport, String> {
    run_rust_project_harness_with_config_for_scope(
        project_root,
        &rust_harness_config_for_project(project_root),
        scope,
    )
}

/// Run the harness with explicit config and package-versus-workspace scope.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn run_rust_project_harness_with_config_for_scope(
    project_root: &Path,
    config: &RustHarnessConfig,
    scope: RustHarnessRunScope,
) -> Result<RustHarnessReport, String> {
    Ok(analyze_rust_project_once(project_root, config, scope)?.to_report(config))
}

/// Analyze the project once with explicit config and package-versus-workspace scope.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn analyze_rust_project_once(
    project_root: &Path,
    config: &RustHarnessConfig,
    scope: RustHarnessRunScope,
) -> Result<RustHarnessAnalysis, String> {
    #[cfg(test)]
    ANALYZE_RUST_PROJECT_CALL_COUNT.with(|count| count.set(count.get().saturating_add(1)));
    if !project_root.exists() {
        return Err(format!(
            "project root does not exist: {}",
            project_root.display()
        ));
    }
    if scope == RustHarnessRunScope::Package {
        return Ok(analyze_single_project(project_root, config));
    }
    let package_roots = discover_cargo_package_roots(
        project_root,
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    );
    if should_run_member_scopes(project_root, &package_roots) {
        return Ok(analyze_member_scoped_project(
            project_root,
            &package_roots,
            config,
        ));
    }
    Ok(analyze_single_project(project_root, config))
}

/// Run the harness over explicit files or directories.
///
/// # Errors
///
/// Returns an error when any requested root does not exist.
pub fn run_rust_lang_harness(paths: &[PathBuf]) -> Result<RustHarnessReport, String> {
    run_rust_lang_harness_with_config(paths, &RustHarnessConfig::default())
}

/// Run the harness over explicit files or directories with explicit config.
///
/// # Errors
///
/// Returns an error when any requested root does not exist.
pub fn run_rust_lang_harness_with_config(
    paths: &[PathBuf],
    config: &RustHarnessConfig,
) -> Result<RustHarnessReport, String> {
    for path in paths {
        if !path.exists() {
            return Err(format!("harness path does not exist: {}", path.display()));
        }
    }
    Ok(run_paths(paths, config))
}

/// Assert a conventional Rust project harness run is clean.
///
/// # Panics
///
/// Panics when the run fails or when configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_project_harness_clean(project_root: &Path) -> RustHarnessReport {
    let report =
        run_rust_project_harness_for_scope(project_root, RustHarnessRunScope::ProjectWorkspace)
            .unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report
}

/// Assert a Cargo test project harness run is ready for agent repair.
///
/// This assertion treats configured-blocking findings and non-blocking agent
/// advice as actionable test feedback. It is intended for cargo-test gate
/// macros, while `assert_rust_project_harness_clean()` keeps the library runner
/// semantics of only blocking on configured severities.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_cargo_test_clean(project_root: &Path) -> RustHarnessReport {
    let report =
        run_rust_project_harness_for_scope(project_root, RustHarnessRunScope::ProjectWorkspace)
            .unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report.assert_no_advisory_findings();
    report
}

/// Assert a configured Rust project harness run is clean.
///
/// # Panics
///
/// Panics when the run fails or when configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_project_harness_clean_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    let report = run_rust_project_harness_with_config_for_scope(
        project_root,
        config,
        RustHarnessRunScope::ProjectWorkspace,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report
}

/// Assert a configured Cargo test project harness run is ready for agent repair.
///
/// # Panics
///
/// Panics when the run fails, when configured-blocking findings exist, or when
/// advisory findings exist.
#[track_caller]
pub fn assert_rust_project_harness_cargo_test_clean_with_config(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> RustHarnessReport {
    let report = run_rust_project_harness_with_config_for_scope(
        project_root,
        config,
        RustHarnessRunScope::ProjectWorkspace,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report.assert_no_advisory_findings();
    report
}

/// Assert an explicit-path Rust harness run is clean.
///
/// # Panics
///
/// Panics when the run fails or when configured-blocking findings exist.
#[track_caller]
pub fn assert_rust_lang_harness_clean(paths: &[PathBuf]) -> RustHarnessReport {
    let report = run_rust_lang_harness(paths).unwrap_or_else(|error| panic!("{error}"));
    report.assert_clean();
    report
}

fn run_paths(paths: &[PathBuf], config: &RustHarnessConfig) -> RustHarnessReport {
    let parsed_modules = parse_paths(paths, config);
    let findings = evaluate_default_rule_packs_with_config(None, &parsed_modules, config);
    let invariant_candidates = invariant_candidates_from_findings(&findings);
    RustHarnessReport {
        modules: parsed_modules
            .into_iter()
            .map(|module| module.report)
            .collect(),
        findings,
        invariant_candidates,
        root_paths: paths.to_vec(),
        blocking_severities: config.blocking_severities.clone(),
        project_scope: None,
        workspace_member_scopes: Vec::new(),
    }
}

fn analyze_single_project(project_root: &Path, config: &RustHarnessConfig) -> RustHarnessAnalysis {
    let scope = rust_project_harness_scope(
        project_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let monitored_paths = scope.monitored_paths();
    let parsed_modules = parse_paths(&monitored_paths, config);
    RustHarnessAnalysis {
        project_root: project_root.to_path_buf(),
        project_scope: scope.clone(),
        package_analyses: vec![RustHarnessPackageAnalysis {
            scope,
            parsed_modules,
        }],
        member_scoped: false,
        parse_pass_count: 1,
    }
}

fn analyze_member_scoped_project(
    project_root: &Path,
    package_roots: &[PathBuf],
    config: &RustHarnessConfig,
) -> RustHarnessAnalysis {
    let package_analyses = package_roots
        .iter()
        .map(|package_root| {
            let scope = rust_project_harness_scope(
                package_root,
                config.include_tests,
                &config.source_dir_names,
                &config.test_dir_names,
            );
            let parsed_modules = parse_paths(&scope.monitored_paths(), config);
            RustHarnessPackageAnalysis {
                scope,
                parsed_modules,
            }
        })
        .collect::<Vec<_>>();
    let parse_pass_count = package_analyses.len();
    RustHarnessAnalysis {
        project_root: project_root.to_path_buf(),
        project_scope: rust_project_harness_scope(
            project_root,
            config.include_tests,
            &config.source_dir_names,
            &config.test_dir_names,
        ),
        package_analyses,
        member_scoped: true,
        parse_pass_count,
    }
}

impl RustHarnessAnalysis {
    pub(crate) fn monitored_paths(&self) -> BTreeSet<PathBuf> {
        self.package_analyses
            .iter()
            .flat_map(|analysis| analysis.scope.monitored_paths())
            .collect()
    }

    /// Number of parser passes used to construct this analysis.
    #[must_use]
    pub const fn parse_pass_count(&self) -> usize {
        self.parse_pass_count
    }

    /// Project rule-pack findings from the already parsed analysis.
    #[must_use]
    pub fn to_report(&self, config: &RustHarnessConfig) -> RustHarnessReport {
        debug_assert_eq!(self.parse_pass_count, self.package_analyses.len());
        let mut modules = Vec::new();
        let mut findings = Vec::new();
        for analysis in &self.package_analyses {
            findings.extend(evaluate_default_rule_packs_with_config(
                Some(&analysis.scope),
                &analysis.parsed_modules,
                config,
            ));
            modules.extend(
                analysis
                    .parsed_modules
                    .iter()
                    .map(|module| module.report.clone()),
            );
        }
        let package_scopes = self
            .package_analyses
            .iter()
            .map(|analysis| analysis.scope.clone())
            .collect::<Vec<_>>();
        findings.extend(evaluate_workspace_rule_packs_with_config(
            &self.project_root,
            &package_scopes,
            config,
        ));
        if self.member_scoped {
            modules.sort_by(|left, right| left.path.cmp(&right.path));
        }
        let invariant_candidates = invariant_candidates_from_findings(&findings);
        let root_paths = if self.member_scoped {
            vec![self.project_root.clone()]
        } else {
            self.package_analyses[0].scope.monitored_paths()
        };
        let workspace_member_scopes = if self.member_scoped {
            self.package_analyses
                .iter()
                .map(|analysis| analysis.scope.clone())
                .collect()
        } else {
            Vec::new()
        };
        RustHarnessReport {
            modules,
            findings,
            invariant_candidates,
            root_paths,
            blocking_severities: config.blocking_severities.clone(),
            project_scope: Some(self.project_scope.clone()),
            workspace_member_scopes,
        }
    }
}

fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}

fn parse_paths(paths: &[PathBuf], config: &RustHarnessConfig) -> Vec<ParsedRustModule> {
    discover_rust_files(
        paths,
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    )
    .into_iter()
    .map(|path| parse_rust_file(&path))
    .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AspProjectConfig {
    #[serde(default)]
    discovery: AspDiscoveryConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AspDiscoveryConfig {
    #[serde(default)]
    ignored_dir_names: BTreeSet<String>,
    #[serde(default)]
    include_hidden_dir_names: BTreeSet<String>,
}

fn apply_asp_project_config(
    project_root: &Path,
    mut config: RustHarnessConfig,
) -> RustHarnessConfig {
    let Some(config_path) = nearest_asp_toml(project_root) else {
        return config;
    };
    let Ok(contents) = std::fs::read_to_string(config_path) else {
        return config;
    };
    let Ok(parsed) = toml::from_str::<AspProjectConfig>(&contents) else {
        return config;
    };
    config
        .ignored_dir_names
        .extend(parsed.discovery.ignored_dir_names);
    config
        .include_hidden_dir_names
        .extend(parsed.discovery.include_hidden_dir_names);
    config
}

fn nearest_asp_toml(project_root: &Path) -> Option<PathBuf> {
    project_root
        .ancestors()
        .map(|ancestor| ancestor.join("asp.toml"))
        .find(|candidate| candidate.is_file())
}
