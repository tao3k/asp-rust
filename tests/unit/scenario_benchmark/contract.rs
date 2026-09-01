use std::{fs, path::Path};

use asp_rust::{
    RustScenarioBenchmarkStatus, RustScenarioBenchmarkViolationKind,
    validate_rust_scenario_benchmark,
};
use tempfile::TempDir;

#[test]
fn scenario_benchmark_contract_accepts_libtest_insta_snapshot_entry() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "workspace_file_rejection_error_snapshot_and_perf"
snapshot = "workspace_file_rejection_error_snapshot_and_perf"
target_total = "750us"
max_total = "1.2ms"
observed_total = "900\u00b5s"
regression_budget = "250us"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Workspace argument validation is an in-process Rust API path."

[observed_timings]
workspace_metadata = "750\u00b5s"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert_eq!(receipt.benchmark.harness, "libtest");
    assert_eq!(
        receipt.benchmark.bench_entry(),
        "harness=libtest test=workspace_file_rejection_error_snapshot_and_perf snapshot=workspace_file_rejection_error_snapshot_and_perf"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
}

#[test]
fn scenario_benchmark_contract_accepts_sub_millisecond_durations_and_generic_timing_keys() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "duration_parser_accepts_sub_millisecond_units"
target_total = "750us"
max_total = "1.2ms"
observed_total = "900µs"
regression_budget = "250μs"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Duration parsing is an in-process TOML contract path."

[observed_timings]
total = "999999ns"
min = "500ns"
max = "1.1ms"
mean = "750us"
median = "750µs"
p50 = "750μs"
p90 = "900us"
p95 = "950µs"
p99 = "990μs"
workspace_metadata = "750us"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
}

#[test]
fn scenario_benchmark_contract_requires_agent_visible_metadata() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario_with_policy_ids(temp.path(), "[]");
    write_benchmark(
        temp.path(),
        r#"
harness = ""
test = ""
target_total = "120ms"
max_total = "100ms"
observed_total = "90ms"
regression_budget = "0ms"
memory_budget_bytes = 0
observed_memory_bytes = 0
target_rationale = ""
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "scenario.policy_ids"
    }));
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.observed_timings"
    }));
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.target_total"
    }));
}

#[test]
fn scenario_benchmark_contract_rejects_self_referential_gate_command() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "orgize_rule_fixtures_have_scenario_benchmarks"
target_total = "25ms"
max_total = "100ms"
observed_total = "25ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Small rule fixture should stay bounded."

[observed_timings]
fixture = "25ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.entry"
            && violation
                .message
                .contains("focused Rust test or bench case")
    }));
}

#[test]
fn scenario_benchmark_contract_rejects_second_scale_hard_gate() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "criterion"
bench = "asp_search_deps"
case = "tokio"
target_total = "250ms"
max_total = "5s"
observed_total = "240ms"
regression_budget = "100ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Dependency seed should stay inside the millisecond gate."

[observed_timings]
asp_search_deps = "240ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "benchmark.max_total"
            && violation.message.contains("hard gate")
    }));
}

fn write_scenario(root: &Path) {
    write_scenario_with_policy_ids(root, r#"["RUST-AGENT-CFG-001"]"#);
}

fn write_scenario_with_policy_ids(root: &Path, policy_ids: &str) {
    fs::write(
        root.join("scenario.toml"),
        format!(
            r#"
id = "contract-test"
title = "Contract test"
policy_ids = {policy_ids}
agent_goal = "Keep the scenario understandable."
reference_repositories = ["rust-lang/rust"]
reference_patterns = ["Test fixtures still name the source of the contract pattern"]
inputs = "inputs"
expected = "expected"
"#
        ),
    )
    .expect("write scenario");
}

fn write_benchmark(root: &Path, text: &str) {
    fs::write(root.join("benchmark.toml"), text).expect("write benchmark");
}
