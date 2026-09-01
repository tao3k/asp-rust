use super::contract::RustScenarioBenchmarkContract;

pub(super) fn benchmark_entry_targets_contract_gate(
    benchmark: &RustScenarioBenchmarkContract,
) -> bool {
    [
        benchmark.test.as_deref(),
        benchmark.bench.as_deref(),
        benchmark.case.as_deref(),
        benchmark.snapshot.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| value.split_ascii_whitespace().any(targets_contract_gate))
}

fn targets_contract_gate(token: &str) -> bool {
    token == "assert_rule_fixture_scenario_benchmarks"
        || token.ends_with("_rule_fixtures_have_scenario_benchmarks")
}

pub(super) fn default_benchmark_toml_template() -> String {
    [
        "generator:",
        "use asp_rust_build_support::{asp_rust_scenario, measure_asp_rust_scenario, render_asp_rust_scenario_benchmark_toml};",
        "",
        "let scenario = asp_rust_scenario! {",
        "    # Declare identity, commands, budgets, warmup_iterations, measure_iterations, and typed metrics here.",
        "    # Do not declare observed_total or observed_timings.",
        "};",
        "let measurement = measure_asp_rust_scenario(&scenario, || run_real_scenario())?;",
        "let benchmark_toml = render_asp_rust_scenario_benchmark_toml(&scenario, &measurement)?;",
        "# Publish benchmark_toml as the measured receipt. A zero clock sample is remeasured, never replaced with 1ns/1us.",
    ]
    .join("\n")
}
