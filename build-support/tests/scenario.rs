use std::hint::black_box;
use std::time::Instant;

use asp_rust_build_support::{
    AspRustScenarioMetricKind, AspRustScenarioObservation, asp_rust_scenario,
    measure_asp_rust_scenario, render_asp_rust_scenario_benchmark_toml,
};

#[test]
fn scenario_macro_generates_real_measurement_and_typed_zero_count() {
    let scenario = asp_rust_scenario! {
        name: "workspace-build-dag-package-once",
        package: "asp-rust",
        description: "Cargo manifests and package policies execute once per workspace atom",
        fixture_root: "tests/unit/scenarios/build_system/workspace_dependency_graph_package_once_v1",
        tags: ["build-system", "workspace-dag"],
        commands: [
            { label: "focused", argv: ["cargo", "test", "scenario_benchmark_workspace_dependency_graph_package_once_v1_snapshot"] }
        ],
        benchmark: {
            harness: "libtest",
            test: "scenario_benchmark_workspace_dependency_graph_package_once_v1_snapshot",
            snapshot: "scenario_benchmark_workspace_dependency_graph_package_once_v1",
            target_total: "5ms",
            max_total: "25ms",
            regression_budget: "5ms",
            memory_budget_bytes: 4_194_304,
            target_rationale: "Workspace parsing is linear in unique Cargo packages.",
            warmup_iterations: 2,
            measure_iterations: 9,
            metrics: [
                { name: "bytes_hashed", unit: "bytes", kind: Exact, target: 65_536 },
                { name: "provider_process_count", unit: "count", kind: Exact, target: 0 }
            ]
        }
    };

    let input = vec![0x5a_u8; 65_536];
    let measurement = measure_asp_rust_scenario(&scenario, || {
        let phase_started_at = Instant::now();
        black_box(blake3::hash(black_box(&input)));
        AspRustScenarioObservation::default()
            .with_timing("content_digest", phase_started_at.elapsed())
            .with_metric("bytes_hashed", input.len() as u64)
            .with_metric("provider_process_count", 0)
    })
    .expect("measure Scenario with the monotonic clock");

    assert_eq!(
        scenario.benchmark.as_ref().expect("benchmark").metrics[0].kind,
        AspRustScenarioMetricKind::Exact
    );
    assert_eq!(measurement.observed_total, measurement.total_p95);
    assert!(measurement.total_p50 <= measurement.total_p95);
    assert!(measurement.total_p95 <= measurement.total_max);
    let rendered = render_asp_rust_scenario_benchmark_toml(&scenario, &measurement)
        .expect("render measured benchmark");
    assert!(rendered.contains("clock = \"std::time::Instant\""));
    assert!(rendered.contains("statistic = \"p95\""));
    assert!(rendered.contains("measure_iterations = 9"));
    assert!(rendered.contains("[metrics.provider_process_count]"));
    assert!(rendered.contains("observed = 0"));
    assert!(!rendered.contains("observed_total = \"0"));
}
