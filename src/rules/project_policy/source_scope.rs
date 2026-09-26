//! Harness scope configuration policy.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::parser::{CargoManifestFacts, file_location};
use crate::{AspRustConfig, AspRustFinding, AspRustRule};

use super::support::display_project_path;
use super::{RUST_PROJ_R013, RUST_PROJ_R014};

const DEFAULT_SOURCE_PATHS: &[&str] = &["src"];
const DEFAULT_TEST_PATHS: &[&str] = &["tests"];

pub(super) fn source_scope_findings(
    project_root: &Path,
    config: &AspRustConfig,
    cargo_manifest: &CargoManifestFacts,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let custom_rule = &rules[RUST_PROJ_R013];
    let reduction_rule = &rules[RUST_PROJ_R014];
    let mut findings = Vec::new();

    findings.extend(
        config
            .source_dir_names
            .iter()
            .filter(|path| {
                custom_path_lacks_explanation(
                    path,
                    DEFAULT_SOURCE_PATHS,
                    &config.source_path_explanations,
                )
            })
            .map(|path| custom_scope_path_finding(project_root, path, "source", custom_rule)),
    );
    findings.extend(
        config
            .test_dir_names
            .iter()
            .filter(|path| {
                custom_path_lacks_explanation(
                    path,
                    DEFAULT_TEST_PATHS,
                    &config.test_path_explanations,
                )
            })
            .map(|path| custom_scope_path_finding(project_root, path, "test", custom_rule)),
    );
    findings.extend(default_scope_reduction_findings(
        project_root,
        config,
        cargo_manifest,
        reduction_rule,
    ));
    findings
}

fn custom_path_lacks_explanation(
    path: &str,
    default_paths: &[&str],
    explanations: &BTreeMap<String, String>,
) -> bool {
    let normalized = normalize_scope_path(path);
    !normalized.is_empty()
        && !default_paths.contains(&normalized.as_str())
        && explanations
            .get(&normalized)
            .is_none_or(|explanation| explanation.trim().is_empty())
}

fn custom_scope_path_finding(
    project_root: &Path,
    path: &str,
    scope_kind: &str,
    rule: &AspRustRule,
) -> AspRustFinding {
    let normalized = normalize_scope_path(path);
    let absolute_path = project_root.join(&normalized);
    AspRustFinding::from_rule(
        rule,
        format!(
            "Config adds custom {scope_kind} scope path `{}` without a non-empty explanation.",
            display_project_path(project_root, &absolute_path)
        ),
        file_location(if absolute_path.exists() {
            absolute_path
        } else {
            project_root.to_path_buf()
        }),
        None,
        "use with_source_path(path, explanation) or with_test_path(path, explanation) so the Agent explains why this scope is necessary",
    )
}

fn default_scope_reduction_findings(
    project_root: &Path,
    config: &AspRustConfig,
    cargo_manifest: &CargoManifestFacts,
    rule: &AspRustRule,
) -> Vec<AspRustFinding> {
    let source_paths = normalized_paths(&config.source_dir_names);
    let test_paths = normalized_paths(&config.test_dir_names);
    let mut findings = Vec::new();

    for &default_path in DEFAULT_SOURCE_PATHS {
        if default_scope_needs_explanation(
            project_root,
            default_path,
            &source_paths,
            &config.source_path_exclusion_explanations,
        ) {
            let default_source_path = project_root.join(default_path);
            findings.push(default_scope_reduction_finding(
                project_root,
                &default_source_path,
                "source",
                "with_source_path_excluded(path, explanation)",
                rule,
            ));
        }
    }

    for &default_path in DEFAULT_TEST_PATHS {
        let default_test_path = project_root.join(default_path);
        let cargo_test_targets = existing_cargo_test_targets(cargo_manifest);
        let has_cargo_test_coverage = default_test_path.exists() || !cargo_test_targets.is_empty();
        if !config.include_tests {
            if has_cargo_test_coverage
                && explanation_is_missing(default_path, &config.test_path_exclusion_explanations)
            {
                let finding_path = if default_test_path.exists() {
                    default_test_path
                } else {
                    cargo_test_targets[0].clone()
                };
                findings.push(default_scope_reduction_finding(
                    project_root,
                    &finding_path,
                    "test",
                    "with_tests_excluded(explanation)",
                    rule,
                ));
            }
            continue;
        }
        if !default_test_path.exists() {
            continue;
        }
        if !test_paths.contains(default_path)
            && explanation_is_missing(default_path, &config.test_path_exclusion_explanations)
        {
            findings.push(default_scope_reduction_finding(
                project_root,
                &default_test_path,
                "test",
                "with_test_path_excluded(path, explanation)",
                rule,
            ));
        }
    }

    findings
}

fn default_scope_needs_explanation(
    project_root: &Path,
    default_path: &str,
    configured_paths: &BTreeSet<String>,
    explanations: &BTreeMap<String, String>,
) -> bool {
    project_root.join(default_path).exists()
        && !configured_paths.contains(default_path)
        && explanation_is_missing(default_path, explanations)
}

fn default_scope_reduction_finding(
    project_root: &Path,
    path: &Path,
    scope_kind: &str,
    fix: &str,
    rule: &AspRustRule,
) -> AspRustFinding {
    AspRustFinding::from_rule(
        rule,
        format!(
            "Config excludes Cargo-backed {scope_kind} scope path `{}` without a non-empty explanation.",
            display_project_path(project_root, path)
        ),
        file_location(path.to_path_buf()),
        None,
        format!("use {fix} so the Agent explains why this Cargo-backed scope is reduced"),
    )
}

fn existing_cargo_test_targets(cargo_manifest: &CargoManifestFacts) -> Vec<PathBuf> {
    cargo_manifest
        .test_target_files
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect()
}

fn explanation_is_missing(path: &str, explanations: &BTreeMap<String, String>) -> bool {
    explanations
        .get(path)
        .is_none_or(|explanation| explanation.trim().is_empty())
}

fn normalized_paths(paths: &[String]) -> BTreeSet<String> {
    paths
        .iter()
        .map(|path| normalize_scope_path(path))
        .collect()
}

fn normalize_scope_path(path: &str) -> String {
    path.trim().trim_matches('/').replace('\\', "/")
}
