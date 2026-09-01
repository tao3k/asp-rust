//! Source-backed test mount policy.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, path_line_location, source_line};
use crate::rules::{display_path, is_under_any_dir};
use crate::{AspRustFinding, AspRustRule, AspRustScope};

use super::{RUST_PROJ_R003, RUST_PROJ_R004};

pub(super) fn source_test_mount_findings(
    scope: &AspRustScope,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut findings = Vec::new();
    for module in modules {
        if !is_under_any_dir(&module.report.path, &scope.source_paths) {
            continue;
        }
        collect_source_test_mount_findings(&scope.project_root, module, rules, &mut findings);
    }
    findings
}

fn collect_source_test_mount_findings(
    project_root: &std::path::Path,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
    findings: &mut Vec<AspRustFinding>,
) {
    for item_mod in &module.syntax_facts.cfg_test_modules {
        if item_mod.is_inline {
            let rule = &rules[RUST_PROJ_R003];
            findings.push(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} contains inline #[cfg(test)] module `{}`.",
                    display_path(&module.report.path),
                    item_mod.ident
                ),
                path_line_location(&module.report.path, item_mod.line),
                source_line(&module.source, item_mod.line),
                "move this test module to tests/unit and mount it with #[path]",
            ));
            continue;
        }
        let Some(path_value) = item_mod.path_attr.as_deref() else {
            let rule = &rules[RUST_PROJ_R003];
            findings.push(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} declares cfg(test) module `{}` without an external #[path].",
                    display_path(&module.report.path),
                    item_mod.ident
                ),
                path_line_location(&module.report.path, item_mod.line),
                source_line(&module.source, item_mod.line),
                "add a #[path = \"...\"] mount into tests/unit",
            ));
            continue;
        };
        let Some(resolved) = item_mod.resolved_path_attr.as_ref() else {
            continue;
        };
        let project_relative = resolved.strip_prefix(project_root).unwrap_or(resolved);
        if !resolved.exists() || !project_relative.starts_with("tests/unit") {
            let rule = &rules[RUST_PROJ_R004];
            findings.push(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} mounts `{path_value}`, but the resolved path should exist under tests/unit.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, item_mod.line),
                source_line(&module.source, item_mod.line),
                "point this external test mount at an existing tests/unit file",
            ));
        }
    }
}
