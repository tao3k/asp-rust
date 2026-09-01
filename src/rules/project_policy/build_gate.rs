//! Build-script harness gate policy.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{CargoManifestFacts, ParsedRustModule, file_location};
use crate::{AspRustFinding, AspRustRule};

use super::RUST_PROJ_R012;
use super::support::display_project_path;

pub(super) fn build_gate_findings(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
    modules: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let build_script_path = project_root.join("build.rs");
    let build_script = root_build_script_module(project_root, modules);
    let build_script_exists = build_script.is_some() || build_script_path.exists();
    let has_build_gate_call = build_script.is_some_and(module_contains_build_gate_call);
    let has_direct_build_gate_call =
        build_script.is_some_and(module_contains_direct_build_gate_call);
    let harness_enabled = cargo_manifest.references_harness || has_build_gate_call;

    if !harness_enabled || project_has_complete_build_gate(project_root, cargo_manifest, modules) {
        return Vec::new();
    }

    let rule = &rules[RUST_PROJ_R012];
    if !cargo_manifest.references_harness_build_dependency && !build_script_exists {
        return vec![AspRustFinding::from_rule(
            rule,
            format!(
                "{} enables ASP Rust without build-time contract evidence.",
                display_project_path(project_root, &project_root.join("Cargo.toml"))
            ),
            file_location(project_root.join("Cargo.toml")),
            None,
            "add asp-rust-build-support under [build-dependencies] and call emit_provider_contract_digest() from a thin root build.rs",
        )];
    }

    if cargo_manifest.references_harness_build_dependency && !build_script_exists {
        return vec![AspRustFinding::from_rule(
            rule,
            format!(
                "{} declares ASP Rust build support but does not provide a root build.rs contract entrypoint.",
                display_project_path(project_root, &project_root.join("Cargo.toml"))
            ),
            file_location(project_root.join("Cargo.toml")),
            None,
            "add a thin root build.rs that calls asp_rust_build_support::emit_provider_contract_digest()",
        )];
    }

    if build_script_exists && !has_build_gate_call {
        return vec![AspRustFinding::from_rule(
            rule,
            format!(
                "{} exists in an ASP Rust-enabled project but does not emit build-time contract evidence.",
                display_project_path(project_root, &build_script_path)
            ),
            file_location(&build_script_path),
            None,
            "call asp_rust_build_support::emit_provider_contract_digest() from root build.rs",
        )];
    }

    if has_direct_build_gate_call && !cargo_manifest.references_harness_build_dependency {
        return vec![AspRustFinding::from_rule(
            rule,
            format!(
                "{} calls the ASP Rust build-time contract entrypoint but Cargo.toml does not declare ASP Rust Build Support as a build-dependency.",
                display_project_path(project_root, &build_script_path)
            ),
            file_location(project_root.join("Cargo.toml")),
            None,
            "add asp-rust-build-support under [build-dependencies] so Cargo can compile the contract entrypoint",
        )];
    }

    Vec::new()
}

pub(super) fn project_has_complete_build_gate(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
    modules: &[ParsedRustModule],
) -> bool {
    root_build_script_module(project_root, modules).is_some_and(|module| {
        module_contains_workspace_build_gate_wrapper_call(module)
            || (cargo_manifest.references_harness_build_dependency
                && module_contains_direct_build_gate_call(module))
    })
}

pub(super) fn root_build_script_module<'a>(
    project_root: &Path,
    modules: &'a [ParsedRustModule],
) -> Option<&'a ParsedRustModule> {
    let build_script_path = project_root.join("build.rs");
    modules
        .iter()
        .find(|module| same_path(&module.report.path, &build_script_path))
}

pub(super) fn module_contains_build_gate_call(module: &ParsedRustModule) -> bool {
    module_contains_direct_build_gate_call(module)
        || module_contains_workspace_build_gate_wrapper_call(module)
}

fn module_contains_direct_build_gate_call(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .contains_function_call_named(BUILD_GATE_FUNCTIONS)
}

fn module_contains_workspace_build_gate_wrapper_call(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .function_calls
        .iter()
        .any(|invocation| is_workspace_build_gate_wrapper_function(&invocation.terminal_name))
}

fn is_workspace_build_gate_wrapper_function(function_name: &str) -> bool {
    function_name.starts_with("assert_")
        && function_name.contains("harness")
        && (function_name.contains("gate") || function_name.contains("policy"))
        && function_name.contains("_from_env")
}

pub(super) fn module_default_build_gate_call_lines(
    module: &ParsedRustModule,
) -> impl Iterator<Item = usize> + '_ {
    module
        .syntax_facts
        .function_calls
        .iter()
        .filter(|invocation| {
            DEFAULT_BUILD_GATE_FUNCTIONS.contains(&invocation.terminal_name.as_str())
        })
        .map(|invocation| invocation.line)
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

const BUILD_GATE_FUNCTIONS: &[&str] = &[
    "emit_provider_contract_digest",
    "assert_asp_rust_build_clean",
    "assert_asp_rust_build_clean_with_config",
    "assert_asp_rust_build_clean_from_env",
    "assert_asp_rust_build_clean_from_env_with_config",
    "assert_asp_rust_cargo_check_clean",
    "assert_asp_rust_cargo_check_clean_with_config",
    "assert_asp_rust_cargo_check_clean_from_env",
    "assert_asp_rust_cargo_check_clean_from_env_with_config",
    "assert_asp_rust_downstream_policy",
    "assert_asp_rust_downstream_policy_from_env",
    "assert_asp_rust_downstream_policy_with_authority",
];

const DEFAULT_BUILD_GATE_FUNCTIONS: &[&str] = &[
    "assert_asp_rust_build_clean",
    "assert_asp_rust_build_clean_from_env",
    "assert_asp_rust_cargo_check_clean",
    "assert_asp_rust_cargo_check_clean_from_env",
];
