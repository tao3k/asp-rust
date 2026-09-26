//! Test leaf bloat policy.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{ParsedRustModule, RustReasoningTreeFacts, file_location};
use crate::{AspRustFinding, AspRustRule};

use super::support::display_project_path;
use super::{
    MAX_INTEGRATION_TEST_EFFECTIVE_LINES, MAX_TEST_SUPPORT_EFFECTIVE_LINES,
    MAX_UNIT_TEST_EFFECTIVE_LINES, MIN_INTEGRATION_TEST_FUNCTIONS, MIN_UNIT_TEST_FUNCTIONS,
    RUST_PROJ_R005, RUST_PROJ_R024,
};

struct LeafBloatLimits {
    suite_name: &'static str,
    max_effective_lines: usize,
    min_test_functions: usize,
}

pub(super) fn test_bloat_findings(
    project_root: &Path,
    modules: &[ParsedRustModule],
    reasoning_tree: &RustReasoningTreeFacts,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut findings = Vec::new();
    collect_leaf_bloat_findings(
        project_root,
        LeafBloatLimits {
            suite_name: "unit",
            max_effective_lines: MAX_UNIT_TEST_EFFECTIVE_LINES,
            min_test_functions: MIN_UNIT_TEST_FUNCTIONS,
        },
        modules,
        reasoning_tree,
        rules,
        &mut findings,
    );
    collect_leaf_bloat_findings(
        project_root,
        LeafBloatLimits {
            suite_name: "integration",
            max_effective_lines: MAX_INTEGRATION_TEST_EFFECTIVE_LINES,
            min_test_functions: MIN_INTEGRATION_TEST_FUNCTIONS,
        },
        modules,
        reasoning_tree,
        rules,
        &mut findings,
    );
    collect_support_bloat_findings(
        project_root,
        "unit",
        modules,
        reasoning_tree,
        rules,
        &mut findings,
    );
    collect_support_bloat_findings(
        project_root,
        "integration",
        modules,
        reasoning_tree,
        rules,
        &mut findings,
    );
    findings
}

fn collect_leaf_bloat_findings(
    project_root: &Path,
    limits: LeafBloatLimits,
    modules: &[ParsedRustModule],
    reasoning_tree: &RustReasoningTreeFacts,
    rules: &BTreeMap<&'static str, AspRustRule>,
    findings: &mut Vec<AspRustFinding>,
) {
    for module in modules {
        let Some(module_facts) = reasoning_tree.module(&module.report.path) else {
            continue;
        };
        if !module.report.is_valid
            || !module_facts.source_path.is_test_source
            || !is_under_suite(project_root, &module.report.path, limits.suite_name)
        {
            continue;
        }
        let effective_lines = module.source_metrics.effective_code_lines;
        if effective_lines < limits.max_effective_lines {
            continue;
        }
        let test_functions = module.syntax_facts.test_function_count;
        if test_functions < limits.min_test_functions {
            continue;
        }
        let rule = &rules[RUST_PROJ_R005];
        findings.push(AspRustFinding::from_rule(
            rule,
            format!(
                "{} carries {effective_lines} effective lines across {test_functions} test functions.",
                display_project_path(project_root, &module.report.path)
            ),
            file_location(&module.report.path),
            None,
            "split this test leaf into a folder-first suite",
        ));
    }
}

fn collect_support_bloat_findings(
    project_root: &Path,
    suite_name: &str,
    modules: &[ParsedRustModule],
    reasoning_tree: &RustReasoningTreeFacts,
    rules: &BTreeMap<&'static str, AspRustRule>,
    findings: &mut Vec<AspRustFinding>,
) {
    for module in modules {
        let Some(module_facts) = reasoning_tree.module(&module.report.path) else {
            continue;
        };
        if !module.report.is_valid
            || !module_facts.source_path.is_test_source
            || !is_under_suite(project_root, &module.report.path, suite_name)
            || !is_test_support_path(&module.report.path)
        {
            continue;
        }
        let effective_lines = module.source_metrics.effective_code_lines;
        if effective_lines < MAX_TEST_SUPPORT_EFFECTIVE_LINES {
            continue;
        }
        let rule = &rules[RUST_PROJ_R024];
        findings.push(AspRustFinding::from_rule(
            rule,
            format!(
                "{} carries {effective_lines} effective lines in a test support module.",
                display_project_path(project_root, &module.report.path)
            ),
            file_location(&module.report.path),
            None,
            "split this test support module into focused support owners",
        ));
    }
}

fn is_under_suite(project_root: &Path, path: &Path, suite_name: &str) -> bool {
    let Ok(relative) = path.strip_prefix(project_root) else {
        return false;
    };
    let components = relative
        .iter()
        .map(|component| component.to_string_lossy())
        .collect::<Vec<_>>();
    matches!(
        components.as_slice(),
        [tests, suite, ..] if tests.as_ref() == "tests" && suite.as_ref() == suite_name
    )
}

fn is_test_support_path(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            "support" | "supports" | "fixture" | "fixtures" | "helper" | "helpers"
        )
    }) || path.file_stem().is_some_and(|stem| {
        matches!(
            stem.to_string_lossy().as_ref(),
            "support" | "supports" | "fixture" | "fixtures" | "helper" | "helpers"
        )
    })
}
