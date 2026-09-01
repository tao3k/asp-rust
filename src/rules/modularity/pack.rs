//! Rust modularity rule catalog and evaluator.

use crate::parser::{ParsedRustModule, rust_reasoning_tree_facts};
use crate::{AspRustFinding, AspRustRule, AspRustScope};

use super::catalog::rules_by_id;
use super::entrypoints::{
    binary_entrypoint_findings, build_script_entrypoint_findings, crate_facade_findings,
    interface_mod_findings,
};
use super::reasoning_tree::{
    inline_source_module_findings, module_source_shadow_findings, orphan_source_module_findings,
};
use super::source_shape::{
    deep_relative_import_findings, glob_import_findings, sibling_file_dir_owner_collision_findings,
    source_file_bloat_findings,
};

pub(crate) const PACK_ID: &str = "rust.modularity";
pub(crate) const RUST_MOD_R001: &str = "RUST-MOD-R001";
pub(crate) const RUST_MOD_R002: &str = "RUST-MOD-R002";
pub(crate) const RUST_MOD_R003: &str = "RUST-MOD-R003";
pub(crate) const RUST_MOD_R004: &str = "RUST-MOD-R004";
pub(crate) const RUST_MOD_R005: &str = "RUST-MOD-R005";
pub(crate) const RUST_MOD_R006: &str = "RUST-MOD-R006";
pub(crate) const RUST_MOD_R007: &str = "RUST-MOD-R007";
pub(crate) const RUST_MOD_R008: &str = "RUST-MOD-R008";
pub(crate) const RUST_MOD_R009: &str = "RUST-MOD-R009";
pub(crate) const RUST_MOD_R010: &str = "RUST-MOD-R010";
pub(crate) const RUST_MOD_R011: &str = "RUST-MOD-R011";

pub(crate) const MAX_SOURCE_EFFECTIVE_LINES: usize = 650;
pub(crate) const MAX_SOURCE_LINES: usize = 1000;
pub(crate) const MIN_SOURCE_PUBLIC_ITEMS: usize = 12;
pub(crate) const MIN_SOURCE_IMPLEMENTATION_ITEMS: usize = 45;

/// Return compact metadata for Rust modularity rules.
#[must_use]
pub fn rust_modularity_rules() -> Vec<AspRustRule> {
    rules_by_id().into_values().collect()
}

pub(crate) fn evaluate(
    scope: Option<&AspRustScope>,
    modules: &[ParsedRustModule],
) -> Vec<AspRustFinding> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let rules = rules_by_id();
    let mut findings = Vec::new();
    let reasoning_tree = rust_reasoning_tree_facts(scope, modules);
    findings.extend(module_source_shadow_findings(&reasoning_tree, &rules));
    findings.extend(orphan_source_module_findings(&reasoning_tree, &rules));
    findings.extend(sibling_file_dir_owner_collision_findings(modules, &rules));
    for module in modules {
        let Some(module_facts) = reasoning_tree.module(&module.report.path) else {
            continue;
        };
        if !module_facts.is_source_module
            && !module_facts.source_path.is_test_source
            && !module_facts.source_path.is_package_entrypoint
        {
            continue;
        }
        findings.extend(source_file_bloat_findings(module, &rules));
        if !module.report.is_valid {
            continue;
        }
        let is_source_or_test_module = module_facts.is_source_module || module_facts.is_test_module;
        if module_facts.is_source_module {
            findings.extend(crate_facade_findings(module_facts, module, &rules));
            findings.extend(binary_entrypoint_findings(module_facts, module, &rules));
            findings.extend(inline_source_module_findings(module_facts, module, &rules));
        }
        if is_source_or_test_module {
            findings.extend(interface_mod_findings(module_facts, module, &rules));
        }
        if is_source_or_test_module {
            findings.extend(deep_relative_import_findings(module_facts, module, &rules));
            findings.extend(glob_import_findings(module_facts, module, &rules));
        }
        findings.extend(build_script_entrypoint_findings(
            module_facts,
            module,
            &rules,
        ));
    }
    findings
}
