//! Special Rust entrypoint and facade policies.

use std::collections::BTreeMap;

use crate::parser::{
    ParsedRustModule, RustReasoningModuleFacts, RustTopLevelItemSyntax, path_line_location,
    source_line,
};
use crate::rules::display_path;
use crate::{AspRustFinding, AspRustRule};

use super::{RUST_MOD_R001, RUST_MOD_R004, RUST_MOD_R005, RUST_MOD_R006};

pub(super) fn crate_facade_findings(
    module_facts: &RustReasoningModuleFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    if !module_facts.source_path.is_crate_facade {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R004];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| !is_crate_facade_item(item))
        .map(|item| {
            AspRustFinding::from_rule(
                rule,
                format!(
                    "{} contains implementation item `{}` inside lib.rs.",
                    display_path(&module.report.path),
                    item.kind
                ),
                path_line_location(&module.report.path, item.line),
                source_line(&module.source, item.line),
                "move facade implementation out of lib.rs into an owned module",
            )
        })
        .collect()
}

pub(super) fn binary_entrypoint_findings(
    module_facts: &RustReasoningModuleFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    if !module_facts.source_path.is_binary_entrypoint {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R005];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| !is_binary_entrypoint_item(item))
        .map(|item| {
            AspRustFinding::from_rule(
                rule,
                format!(
                    "{} contains implementation item `{}` inside a binary entrypoint.",
                    display_path(&module.report.path),
                    item.kind
                ),
                path_line_location(&module.report.path, item.line),
                source_line(&module.source, item.line),
                "move binary implementation out of the entrypoint into an owned module",
            )
        })
        .collect()
}

pub(super) fn build_script_entrypoint_findings(
    module_facts: &RustReasoningModuleFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    if !module_facts.source_path.is_build_script_entrypoint {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R006];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| !is_build_script_entrypoint_item(item))
        .map(|item| {
            AspRustFinding::from_rule(
                rule,
                format!(
                    "{} contains implementation item `{}` inside build.rs.",
                    display_path(&module.report.path),
                    item.kind
                ),
                path_line_location(&module.report.path, item.line),
                source_line(&module.source, item.line),
                "move build script implementation out of build.rs into a build dependency",
            )
        })
        .collect()
}

pub(super) fn interface_mod_findings(
    module_facts: &RustReasoningModuleFacts,
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    if !module_facts.source_path.is_interface_mod {
        return Vec::new();
    }
    let rule = &rules[RUST_MOD_R001];
    module
        .syntax_facts
        .top_level_items
        .iter()
        .filter(|item| !is_interface_item(item))
        .map(|item| {
            AspRustFinding::from_rule(
                rule,
                format!(
                    "{} contains implementation item `{}` inside mod.rs.",
                    display_path(&module.report.path),
                    item.kind
                ),
                path_line_location(&module.report.path, item.line),
                source_line(&module.source, item.line),
                "move implementation out of mod.rs into an owned leaf module",
            )
        })
        .collect()
}

fn is_interface_item(item: &RustTopLevelItemSyntax) -> bool {
    item.module.as_ref().is_some_and(|module| !module.is_inline) || item.is_use
}

fn is_crate_facade_item(item: &RustTopLevelItemSyntax) -> bool {
    item.module.as_ref().is_some_and(|module| !module.is_inline)
        || item.is_use
        || item.is_extern_crate
        || item.has_proc_macro_export_attr
        || is_crate_boundary_macro(item)
}

fn is_binary_entrypoint_item(item: &RustTopLevelItemSyntax) -> bool {
    item.function_name.as_deref() == Some("main") || item.is_use
}

fn is_build_script_entrypoint_item(item: &RustTopLevelItemSyntax) -> bool {
    item.function_name.as_deref() == Some("main") || item.is_use
}

fn is_source_gate_macro_name(name: &str) -> bool {
    matches!(
        name,
        "asp_rust_gate"
            | "asp_rust_cargo_test_gate"
            | "asp_rust_source_gate"
            | "crate_test_policy_source_harness"
            | "crate_testing_source_gate"
            | "crate_testing_gate"
            | "crate_test_policy_harness"
    )
}

fn is_crate_boundary_macro(item: &RustTopLevelItemSyntax) -> bool {
    let Some(name) = item.macro_name.as_deref() else {
        return false;
    };
    is_source_gate_macro_name(name)
        || (name == "compile_error" && item.has_cfg_attr)
        || (name != "macro_rules" && item.macro_body_is_facade_boundary)
}
