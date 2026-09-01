//! Default rule-pack execution.

use crate::parser::ParsedRustModule;
use crate::{AspRustConfig, AspRustFinding, AspRustScope};

use super::{agent_policy, modularity, project_policy, syntax};

pub(crate) fn evaluate_default_rule_packs_with_config(
    scope: Option<&AspRustScope>,
    modules: &[ParsedRustModule],
    config: &AspRustConfig,
) -> Vec<AspRustFinding> {
    let mut findings = Vec::new();
    findings.extend(syntax::evaluate(modules));
    findings.extend(project_policy::evaluate(scope, modules, config));
    findings.extend(modularity::evaluate(scope, modules));
    findings.extend(agent_policy::evaluate(scope, modules));
    apply_policy_config(findings, config)
}

pub(crate) fn evaluate_workspace_rule_packs_with_config(
    workspace_root: &std::path::Path,
    package_scopes: &[AspRustScope],
    config: &AspRustConfig,
) -> Vec<AspRustFinding> {
    apply_policy_config(
        super::project_policy::evaluate_workspace(workspace_root, package_scopes, config),
        config,
    )
}

fn apply_policy_config(
    findings: Vec<AspRustFinding>,
    config: &AspRustConfig,
) -> Vec<AspRustFinding> {
    findings
        .into_iter()
        .filter_map(|mut finding| {
            if config.disabled_rules.contains(&finding.rule_id) {
                return None;
            }
            if let Some(severity) = config.rule_severity_overrides.get(&finding.rule_id) {
                finding.severity = *severity;
            }
            Some(finding)
        })
        .collect()
}
