#[test]
fn scenario_benchmark_hard_gate_accepts_current_required_suite() {
    assert_rule_fixture_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"));
}

#[test]
fn scenario_benchmark_numeric_gate_reports_speed_and_memory_failures() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "scenario_benchmark_numeric_gate_reports_speed_and_memory_failures"
snapshot = "scenario_benchmark_numeric_gate_reports_speed_and_memory_failures"
target_total = "25ms"
max_total = "100ms"
observed_total = "140ms"
regression_budget = "20ms"
memory_budget_bytes = 1024
observed_memory_bytes = 2048
target_rationale = "The fixture should stay bounded."

[observed_timings]
parse = "120ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Fail);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Performance
            && violation.field == "benchmark.observed_total"
    }));
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Memory
            && violation.field == "benchmark.observed_memory_bytes"
    }));
}

#[test]
fn scenario_benchmark_accepts_zero_count_with_real_timing_provenance() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "scenario_benchmark_accepts_zero_count_with_real_timing_provenance"
snapshot = "scenario_benchmark_accepts_zero_count_with_real_timing_provenance"
target_total = "25ms"
max_total = "100ms"
observed_total = "31us"
regression_budget = "20ms"
memory_budget_bytes = 1024
observed_memory_bytes = 0
target_rationale = "Counts and durations have separate typed evidence."

[measurement]
clock = "std::time::Instant"
statistic = "p95"
warmup_iterations = 2
measure_iterations = 9
total_p50 = "27us"
total_p95 = "31us"
total_max = "38us"

[observed_timings]
build_dag_derivation = "29us"

[metrics.provider_process_count]
unit = "count"
kind = "exact"
target = 0
observed = 0
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");
    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
}

#[test]
fn scenario_benchmark_rejects_zero_clock_sample_instead_of_inventing_time() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "scenario_benchmark_rejects_zero_clock_sample_instead_of_inventing_time"
snapshot = "scenario_benchmark_rejects_zero_clock_sample_instead_of_inventing_time"
target_total = "25ms"
max_total = "100ms"
observed_total = "0us"
regression_budget = "20ms"
memory_budget_bytes = 1024
observed_memory_bytes = 0
target_rationale = "A zero clock sample is an invalid measurement, not a duration budget pass."

[measurement]
clock = "std::time::Instant"
statistic = "p95"
warmup_iterations = 2
measure_iterations = 9
total_p50 = "0us"
total_p95 = "0us"
total_max = "0us"

[observed_timings]
build_dag_derivation = "0us"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");
    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.field == "benchmark.measurement.total_p95"
            && violation.message.contains("resolution")
    }));
    assert!(receipt.violations.iter().any(|violation| {
        violation.field == "benchmark.observed_timings.build_dag_derivation"
            && violation.message.contains("remeasure")
    }));
}

#[test]
fn scenario_benchmark_comparison_allows_expected_to_be_slower_than_input() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "scenario_benchmark_comparison_allows_expected_to_be_slower_than_input"
snapshot = "scenario_benchmark_comparison_allows_expected_to_be_slower_than_input"
target_total = "25ms"
max_total = "100ms"
observed_total = "30ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "The expected fixture may trade a small runtime cost for clearer safety boundaries."

[input_expected_comparison]
input_total = "9ms"
expected_total = "12ms"
input_memory_bytes = 1048576
expected_memory_bytes = 2097152
interpretation = "The input fixture is faster here, but the expected fixture documents the safe owner boundary."
expected_not_faster_annotation = "Expected is intentionally slower here because the scenario is validating annotation behavior."

[observed_timings]
fixture = "30ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .expect("comparison is part of the contract");
    assert!(comparison.expected_total > comparison.input_total);
    assert!(comparison.expected_not_faster_annotation.is_some());
}

#[test]
fn scenario_benchmark_comparison_requires_annotation_when_expected_is_not_faster() {
    let temp = TempDir::new().expect("temp dir");
    write_scenario(temp.path());
    write_benchmark(
        temp.path(),
        r#"
harness = "libtest"
test = "scenario_benchmark_comparison_requires_annotation_when_expected_is_not_faster"
snapshot = "scenario_benchmark_comparison_requires_annotation_when_expected_is_not_faster"
target_total = "25ms"
max_total = "100ms"
observed_total = "30ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "The expected fixture must annotate when it does not beat input."

[input_expected_comparison]
input_total = "9ms"
expected_total = "12ms"
input_memory_bytes = 1048576
expected_memory_bytes = 2097152
interpretation = "This is incomplete because the slower expected fixture has no annotation."

[observed_timings]
fixture = "30ms"
"#,
    );

    let receipt = validate_rust_scenario_benchmark(temp.path()).expect("validate scenario");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field
                == "benchmark.input_expected_comparison.expected_not_faster_annotation"
    }));
}

#[test]
fn scenario_benchmark_suite_reports_missing_required_benchmark() {
    let temp = TempDir::new().expect("temp dir");
    let scenario_root = temp
        .path()
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("missing_benchmark");
    fs::create_dir_all(&scenario_root).expect("create scenario root");
    write_scenario(&scenario_root);

    let receipt = validate_required_rust_scenario_benchmarks(temp.path()).expect("validate suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Invalid);
    assert_eq!(receipt.requirements.len(), 1);
    assert!(receipt.receipts.is_empty());
    assert!(receipt.violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Contract
            && violation.field == "tests/unit/scenarios/missing_benchmark/benchmark.toml"
    }));

    let panic = std::panic::catch_unwind(|| {
        assert_rule_fixture_scenario_benchmarks(temp.path());
    })
    .expect_err("hard gate should panic for missing benchmark");
    let message = panic_message(panic);
    assert!(message.contains("scenario benchmark hard gate failed"));
    assert!(message.contains("preferred fix: add benchmark.toml"));
    assert!(message.contains("measure_asp_rust_scenario"));
    assert!(message.contains("Do not declare observed_total"));
    assert!(!message.contains("advisory mode ="));
    assert!(!message.contains("expires ="));
}

#[test]
fn scenario_benchmark_hard_gate_panics_with_repair_template() {
    let temp = TempDir::new().expect("temp dir");
    let scenario_root = temp
        .path()
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("missing_benchmark");
    fs::create_dir_all(&scenario_root).expect("create scenario root");
    write_scenario(&scenario_root);

    let panic = std::panic::catch_unwind(|| {
        assert_rule_fixture_scenario_benchmarks(temp.path());
    })
    .expect_err("hard gate should panic for missing benchmark");
    let message = panic_message(panic);

    assert!(message.contains("scenario benchmark hard gate failed"));
    assert!(message.contains("tests/unit/scenarios/missing_benchmark/benchmark.toml"));
    assert!(message.contains("preferred fix: add benchmark.toml"));
    assert!(message.contains("asp_rust_scenario!"));
    assert!(message.contains("render_asp_rust_scenario_benchmark_toml"));
    assert!(message.contains("zero clock sample"));
    assert!(!message.contains("advisory mode ="));
    assert!(!message.contains("expires ="));
}

#[test]
fn scenario_benchmark_suite_reports_ast_patch_speed_failure() {
    let temp = TempDir::new().expect("temp dir");
    let scenario_root = temp
        .path()
        .join("tests")
        .join("fixtures")
        .join("ast_patch_scenarios")
        .join("slow_apply");
    fs::create_dir_all(&scenario_root).expect("create ast patch scenario root");
    fs::write(
        scenario_root.join("scenario.json"),
        r#"
{
  "mode": "apply",
  "expectedStatus": "applied",
  "expectedCapability": "provider-ast-apply",
  "expectedMutationAvailable": true,
  "expectedOperation": "replace_item"
}
"#,
    )
    .expect("write ast patch scenario");
    write_benchmark(
        &scenario_root,
        r#"
harness = "libtest"
test = "ast_patch_scenarios::slow_apply"
snapshot = "ast_patch_scenarios::slow_apply"
target_total = "25ms"
max_total = "100ms"
observed_total = "140ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "AST patch scenario should stay bounded."

[observed_timings]
manifest = "5ms"
apply = "120ms"
"#,
    );

    let receipt = validate_required_rust_scenario_benchmarks(temp.path()).expect("validate suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Fail);
    assert_eq!(receipt.requirements.len(), 1);
    assert_eq!(receipt.receipts.len(), 1);
    assert!(receipt.receipts[0].violations.iter().any(|violation| {
        violation.kind == RustScenarioBenchmarkViolationKind::Performance
            && violation.field == "benchmark.observed_total"
    }));
}
use super::{
    RustScenarioBenchmarkStatus, RustScenarioBenchmarkViolationKind, TempDir,
    assert_rule_fixture_scenario_benchmarks, fs, panic_message,
    validate_required_rust_scenario_benchmarks, validate_rust_scenario_benchmark, write_benchmark,
    write_scenario,
};
