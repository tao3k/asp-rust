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
use super::{
    AspRustScenarioObservation, AspRustWorkspacePolicy, Instant, RustScenarioBenchmarkContract,
    RustScenarioBenchmarkPhase, RustScenarioBenchmarkStatus, asp_rust_scenario,
    asp_rust_workspace_build_dag_with_metrics, assert_asp_rust_workspace_policy_with, black_box,
    fixture_root, measure_asp_rust_scenario, render_asp_rust_scenario_benchmark_toml,
    validate_rust_scenario_benchmark,
};
