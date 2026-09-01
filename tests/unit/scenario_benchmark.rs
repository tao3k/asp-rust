use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

use asp_rust::{
    AspRustWorkspacePolicy, RustScenarioBenchmarkContract, RustScenarioBenchmarkPhase,
    RustScenarioBenchmarkStatus, RustScenarioBenchmarkViolationKind,
    asp_rust_workspace_build_dag_with_metrics, assert_asp_rust_workspace_policy_with,
    assert_rule_fixture_scenario_benchmarks, validate_required_rust_scenario_benchmarks,
    validate_rust_scenario_benchmark,
};
use asp_rust_build_support::{
    AspRustScenarioObservation, asp_rust_scenario, measure_asp_rust_scenario,
    render_asp_rust_scenario_benchmark_toml,
};
use tempfile::TempDir;

#[path = "scenario_benchmark/snapshots_core.rs"]
mod snapshots_core;

#[path = "scenario_benchmark/snapshots_async.rs"]
mod snapshots_async;

#[path = "scenario_benchmark/policy_aliases.rs"]
mod policy_aliases;

#[path = "scenario_benchmark/gates.rs"]
mod gates;

#[path = "scenario_benchmark/support.rs"]
mod support;

use support::{fixture_root, panic_message, write_benchmark, write_scenario};
#[path = "scenario_benchmark/contract.rs"]
mod contract;

#[path = "scenario_benchmark/public_dynamic_json_api_boundary.rs"]
mod public_dynamic_json_api_boundary;
