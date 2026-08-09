#![deny(dead_code)]

#[cfg(feature = "cli")]
#[path = "unit/cli/mod.rs"]
mod cli;

#[path = "unit/public_api/mod.rs"]
mod public_api;

#[path = "unit/policy_contract.rs"]
mod policy_contract;

#[path = "unit/harness_rules.rs"]
mod harness_rules;

#[path = "unit/policy_config.rs"]
mod policy_config;

#[path = "unit/path_policy.rs"]
mod path_policy;

#[path = "unit/rule_catalog.rs"]
mod rule_catalog;

#[path = "unit/invariant_catalog.rs"]
mod invariant_catalog;

#[path = "unit/render_snapshot.rs"]
mod render_snapshot;

#[path = "unit/mod_policy_snapshot.rs"]
mod mod_policy_snapshot;

#[path = "unit/agent_policy_snapshot.rs"]
mod agent_policy_snapshot;

#[path = "unit/software_criterion_snapshot.rs"]
mod software_criterion_snapshot;

#[path = "unit/scenario_benchmark.rs"]
mod scenario_benchmark;

#[path = "unit/agent_reasoning_snapshot.rs"]
mod agent_reasoning_snapshot;

#[path = "unit/verification/mod.rs"]
mod verification;

#[path = "unit/runner_config/mod.rs"]
mod runner_config;

#[path = "unit/sample_project.rs"]
mod sample_project;

#[path = "unit/nested_item_facts.rs"]
mod nested_item_facts;
#[path = "unit/no_asp_crate_dependencies.rs"]
mod no_asp_crate_dependencies;
#[path = "unit/provider_workspace_search_identity.rs"]
mod provider_workspace_search_identity;
#[path = "unit/search.rs"]
mod search;
#[path = "unit/structural_selector.rs"]
mod structural_selector;

#[path = "unit/query_workspace_contract.rs"]
mod query_workspace_contract;

#[path = "unit/rs_harness_attribute.rs"]
mod rs_harness_attribute;
