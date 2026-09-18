//! Agent policy rules derived from public API shape facts.

use std::collections::BTreeMap;

use crate::parser::{ParsedRustModule, path_line_location, source_line};
use crate::{AspRustFinding, AspRustRule};

use crate::rules::display_path;

use super::doc_boundary::documented_agent_boundary;
use super::{
    RUST_AGENT_POLICY_PUBLIC_DYNAMIC_JSON_API_BOUNDARY_V1,
    RUST_AGENT_POLICY_PUBLIC_TUPLE_API_SURFACE_V1,
};

pub(super) fn api_shape_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut findings = public_tuple_api_surface_findings(module, rules);
    findings.extend(public_dynamic_json_api_surface_findings(module, rules));
    findings
}

fn public_tuple_api_surface_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_PUBLIC_TUPLE_API_SURFACE_V1];
    module
        .syntax_facts
        .public_tuple_api_surfaces
        .iter()
        .filter(|surface| !surface.is_test_context)
        .filter_map(|surface| {
            if documented_agent_boundary(
                &module.source,
                surface.function_line,
                &["tuple api boundary", "raw dto boundary", "anonymous payload boundary"],
            ) {
                return None;
            }
            let element_list = surface.element_contract_types.join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public function `{}` {} as anonymous tuple with primitive elements: {element_list}.",
                    display_path(&module.report.path),
                    surface.function_name,
                    surface.surface_name
                ),
                path_line_location(&module.report.path, surface.line),
                source_line(&module.source, surface.line),
                "replace the tuple with a named struct, enum, or newtype that carries field intent",
            ))
        })
        .collect()
}

fn public_dynamic_json_api_surface_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_PUBLIC_DYNAMIC_JSON_API_BOUNDARY_V1];
    module
        .syntax_facts
        .public_dynamic_json_api_surfaces
        .iter()
        .filter(|surface| !surface.is_test_context)
        .filter_map(|surface| {
            if documented_agent_boundary(
                &module.source,
                surface.function_line,
                &[
                    "dynamic json api boundary",
                    "raw json api boundary",
                    "serde_json value boundary",
                    "untyped payload boundary",
                ],
            ) {
                return None;
            }
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public function `{}` {} as dynamic JSON `{}` through `{}`.",
                    display_path(&module.report.path),
                    surface.function_name,
                    surface.surface_name,
                    surface.json_type_name,
                    surface.type_text
                ),
                path_line_location(&module.report.path, surface.line),
                source_line(&module.source, surface.line),
                "replace `serde_json::Value` with a named request, response, enum, or documented boundary type",
            ))
        })
        .collect()
}
