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

#[test]
fn scenario_benchmark_control_flow_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/control_flow_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate control-flow scenario benchmark");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
}

#[test]
fn scenario_benchmark_source_index_fallback_control_v1_snapshot() {
    let scenario_root = fixture_root("search_interface/source_index_fallback_control_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate source-index fallback-control scenario benchmark");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    assert_eq!(
        receipt.benchmark.fallback_reason.as_deref(),
        Some("explicit-miss-or-rejected-only")
    );
}

#[test]
fn scenario_benchmark_workspace_dependency_graph_package_once_v1_snapshot() {
    let scenario_root = fixture_root("build_system/workspace_dependency_graph_package_once_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate workspace dependency-graph package-once scenario benchmark");
    let workspace_root = scenario_root.join(&receipt.scenario.inputs);
    let derivation = asp_rust_workspace_build_dag_with_metrics(
        &workspace_root,
        &asp_rust::default_asp_rust_config(),
    )
    .expect("derive the Cargo workspace dependency graph");
    let package_names = derivation
        .build_dag
        .packages
        .iter()
        .map(|package| package.package_name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert_eq!(package_names, ["shared", "left", "right", "app"]);
    assert_eq!(derivation.metrics.admitted_package_count, 4);
    assert_eq!(derivation.metrics.parsed_manifest_count, 5);
    assert_eq!(derivation.metrics.local_dependency_edge_count, 4);
    assert_eq!(derivation.metrics.discovered_package_root_count, 4);
    assert_eq!(
        package_names
            .iter()
            .filter(|package_name| **package_name == "shared")
            .count(),
        1,
        "the diamond dependency must appear exactly once in the Build DAG"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
}

#[test]
fn workspace_build_dag_benchmark_is_generated_from_real_samples() {
    let scenario_root = fixture_root("build_system/workspace_dependency_graph_package_once_v1");
    let workspace_root = scenario_root.join("inputs");
    let scenario = asp_rust_scenario! {
        name: "workspace-build-dag-package-once",
        package: "asp-rust",
        description: "Cargo manifests and package policies execute once per workspace atom",
        fixture_root: "tests/unit/scenarios/build_system/workspace_dependency_graph_package_once_v1",
        tags: ["build-system", "workspace-dag"],
        commands: [
            { label: "focused", argv: ["cargo", "test", "workspace_build_dag_benchmark_is_generated_from_real_samples"] }
        ],
        benchmark: {
            harness: "libtest",
            test: "workspace_build_dag_benchmark_is_generated_from_real_samples",
            snapshot: "scenario_benchmark_workspace_dependency_graph_package_once_v1",
            target_total: "5ms",
            max_total: "25ms",
            regression_budget: "5ms",
            memory_budget_bytes: 4_194_304,
            target_rationale: "Manifest-owned Build DAG derivation parses every unique workspace manifest once.",
            warmup_iterations: 2,
            measure_iterations: 9,
            metrics: [
                { name: "discovered_package_root_count", unit: "count", kind: Stable, },
                { name: "admitted_package_count", unit: "count", kind: Stable, },
                { name: "parsed_manifest_count", unit: "count", kind: Stable, },
                { name: "policy_execution_count", unit: "count", kind: Stable, },
                { name: "unique_local_dependency_edge_count", unit: "count", kind: Stable, }
            ]
        }
    };

    let measurement = measure_asp_rust_scenario(&scenario, || {
        let phase_started_at = Instant::now();
        let derivation = asp_rust_workspace_build_dag_with_metrics(
            &workspace_root,
            &asp_rust::default_asp_rust_config(),
        )
        .expect("derive measured Build DAG");
        black_box(&derivation.build_dag);
        let build_dag_elapsed = phase_started_at.elapsed();
        let policy_started_at = Instant::now();
        let mut policy_execution_count = 0_u64;
        let workspace_policy = AspRustWorkspacePolicy::new(
            "workspace-build-dag-package-once",
            asp_rust::default_asp_rust_config(),
        );
        let report = assert_asp_rust_workspace_policy_with(
            &workspace_root,
            &workspace_policy,
            |_package_name, config| {
                policy_execution_count += 1;
                config
            },
        );
        black_box(report);
        AspRustScenarioObservation::default()
            .with_timing("build_dag_derivation", build_dag_elapsed)
            .with_timing("policy_execution", policy_started_at.elapsed())
            .with_metric(
                "discovered_package_root_count",
                derivation.metrics.discovered_package_root_count as u64,
            )
            .with_metric(
                "parsed_manifest_count",
                derivation.metrics.parsed_manifest_count as u64,
            )
            .with_metric(
                "admitted_package_count",
                derivation.metrics.admitted_package_count as u64,
            )
            .with_metric("policy_execution_count", policy_execution_count)
            .with_metric(
                "unique_local_dependency_edge_count",
                derivation.metrics.local_dependency_edge_count as u64,
            )
    })
    .expect("measure Build DAG Scenario");
    let rendered = render_asp_rust_scenario_benchmark_toml(&scenario, &measurement)
        .expect("render measured Build DAG benchmark");
    let generated = toml::from_str::<RustScenarioBenchmarkContract>(&rendered)
        .expect("decode generated benchmark contract");

    assert_eq!(
        generated.observed_total,
        generated
            .measurement
            .as_ref()
            .expect("generated measurement provenance")
            .total_p95
    );
    assert!(generated.observed_total.as_duration().as_nanos() > 0);
    assert_eq!(
        generated.metrics["discovered_package_root_count"].observed,
        4
    );
    assert_eq!(generated.metrics["admitted_package_count"].observed, 4);
    assert_eq!(generated.metrics["parsed_manifest_count"].observed, 5);
    assert_eq!(
        generated.metrics["parsed_manifest_count"].observed,
        generated.metrics["admitted_package_count"].observed + 1,
        "one workspace manifest plus one parse per admitted package"
    );
    assert_eq!(
        generated.metrics["policy_execution_count"].observed,
        generated.metrics["admitted_package_count"].observed,
        "each admitted Build DAG node must execute policy exactly once"
    );
    eprintln!("generated workspace Build DAG benchmark:\n{rendered}");
}

#[test]
fn scenario_benchmark_data_structure_linear_membership_scan_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/data_structure_linear_membership_scan_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate data-structure linear membership scan scenario benchmark");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-DS-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("linear membership scan scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes <= comparison.input_memory_bytes,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_process_command_probe_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/process_command_probe_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate process-command probe scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("process-command probe scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_phase_is_typed() {
    let contract = toml::from_str::<RustScenarioBenchmarkContract>(
        r#"
harness = "libtest"
test = "unbounded_async_queue_without_backpressure_is_agent_advice"
snapshot = "scenario_benchmark_async_backpressure_boundary_v1"
phase = "cold"
target_total = "20ms"
max_total = "70ms"
observed_total = "9ms"
regression_budget = "15ms"
memory_budget_bytes = 1048576
observed_memory_bytes = 245760
target_rationale = "fixture"
"#,
    )
    .expect("deserialize typed benchmark phase");

    assert_eq!(contract.phase, Some(RustScenarioBenchmarkPhase::Cold));
}

#[test]
fn scenario_benchmark_async_blocking_boundary_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_blocking_boundary_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async blocking boundary scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-BLOCKING-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async blocking boundary scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes <= comparison.input_memory_bytes,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_async_sync_lock_boundary_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_sync_lock_boundary_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async sync lock boundary scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-SYNC-LOCK-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async sync lock boundary scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_async_backpressure_boundary_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_backpressure_boundary_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async backpressure boundary scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-BACKPRESSURE-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async backpressure boundary scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_async_select_cancellation_safety_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_select_cancellation_safety_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async select cancellation safety scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-CANCEL-SAFETY-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async select cancellation safety scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_async_timeout_cancellation_safety_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_timeout_cancellation_safety_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async timeout cancellation safety scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-CANCEL-SAFETY-002"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async timeout cancellation safety scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_async_task_lifecycle_boundary_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/async_task_lifecycle_boundary_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate async task lifecycle boundary scenario benchmark");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt
            .scenario
            .policy_ids
            .iter()
            .any(|policy_id| policy_id == "RUST-AGENT-ASYNC-TASK-LIFECYCLE-001"),
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("async task lifecycle boundary scenario should compare input and expected");
    assert!(
        comparison.expected_total < comparison.input_total,
        "{comparison:?}"
    );
    assert!(
        comparison.expected_memory_bytes < comparison.input_memory_bytes,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_rust_package_edition_2024_v1_snapshot() {
    let scenario_root = fixture_root("software_criteria/rust_package_edition_2024_v1");
    let receipt = validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate Rust package edition 2024 scenario benchmark");

    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert!(scenario_root.join(&receipt.scenario.inputs).is_dir());
    assert!(scenario_root.join(&receipt.scenario.expected).is_dir());
    assert!(
        receipt.benchmark.observed_total <= receipt.benchmark.max_total,
        "{receipt:?}"
    );
    assert!(
        receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes,
        "{receipt:?}"
    );
    let comparison = receipt
        .benchmark
        .input_expected_comparison
        .as_ref()
        .expect("edition scenario should compare input and expected");
    assert!(
        comparison.expected_total <= comparison.input_total,
        "{comparison:?}"
    );
}

#[test]
fn scenario_benchmark_suite_covers_all_required_current_scenarios() {
    let receipt = validate_required_rust_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"))
        .expect("validate required scenario benchmark suite");

    assert_eq!(receipt.status, RustScenarioBenchmarkStatus::Pass);
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
    assert_eq!(receipt.receipts.len(), receipt.requirements.len());
    let covered_rule_ids = receipt
        .policy_coverage
        .iter()
        .map(|coverage| coverage.rule_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let required_rule_ids = asp_rust::rust_agent_policy_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<std::collections::BTreeSet<_>>();
    let missing_rule_ids = required_rule_ids
        .difference(&covered_rule_ids)
        .copied()
        .collect::<Vec<_>>();
    assert!(
        missing_rule_ids.is_empty(),
        "missing policy scenario coverage for {missing_rule_ids:?}: {receipt:?}"
    );
    assert!(
        covered_rule_ids.contains("RUST-AGENT-PROJECT-MANIFEST-023"),
        "missing project manifest scenario coverage: {receipt:?}"
    );
    assert!(
        receipt
            .receipts
            .iter()
            .any(|scenario_receipt| scenario_receipt.scenario.id
                == "source-index-fallback-control-v1"),
        "missing search-interface fallback-control scenario coverage: {receipt:?}"
    );
    assert!(receipt.policy_coverage.iter().all(|coverage| {
        receipt
            .receipts
            .iter()
            .any(|scenario_receipt| scenario_receipt.root == coverage.root)
    }));
    assert!(receipt.receipts.iter().all(|receipt| {
        receipt.benchmark.observed_total <= receipt.benchmark.max_total
            && receipt.benchmark.observed_memory_bytes <= receipt.benchmark.memory_budget_bytes
    }));
}

#[test]
fn agent_policy_schema_ids_do_not_expose_legacy_aliases() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    for relative_root in ["src", "tests", "docs"] {
        collect_legacy_agent_policy_aliases(&crate_root.join(relative_root), &mut offenders);
    }
    offenders.sort();

    assert!(
        offenders.is_empty(),
        "legacy agent policy aliases must not be public: {offenders:?}"
    );
}

fn collect_legacy_agent_policy_aliases(path: &std::path::Path, offenders: &mut Vec<String>) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if metadata.is_dir() {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return;
        };
        if matches!(name, "target" | ".git") {
            return;
        }
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            collect_legacy_agent_policy_aliases(&entry.path(), offenders);
        }
        return;
    }

    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return;
    };
    if !matches!(extension, "rs" | "md" | "toml") {
        return;
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    if contains_legacy_agent_policy_alias(&content) {
        offenders.push(path.display().to_string());
    }
}

fn contains_legacy_agent_policy_alias(content: &str) -> bool {
    let hyphen_alias = ["AGENT", "-R"].concat();
    let underscore_alias = ["AGENT", "_R"].concat();
    content
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '-' || character == '_')
        })
        .any(|token| {
            [&hyphen_alias, &underscore_alias].iter().any(|prefix| {
                token.strip_prefix(prefix.as_str()).is_some_and(|suffix| {
                    suffix
                        .chars()
                        .take(3)
                        .all(|character| character.is_ascii_digit())
                })
            })
        })
}

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

fn fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join(name)
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

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    "<non-string panic>".to_string()
}
#[path = "scenario_benchmark/contract.rs"]
mod contract;

#[path = "scenario_benchmark/public_dynamic_json_api_boundary.rs"]
mod public_dynamic_json_api_boundary;
