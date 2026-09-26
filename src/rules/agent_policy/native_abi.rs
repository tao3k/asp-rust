//! Agent policy rules for native ABI contract ownership.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, path_line_location, source_line};
use crate::rules::display_path;
use crate::{AspRustFinding, AspRustRule};

use super::RUST_AGENT_POLICY_NATIVE_ABI_CONTRACT_V1;

pub(super) fn native_abi_contract_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_NATIVE_ABI_CONTRACT_V1];
    module
        .syntax_facts
        .native_abi_surfaces
        .iter()
        .filter(|surface| !surface.is_test_context && surface.has_native_abi_marker)
        .filter_map(|surface| {
            let mut missing = Vec::new();
            if !surface.has_abi_version_const {
                missing.push("ABI_VERSION");
            }
            if !surface.has_abi_id_const {
                missing.push("ABI_ID");
            }
            if !surface.has_header_path_const {
                missing.push("HEADER_PATH");
            }
            if !surface.has_header_source_const {
                missing.push("HEADER_SOURCE");
            }
            if missing.is_empty() {
                return None;
            }
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes native ABI {} `{}` without {} in the same owner.",
                    display_path(&module.report.path),
                    surface.item_kind,
                    surface.item_name,
                    missing.join(", "),
                ),
                path_line_location(&module.report.path, surface.line),
                source_line(&module.source, surface.line),
                "co-locate native ABI layout types with ABI version, ABI id, header path, and header source constants so agents update Rust, C, and projection contracts together",
            ))
        })
        .collect()
}
