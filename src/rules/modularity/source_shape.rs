//! Source file shape and owner-boundary policies.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::parser::{
    ParsedRustModule, RustReasoningModuleFacts, RustUseGlobScopeKind, RustUseStatementSyntax,
    file_location, path_line_location, source_line,
};
use crate::rules::display_path;
use crate::{AspRustFinding, AspRustRule};

use super::{
    MAX_SOURCE_EFFECTIVE_LINES, MAX_SOURCE_LINES, MIN_SOURCE_IMPLEMENTATION_ITEMS,
    MIN_SOURCE_PUBLIC_ITEMS, RUST_MOD_R002, RUST_MOD_R003, RUST_MOD_R010, RUST_MOD_R011,
};

pub(super) fn source_file_bloat_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let source_lines = module.source_metrics.source_lines;
    let effective_lines = module.source_metrics.effective_code_lines;
    let has_absolute_line_pressure = source_lines >= MAX_SOURCE_LINES;
    let has_effective_line_pressure = effective_lines >= MAX_SOURCE_EFFECTIVE_LINES;
    let public_items = module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.is_public)
        .count();
    let implementation_items = module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| item.is_implementation_item)
        .count();
    let has_item_pressure = public_items >= MIN_SOURCE_PUBLIC_ITEMS
        || implementation_items >= MIN_SOURCE_IMPLEMENTATION_ITEMS;
    let has_effective_item_pressure = has_effective_line_pressure && has_item_pressure;
    if !has_absolute_line_pressure && !has_effective_item_pressure {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R002];
    vec![AspRustFinding::from_rule(
        rule,
        format!(
            "{} carries {source_lines} source lines, {effective_lines} effective code lines, {public_items} public items, {implementation_items} top-level implementation items, and {}.",
            display_path(&module.report.path),
            if has_item_pressure {
                "large item pressure"
            } else {
                "absolute source line pressure"
            }
        ),
        file_location(&module.report.path),
        None,
        "split this source file by responsibility",
    )]
}

pub(super) fn sibling_file_dir_owner_collision_findings(
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rust_sources = rust_source_paths(modules);
    let rust_source_ancestor_dirs = rust_source_ancestor_dirs(&rust_sources);
    let mut reported = BTreeSet::new();
    let mut findings = Vec::new();

    for file_path in &rust_sources {
        if !is_named_rust_file(file_path) || is_under_tests_dir(file_path) {
            continue;
        }
        let Some(stem) = file_path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(parent) = file_path.parent() else {
            continue;
        };
        let owner_dir = parent.join(stem);
        let has_child_sources = rust_source_ancestor_dirs.contains(&owner_dir);
        if !has_child_sources || !reported.insert(file_path.clone()) {
            continue;
        }

        let rule = &rules[RUST_MOD_R011];
        findings.push(AspRustFinding::from_rule(
            rule,
            format!(
                "{} and {}/ share the same owner name at one filesystem level.",
                display_path(file_path),
                display_path(&owner_dir)
            ),
            file_location(file_path),
            None,
            "move the owner interface to mod.rs under the directory",
        ));
    }

    findings
}

fn rust_source_paths(modules: &[ParsedRustModule]) -> BTreeSet<PathBuf> {
    modules
        .iter()
        .filter(|module| {
            module
                .report
                .path
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .map(|module| module.report.path.clone())
        .collect()
}

fn rust_source_ancestor_dirs(rust_sources: &BTreeSet<PathBuf>) -> BTreeSet<PathBuf> {
    let mut dirs = BTreeSet::new();
    for source in rust_sources {
        let Some(parent) = source.parent() else {
            continue;
        };
        dirs.extend(parent.ancestors().map(Path::to_path_buf));
    }
    dirs
}

fn is_named_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
        && !path
            .file_stem()
            .and_then(|value| value.to_str())
            .is_some_and(|stem| matches!(stem, "lib" | "main" | "mod"))
}

fn is_under_tests_dir(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "tests")
}

pub(super) fn deep_relative_import_findings(
    module_facts: &RustReasoningModuleFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_MOD_R003];
    module_facts
        .import_summary
        .deep_relative_import_facts
        .iter()
        .map(|deep_relative_import| {
            let crate_path = deep_relative_import.rendered_crate_path();
            let suggestion = crate_path
                .as_deref()
                .unwrap_or("a crate::... owner/facade import");
            let summary_suffix = crate_path
                .as_ref()
                .map(|crate_path| format!("; parser suggests `{crate_path}`."))
                .unwrap_or_else(|| {
                    "; parser could not derive a safe crate-relative replacement.".to_string()
                });
            AspRustFinding::from_rule(
                rule,
                format!(
                    "{} uses deep relative import `{}`{}",
                    display_path(&module.report.path),
                    deep_relative_import.rendered_path(),
                    summary_suffix
                ),
                path_line_location(&module.report.path, deep_relative_import.line),
                source_line(&module.source, deep_relative_import.line),
                format!(
                    "replace with {} or expose it through an owner facade",
                    suggestion
                ),
            )
        })
        .collect()
}

pub(super) fn glob_import_findings(
    module_facts: &RustReasoningModuleFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_MOD_R010];
    module
        .syntax_facts
        .use_statements
        .iter()
        .filter_map(|use_syntax| {
            if use_syntax.contains_glob_import {
                Some(AspRustFinding::from_rule(
                    rule,
                    glob_import_summary(module_facts, module, use_syntax),
                    path_line_location(&module.report.path, use_syntax.line),
                    source_line(&module.source, use_syntax.line),
                    glob_import_label(use_syntax),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn glob_import_summary(
    module_facts: &RustReasoningModuleFacts,
    module: &ParsedRustModule,
    use_syntax: &RustUseStatementSyntax,
) -> String {
    format!(
        "{} uses {}{}.",
        display_path(&module.report.path),
        glob_import_descriptor(use_syntax),
        glob_import_context(module_facts, use_syntax),
    )
}

fn glob_import_descriptor(use_syntax: &RustUseStatementSyntax) -> String {
    let imports = &use_syntax.glob_imports;
    let Some(first_import) = imports.first() else {
        return "a Rust glob import".to_string();
    };
    if imports.len() > 1 {
        return format!(
            "{} Rust glob imports ({})",
            imports.len(),
            imports
                .iter()
                .map(|glob_import| glob_import.rendered_path())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let rendered_path = first_import.rendered_path();
    if first_import.is_direct_parent_scope_glob {
        return format!("parent-scope glob import `{rendered_path}`");
    }
    if first_import.is_parent_relative_glob {
        return format!("parent-relative glob import `{rendered_path}`");
    }
    if first_import.is_prelude_glob {
        return format!("prelude glob import `{rendered_path}`");
    }
    if first_import.scope_kind == RustUseGlobScopeKind::CrateOwner {
        return format!("crate-owner glob import `{rendered_path}`");
    }
    format!("Rust glob import `{rendered_path}`")
}

fn glob_import_context(
    module_facts: &RustReasoningModuleFacts,
    use_syntax: &RustUseStatementSyntax,
) -> &'static str {
    if module_facts.source_path.is_test_source || use_syntax.context.is_inside_cfg_test_module {
        " in test context"
    } else if use_syntax.context.is_inside_inline_module {
        " inside inline module"
    } else {
        ""
    }
}

fn glob_import_label(use_syntax: &RustUseStatementSyntax) -> &'static str {
    if use_syntax
        .glob_imports
        .iter()
        .any(|glob_import| glob_import.is_direct_parent_scope_glob)
    {
        "replace parent-scope glob with explicit imports"
    } else {
        "replace glob import with explicit owner imports"
    }
}
