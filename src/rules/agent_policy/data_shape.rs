//! Agent policy rules derived from public data-shape facts.

use std::collections::BTreeMap;

use crate::parser::{
    ParsedRustModule, RustPublicEnumTupleVariantFieldSyntax, RustPublicEnumVariantFieldSyntax,
    RustPublicStructFieldSyntax, RustPublicTypeAliasSyntax, path_line_location, source_line,
};
use crate::{AspRustFinding, AspRustRule};

use crate::rules::display_path;

use super::doc_boundary::documented_agent_boundary;
use super::{
    RUST_AGENT_POLICY_API_PRIMITIVE_TYPE_ALIAS_V1, RUST_AGENT_POLICY_DATA_DERIVABLE_BOUNDS_V1,
    RUST_AGENT_POLICY_DATA_ENUM_PRIMITIVE_PAYLOAD_V1, RUST_AGENT_POLICY_DATA_ENUM_TUPLE_PAYLOAD_V1,
    RUST_AGENT_POLICY_DATA_PRIMITIVE_FIELD_V1, RUST_AGENT_POLICY_DATA_STRINGLY_STATE_FIELD_V1,
};

const MIN_SEMANTIC_PRIMITIVE_FIELDS: usize = 3;
const MIN_ENUM_VARIANT_SEMANTIC_PRIMITIVE_FIELDS: usize = 2;
const MIN_ENUM_TUPLE_VARIANT_SEMANTIC_PRIMITIVE_FIELDS: usize = 2;

pub(super) fn data_shape_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut findings = Vec::new();
    findings.extend(public_data_struct_primitive_field_findings(module, rules));
    findings.extend(public_enum_variant_primitive_payload_findings(
        module, rules,
    ));
    findings.extend(public_type_generic_bound_findings(module, rules));
    findings.extend(public_enum_tuple_variant_payload_findings(module, rules));
    findings.extend(public_type_alias_primitive_findings(module, rules));
    findings.extend(public_stringly_state_field_findings(module, rules));
    findings
}

fn public_data_struct_primitive_field_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut fields_by_struct = BTreeMap::<(usize, String), Vec<(usize, String)>>::new();
    for field in &module.syntax_facts.public_struct_fields {
        if field.is_test_context {
            continue;
        }
        let Some(contract_type) = public_data_field_contract_type(field) else {
            continue;
        };
        fields_by_struct
            .entry((field.struct_line, field.struct_name.clone()))
            .or_default()
            .push((field.line, format!("{}: {contract_type}", field.field_name)));
    }

    let rule = &rules[RUST_AGENT_POLICY_DATA_PRIMITIVE_FIELD_V1];
    fields_by_struct
        .into_iter()
        .filter_map(|((struct_line, struct_name), mut fields)| {
            if fields.len() < MIN_SEMANTIC_PRIMITIVE_FIELDS {
                return None;
            }
            if documented_agent_boundary(
                &module.source,
                struct_line,
                &[
                    "raw dto boundary",
                    "primitive field boundary",
                    "semantic field boundary",
                ],
            ) {
                return None;
            }
            fields.sort_by_key(|(line, _)| *line);
            let field_list = fields
                .into_iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>()
                .join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public data struct `{struct_name}` with primitive semantic fields: {field_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, struct_line),
                source_line(&module.source, struct_line),
                "wrap these fields in named domain types or make the raw DTO boundary explicit",
            ))
        })
        .collect()
}

fn public_enum_variant_primitive_payload_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut fields_by_variant = BTreeMap::<(usize, String, String), Vec<(usize, String)>>::new();
    for field in &module.syntax_facts.public_enum_variant_fields {
        if field.is_test_context {
            continue;
        }
        let Some(contract_type) = public_enum_payload_field_contract_type(field) else {
            continue;
        };
        fields_by_variant
            .entry((
                field.variant_line,
                field.enum_name.clone(),
                field.variant_name.clone(),
            ))
            .or_default()
            .push((field.line, format!("{}: {contract_type}", field.field_name)));
    }

    let rule = &rules[RUST_AGENT_POLICY_DATA_ENUM_PRIMITIVE_PAYLOAD_V1];
    fields_by_variant
        .into_iter()
        .filter_map(|((variant_line, enum_name, variant_name), mut fields)| {
            if fields.len() < MIN_ENUM_VARIANT_SEMANTIC_PRIMITIVE_FIELDS {
                return None;
            }
            if documented_agent_boundary(
                &module.source,
                variant_line,
                &[
                    "raw dto boundary",
                    "primitive payload boundary",
                    "semantic payload boundary",
                ],
            ) {
                return None;
            }
            fields.sort_by_key(|(line, _)| *line);
            let field_list = fields
                .into_iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>()
                .join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public enum `{enum_name}` variant `{variant_name}` with primitive semantic payload fields: {field_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, variant_line),
                source_line(&module.source, variant_line),
                "wrap this variant payload in named domain types or move it into a named payload struct",
            ))
        })
        .collect()
}

fn public_type_generic_bound_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut bounds_by_type = BTreeMap::<(usize, &'static str, String), Vec<(usize, String)>>::new();
    for bound in &module.syntax_facts.public_type_generic_bounds {
        if bound.is_test_context {
            continue;
        }
        bounds_by_type
            .entry((bound.type_line, bound.type_kind, bound.type_name.clone()))
            .or_default()
            .push((
                bound.line,
                format!("{}: {}", bound.param_name, bound.bound_name),
            ));
    }

    let rule = &rules[RUST_AGENT_POLICY_DATA_DERIVABLE_BOUNDS_V1];
    bounds_by_type
        .into_iter()
        .filter_map(|((type_line, type_kind, type_name), mut bounds)| {
            if documented_agent_boundary(
                &module.source,
                type_line,
                &["generic bound boundary", "derived bound boundary"],
            ) {
                return None;
            }
            bounds.sort_by_key(|(line, _)| *line);
            bounds.dedup();
            let bound_list = bounds
                .into_iter()
                .map(|(_, bound)| bound)
                .collect::<Vec<_>>()
                .join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public {type_kind} `{type_name}` with duplicated data-type bounds: {bound_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, type_line),
                source_line(&module.source, type_line),
                "move these bounds to derived impls, inherent impls, or methods that actually require them",
            ))
        })
        .collect()
}

fn public_enum_tuple_variant_payload_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let mut fields_by_variant = BTreeMap::<(usize, String, String), Vec<(usize, String)>>::new();
    for field in &module.syntax_facts.public_enum_tuple_variant_fields {
        if field.is_test_context {
            continue;
        }
        let Some(contract_type) = public_enum_tuple_payload_field_contract_type(field) else {
            continue;
        };
        fields_by_variant
            .entry((
                field.variant_line,
                field.enum_name.clone(),
                field.variant_name.clone(),
            ))
            .or_default()
            .push((
                field.line,
                format!("#{}: {contract_type}", field.field_index + 1),
            ));
    }

    let rule = &rules[RUST_AGENT_POLICY_DATA_ENUM_TUPLE_PAYLOAD_V1];
    fields_by_variant
        .into_iter()
        .filter_map(|((variant_line, enum_name, variant_name), mut fields)| {
            if fields.len() < MIN_ENUM_TUPLE_VARIANT_SEMANTIC_PRIMITIVE_FIELDS {
                return None;
            }
            if documented_agent_boundary(
                &module.source,
                variant_line,
                &[
                    "tuple payload boundary",
                    "raw dto boundary",
                    "anonymous payload boundary",
                ],
            ) {
                return None;
            }
            fields.sort_by_key(|(line, _)| *line);
            let field_list = fields
                .into_iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>()
                .join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public enum `{enum_name}` tuple variant `{variant_name}` with anonymous primitive payload fields: {field_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, variant_line),
                source_line(&module.source, variant_line),
                "replace the tuple variant payload with named fields, a named payload struct, or domain newtypes",
            ))
        })
        .collect()
}

fn public_type_alias_primitive_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_API_PRIMITIVE_TYPE_ALIAS_V1];
    module
        .syntax_facts
        .public_type_aliases
        .iter()
        .filter(|alias| !alias.is_test_context)
        .filter_map(|alias| {
            if documented_agent_boundary(
                &module.source,
                alias.line,
                &["primitive alias boundary", "newtype compatibility boundary"],
            ) {
                return None;
            }
            let contract_type = public_type_alias_contract_type(alias)?;
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public semantic type alias `{}` = `{}` over primitive carrier {contract_type}.",
                    display_path(&module.report.path),
                    alias.alias_name,
                    alias.target_type_text
                ),
                path_line_location(&module.report.path, alias.line),
                source_line(&module.source, alias.line),
                "replace this alias with a newtype or named struct boundary",
            ))
        })
        .collect()
}

fn public_stringly_state_field_findings(
    module: &ParsedRustModule,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let rule = &rules[RUST_AGENT_POLICY_DATA_STRINGLY_STATE_FIELD_V1];
    let mut findings = Vec::new();
    findings.extend(public_struct_stringly_state_field_findings(module, rule));
    findings.extend(public_enum_variant_stringly_state_field_findings(
        module, rule,
    ));
    findings
}

fn public_struct_stringly_state_field_findings(
    module: &ParsedRustModule,
    rule: &AspRustRule,
) -> Vec<AspRustFinding> {
    let mut fields_by_struct = BTreeMap::<(usize, String), Vec<(usize, String)>>::new();
    for field in &module.syntax_facts.public_struct_fields {
        if field.is_test_context || !is_stringly_state_field(&field.field_name) {
            continue;
        }
        let Some(contract_type) = string_contract_type(field.primitive_contract_type.as_deref())
        else {
            continue;
        };
        fields_by_struct
            .entry((field.struct_line, field.struct_name.clone()))
            .or_default()
            .push((field.line, format!("{}: {contract_type}", field.field_name)));
    }
    fields_by_struct
        .into_iter()
        .filter_map(|((struct_line, struct_name), mut fields)| {
            if documented_agent_boundary(
                &module.source,
                struct_line,
                &["stringly state boundary", "typed catalog boundary"],
            ) {
                return None;
            }
            fields.sort_by_key(|(line, _)| *line);
            let field_list = fields
                .into_iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>()
                .join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public data struct `{struct_name}` with stringly state fields: {field_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, struct_line),
                source_line(&module.source, struct_line),
                "replace this stringly state surface with an enum, newtype, or typed catalog boundary",
            ))
        })
        .collect()
}

fn public_enum_variant_stringly_state_field_findings(
    module: &ParsedRustModule,
    rule: &AspRustRule,
) -> Vec<AspRustFinding> {
    let mut fields_by_variant = BTreeMap::<(usize, String, String), Vec<(usize, String)>>::new();
    for field in &module.syntax_facts.public_enum_variant_fields {
        if field.is_test_context || !is_stringly_state_field(&field.field_name) {
            continue;
        }
        let Some(contract_type) = string_contract_type(field.primitive_contract_type.as_deref())
        else {
            continue;
        };
        fields_by_variant
            .entry((
                field.variant_line,
                field.enum_name.clone(),
                field.variant_name.clone(),
            ))
            .or_default()
            .push((field.line, format!("{}: {contract_type}", field.field_name)));
    }
    fields_by_variant
        .into_iter()
        .filter_map(|((variant_line, enum_name, variant_name), mut fields)| {
            if documented_agent_boundary(
                &module.source,
                variant_line,
                &["stringly state boundary", "typed catalog boundary"],
            ) {
                return None;
            }
            fields.sort_by_key(|(line, _)| *line);
            let field_list = fields
                .into_iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>()
                .join(", ");
            Some(AspRustFinding::from_rule(
                rule,
                format!(
                    "{} exposes public enum `{enum_name}` variant `{variant_name}` with stringly state fields: {field_list}.",
                    display_path(&module.report.path)
                ),
                path_line_location(&module.report.path, variant_line),
                source_line(&module.source, variant_line),
                "replace this stringly state payload with an enum, newtype, or typed catalog boundary",
            ))
        })
        .collect()
}

fn public_data_field_contract_type(field: &RustPublicStructFieldSyntax) -> Option<&str> {
    if is_semantic_public_data_field(&field.field_name) {
        return field
            .primitive_contract_type
            .as_deref()
            .or(field.flag_contract_type.as_deref());
    }
    is_public_flag_field(&field.field_name)
        .then_some(field.flag_contract_type.as_deref())
        .flatten()
}

fn public_enum_payload_field_contract_type(
    field: &RustPublicEnumVariantFieldSyntax,
) -> Option<&str> {
    if is_semantic_public_data_field(&field.field_name) {
        return field
            .primitive_contract_type
            .as_deref()
            .or(field.flag_contract_type.as_deref());
    }
    is_public_flag_field(&field.field_name)
        .then_some(field.flag_contract_type.as_deref())
        .flatten()
}

fn public_enum_tuple_payload_field_contract_type(
    field: &RustPublicEnumTupleVariantFieldSyntax,
) -> Option<&str> {
    field
        .primitive_contract_type
        .as_deref()
        .or(field.flag_contract_type.as_deref())
}

fn public_type_alias_contract_type(alias: &RustPublicTypeAliasSyntax) -> Option<&str> {
    if is_semantic_public_type_alias(&alias.alias_name)
        || is_public_flag_type_alias(&alias.alias_name)
    {
        return alias
            .primitive_contract_type
            .as_deref()
            .or(alias.flag_contract_type.as_deref());
    }
    None
}

fn is_semantic_public_data_field(name: &str) -> bool {
    is_semantic_identifier(name)
        || matches!(name, "path" | "url" | "uri" | "key" | "token")
        || name.ends_with("_path")
        || name.ends_with("_url")
        || name.ends_with("_uri")
        || name.ends_with("_key")
        || name.ends_with("_token")
        || name.ends_with("_ms")
        || name.ends_with("_secs")
        || name.ends_with("_seconds")
        || name.ends_with("_bytes")
}

fn is_stringly_state_field(name: &str) -> bool {
    let name = name.strip_prefix("r#").unwrap_or(name);
    matches!(
        name,
        "kind" | "type" | "status" | "state" | "mode" | "phase" | "tag" | "category"
    ) || name.ends_with("_kind")
        || name.ends_with("_type")
        || name.ends_with("_status")
        || name.ends_with("_state")
        || name.ends_with("_mode")
        || name.ends_with("_phase")
        || name.ends_with("_tag")
        || name.ends_with("_category")
}

fn string_contract_type(contract_type: Option<&str>) -> Option<&str> {
    let contract_type = contract_type?;
    matches!(contract_type, "String" | "Option<String>").then_some(contract_type)
}

fn is_semantic_identifier(name: &str) -> bool {
    name == "id" || name.ends_with("_id")
}

fn is_semantic_public_type_alias(name: &str) -> bool {
    name == "Id"
        || name.ends_with("Id")
        || name.ends_with("ID")
        || name.ends_with("Path")
        || name.ends_with("Url")
        || name.ends_with("URL")
        || name.ends_with("Uri")
        || name.ends_with("URI")
        || name.ends_with("Key")
        || name.ends_with("Token")
        || name.ends_with("Ms")
        || name.ends_with("Secs")
        || name.ends_with("Seconds")
        || name.ends_with("Bytes")
}

fn is_public_flag_field(name: &str) -> bool {
    matches!(name, "enabled" | "disabled")
        || name.starts_with("is_")
        || name.starts_with("has_")
        || name.starts_with("can_")
        || name.starts_with("allow_")
        || name.starts_with("include_")
        || name.starts_with("should_")
}

fn is_public_flag_type_alias(name: &str) -> bool {
    name.ends_with("Flag")
        || name.ends_with("Enabled")
        || name.ends_with("Disabled")
        || name.ends_with("Allowed")
        || name.ends_with("Included")
}
