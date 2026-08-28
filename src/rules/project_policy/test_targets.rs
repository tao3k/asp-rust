//! Cargo test target policy.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{
    CargoManifestFacts, ParsedRustModule, RustTopLevelItemSyntax, path_line_location, source_line,
};
use crate::{RustHarnessFinding, RustHarnessRule};

use super::config::is_allowed_test_suite_path;
use super::support::display_project_path;
use super::{RUST_PROJ_R006, RUST_PROJ_R007, RUST_PROJ_R008};

const CARGO_TEST_GATE_MACROS: &[&str] = &[
    "rust_project_harness_gate",
    "rust_project_harness_cargo_test_gate",
];

pub(super) fn retired_test_target_gate_findings(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
    cargo_test_targets: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    if !cargo_manifest.references_harness {
        return Vec::new();
    }

    let rule = &rules[RUST_PROJ_R006];
    cargo_test_targets
        .iter()
        .flat_map(|parsed| {
            parsed
                .syntax_facts
                .macro_invocations
                .iter()
                .filter(|invocation| {
                    CARGO_TEST_GATE_MACROS.contains(&invocation.terminal_name.as_str())
                })
                .map(|invocation| {
                    RustHarnessFinding::from_rule(
                        rule,
                        format!(
                            "{} mounts a retired cargo-test harness gate.",
                            display_project_path(project_root, &parsed.report.path)
                        ),
                        path_line_location(&parsed.report.path, invocation.line),
                        source_line(&parsed.source, invocation.line),
                        "move parser-native harness policy to [build-dependencies] plus root build.rs using assert_rust_project_harness_cargo_check_clean_from_env_with_config(...), then keep this test target as a thin suite aggregate",
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(super) fn test_target_aggregate_findings(
    project_root: &Path,
    cargo_test_targets: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    let rule = &rules[RUST_PROJ_R007];
    for parsed in cargo_test_targets {
        if is_explicit_suite_leaf_target(project_root, &parsed.report.path) {
            continue;
        }
        for item in parsed
            .syntax_facts
            .top_level_items
            .iter()
            .filter(|item| !is_test_target_aggregate_item_syntax(item))
        {
            findings.push(RustHarnessFinding::from_rule(
                rule,
                format!(
                    "{} contains top-level implementation item `{}`.",
                    display_project_path(project_root, &parsed.report.path),
                    item.kind
                ),
                path_line_location(&parsed.report.path, item.line),
                source_line(&parsed.source, item.line),
                "move test implementation into a suite module and mount it from the root target",
            ));
        }
    }
    findings
}

pub(super) fn test_target_module_mount_findings(
    project_root: &Path,
    cargo_test_targets: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, RustHarnessRule>,
) -> Vec<RustHarnessFinding> {
    let mut findings = Vec::new();
    let rule = &rules[RUST_PROJ_R008];
    for parsed in cargo_test_targets {
        if is_explicit_suite_leaf_target(project_root, &parsed.report.path) {
            continue;
        }
        for item_mod in parsed
            .syntax_facts
            .top_level_items
            .iter()
            .filter_map(|item| item.module.as_ref())
            .filter(|item_mod| !item_mod.is_inline)
        {
            let Some(path_value) = item_mod.path_attr.as_deref() else {
                findings.push(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} declares root module `{}` without an explicit #[path].",
                        display_project_path(project_root, &parsed.report.path),
                        item_mod.ident
                    ),
                    path_line_location(&parsed.report.path, item_mod.line),
                    source_line(&parsed.source, item_mod.line),
                    "mount this root test module with #[path = \"suite/file.rs\"]",
                ));
                continue;
            };
            let Some(resolved) = item_mod.resolved_path_attr.as_ref() else {
                continue;
            };
            let allowed = is_allowed_resolved_test_suite_path(project_root, resolved)
                || parsed.report.path.parent().is_some_and(|parent| {
                    let candidate = parent.join(path_value);
                    is_allowed_resolved_test_suite_path(project_root, &candidate)
                });
            if !allowed {
                findings.push(RustHarnessFinding::from_rule(
                    rule,
                    format!(
                        "{} mounts `{path_value}`, but root test modules must resolve under an allowed tests suite directory.",
                        display_project_path(project_root, &parsed.report.path)
                    ),
                    path_line_location(&parsed.report.path, item_mod.line),
                    source_line(&parsed.source, item_mod.line),
                    "point this root test module at tests/unit, tests/integration, or another standard suite",
                ));
            }
        }
    }
    findings
}

fn is_explicit_suite_leaf_target(project_root: &Path, candidate: &Path) -> bool {
    let project_relative = candidate.strip_prefix(project_root).unwrap_or(candidate);
    is_allowed_test_suite_path(project_root, project_relative)
}

fn is_test_target_aggregate_item_syntax(item: &RustTopLevelItemSyntax) -> bool {
    item.is_macro || item.is_use || item.module.as_ref().is_some_and(|module| !module.is_inline)
}

fn is_allowed_resolved_test_suite_path(project_root: &Path, candidate: &Path) -> bool {
    let project_relative = candidate.strip_prefix(project_root).unwrap_or(candidate);
    candidate.exists() && is_allowed_test_suite_path(project_root, project_relative)
}
