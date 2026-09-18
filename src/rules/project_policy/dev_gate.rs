//! Cargo-test harness gate policy.

use std::collections::BTreeMap;
use std::path::Path;

use crate::parser::{CargoManifestFacts, ParsedRustModule, file_location};
use crate::{AspRustFinding, AspRustRule};

use super::support::display_project_path;
use super::{RUST_PROJ_R006, RUST_PROJ_R009, RUST_PROJ_R012};

pub(super) fn cargo_test_gate_findings(
    project_root: &Path,
    cargo_manifest: &CargoManifestFacts,
    modules: &[ParsedRustModule],
    cargo_test_targets: &[ParsedRustModule],
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let build_script_path = project_root.join("build.rs");
    let build_script = root_build_script_module(project_root, modules);
    let has_dev_gate_call = build_script.is_some_and(module_contains_dev_gate_call);
    let has_direct_cargo_test_gate = modules
        .iter()
        .chain(cargo_test_targets)
        .any(module_contains_cargo_test_gate);
    let has_build_support_test_gate = cargo_test_targets
        .iter()
        .any(module_contains_workspace_test_gate_wrapper);
    let has_cargo_test_gate = has_direct_cargo_test_gate || has_build_support_test_gate;
    let harness_enabled = cargo_manifest.references_harness_dev_dependency
        || cargo_manifest.references_harness_build_dependency
        || cargo_manifest.references_harness_non_optional_normal_dependency
        || has_dev_gate_call
        || has_cargo_test_gate;

    if !harness_enabled {
        return Vec::new();
    }

    let mut findings = Vec::new();
    if cargo_manifest.references_harness_build_dependency
        || (cargo_manifest.references_harness_non_optional_normal_dependency
            && !cargo_manifest.is_build_support_package)
    {
        findings.push(AspRustFinding::from_rule(
            &rules[RUST_PROJ_R006],
            format!(
                "{} activates ASP Rust outside the test dependency graph.",
                display_project_path(project_root, &project_root.join("Cargo.toml"))
            ),
            file_location(project_root.join("Cargo.toml")),
            None,
            "move asp-rust to [dev-dependencies] so ordinary builds and downstream package consumers do not compile the policy provider",
        ));
    }

    if has_dev_gate_call {
        findings.push(AspRustFinding::from_rule(
            &rules[RUST_PROJ_R009],
            format!(
                "{} activates ASP Rust while Cargo is building the package.",
                display_project_path(project_root, &build_script_path)
            ),
            file_location(&build_script_path),
            None,
            "remove the ASP Rust call from build.rs and mount asp_rust_cargo_test_gate! from a cargo-test target",
        ));
    }

    if !cargo_manifest.is_build_support_package
        && ((!cargo_manifest.references_harness_dev_dependency
            && !cargo_manifest.is_asp_rust_package
            && !has_build_support_test_gate)
            || !has_cargo_test_gate)
    {
        findings.push(AspRustFinding::from_rule(
            &rules[RUST_PROJ_R012],
            format!(
                "{} does not have a complete cargo-test ASP Rust policy gate.",
                display_project_path(project_root, &project_root.join("Cargo.toml"))
            ),
            file_location(project_root.join("Cargo.toml")),
            None,
            "declare asp-rust or the workspace Build Support crate under [dev-dependencies], then mount its policy macro in a cargo-test target",
        ));
    }

    findings
}

fn module_contains_cargo_test_gate(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .macro_invocations
        .iter()
        .any(|invocation| {
            matches!(
                invocation.terminal_name.as_str(),
                "asp_rust_gate" | "asp_rust_cargo_test_gate"
            )
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

pub(super) fn module_contains_dev_gate_call(module: &ParsedRustModule) -> bool {
    module_contains_direct_dev_gate_call(module)
        || module_contains_workspace_dev_gate_wrapper_call(module)
}

fn module_contains_direct_dev_gate_call(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .contains_function_call_named(LEGACY_BUILD_POLICY_FUNCTIONS)
}

fn module_contains_workspace_dev_gate_wrapper_call(module: &ParsedRustModule) -> bool {
    module
        .syntax_facts
        .function_calls
        .iter()
        .any(|invocation| is_workspace_dev_gate_wrapper_function(&invocation.terminal_name))
}

fn module_contains_workspace_test_gate_wrapper(module: &ParsedRustModule) -> bool {
    module_contains_workspace_dev_gate_wrapper_call(module)
        || module
            .syntax_facts
            .macro_invocations
            .iter()
            .any(|invocation| is_workspace_dev_gate_wrapper_macro(&invocation.terminal_name))
}

fn is_workspace_dev_gate_wrapper_function(function_name: &str) -> bool {
    !matches!(function_name, "asp_rust_gate" | "asp_rust_cargo_test_gate")
        && (function_name.starts_with("assert_") || function_name.ends_with("_gate"))
        && (function_name.contains("asp") || function_name.contains("harness"))
        && (function_name.contains("gate") || function_name.contains("policy"))
        && function_name.contains("_from_env")
}

fn is_workspace_dev_gate_wrapper_macro(macro_name: &str) -> bool {
    !matches!(macro_name, "asp_rust_gate" | "asp_rust_cargo_test_gate")
        && macro_name.ends_with("_gate")
        && (macro_name.contains("asp") || macro_name.contains("harness"))
        && (macro_name.contains("gate") || macro_name.contains("policy"))
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

const LEGACY_BUILD_POLICY_FUNCTIONS: &[&str] = &[
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
