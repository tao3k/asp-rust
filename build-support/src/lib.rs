//! Lightweight build-script evidence functions for ASP Rust packages.

mod provider_contract;
mod scenario;

pub use provider_contract::emit_provider_contract_digest;
pub use scenario::{
    AspRustScenario, AspRustScenarioBenchmarkSpec, AspRustScenarioCommand,
    AspRustScenarioMeasurement, AspRustScenarioMetricKind, AspRustScenarioMetricSpec,
    AspRustScenarioObservation, AspRustScenarioPackage, measure_asp_rust_scenario,
    render_asp_rust_scenario_benchmark_toml, write_asp_rust_scenario_benchmark_toml,
};
