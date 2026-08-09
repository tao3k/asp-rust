use std::collections::BTreeSet;
use std::path::Path;

use cargo_toml::{Inheritable, LintGroups, Manifest, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CargoCfgFacts {
    pub(crate) cfg: String,
    pub(crate) declared_in: String,
    pub(crate) expression: String,
}

pub(crate) fn parse_cargo_cfg_facts(project_root: &Path) -> Vec<CargoCfgFacts> {
    let Some(manifest) = super::cargo_manifest::read_manifest_for_cfg(project_root) else {
        return Vec::new();
    };
    let mut cfgs = manifest_cfg_facts(&manifest);
    cfgs.sort();
    cfgs.dedup();
    cfgs
}

fn manifest_cfg_facts(manifest: &Manifest) -> Vec<CargoCfgFacts> {
    let mut cfgs = feature_cfg_facts(&manifest.features);
    cfgs.extend(lint_cfg_facts(
        "workspace.lints.rust.unexpected_cfgs",
        manifest
            .workspace
            .as_ref()
            .map(|workspace| &workspace.lints),
    ));
    if let Ok(lints) = manifest.lints.get() {
        cfgs.extend(lint_cfg_facts("lints.rust.unexpected_cfgs", Some(lints)));
    } else if matches!(manifest.lints, Inheritable::Inherited) {
        cfgs.push(CargoCfgFacts {
            cfg: "workspace".to_string(),
            declared_in: "lints".to_string(),
            expression: "workspace=true".to_string(),
        });
    }
    cfgs.extend(
        manifest
            .target
            .keys()
            .flat_map(|target| target_cfg_facts(target)),
    );
    cfgs
}

fn feature_cfg_facts(features: &cargo_toml::FeatureSet) -> Vec<CargoCfgFacts> {
    features
        .keys()
        .map(|name| CargoCfgFacts {
            cfg: format!("feature:{name}"),
            declared_in: "features".to_string(),
            expression: format!("cfg(feature=\"{name}\")"),
        })
        .collect()
}

fn lint_cfg_facts(declared_in: &str, lints: Option<&LintGroups>) -> Vec<CargoCfgFacts> {
    let Some(lint) = lints
        .and_then(|groups| groups.get("rust"))
        .and_then(|rust| rust.get("unexpected_cfgs"))
    else {
        return Vec::new();
    };
    lint.config
        .get("check-cfg")
        .into_iter()
        .flat_map(cargo_cfg_strings)
        .flat_map(|expression| cfg_facts_for_expression(declared_in, &expression))
        .collect()
}

fn cargo_cfg_strings(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => vec![value.clone()],
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn target_cfg_facts(target_name: &str) -> Vec<CargoCfgFacts> {
    cfg_facts_for_expression("target.dependencies", target_name)
}

fn cfg_facts_for_expression(declared_in: &str, expression: &str) -> Vec<CargoCfgFacts> {
    let expression = compact_cfg_expression(expression);
    cfg_labels_from_expression(&expression)
        .into_iter()
        .map(|cfg| CargoCfgFacts {
            cfg,
            declared_in: declared_in.to_string(),
            expression: expression.clone(),
        })
        .collect()
}

fn cfg_labels_from_expression(expression: &str) -> BTreeSet<String> {
    let mut labels = BTreeSet::new();
    let mut token = String::new();
    let mut in_quote = false;
    let has_feature_cfg = expression_has_token(expression, "feature");
    for character in expression.chars() {
        if character == '"' {
            if in_quote && has_feature_cfg && !token.is_empty() {
                labels.insert(format!("feature:{token}"));
            }
            token.clear();
            in_quote = !in_quote;
        } else if in_quote {
            token.push(character);
        } else if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            token.push(character);
        } else {
            push_cfg_label(&mut labels, &mut token);
        }
    }
    push_cfg_label(&mut labels, &mut token);
    labels
}

fn expression_has_token(expression: &str, needle: &str) -> bool {
    expression
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && character != '_' && character != '-'
        })
        .any(|token| token == needle)
}

fn push_cfg_label(labels: &mut BTreeSet<String>, token: &mut String) {
    if token.is_empty() {
        return;
    }
    if !matches!(token.as_str(), "cfg" | "all" | "any" | "not" | "values") {
        labels.insert(std::mem::take(token));
    } else {
        token.clear();
    }
}

fn compact_cfg_expression(expression: &str) -> String {
    expression
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
