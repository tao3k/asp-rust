//! Declarative Scenario measurement shared by ASP Rust and downstream workspaces.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AspRustScenarioCommand {
    pub label: &'static str,
    pub argv: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AspRustScenarioMetricKind {
    Stable,
    Exact,
    Maximum,
    Minimum,
}

impl AspRustScenarioMetricKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Exact => "exact",
            Self::Maximum => "maximum",
            Self::Minimum => "minimum",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AspRustScenarioMetricSpec {
    pub name: &'static str,
    pub unit: &'static str,
    pub kind: AspRustScenarioMetricKind,
    pub target: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AspRustScenarioObservation {
    pub memory_bytes: u64,
    pub phase_timings: BTreeMap<String, Duration>,
    pub metrics: BTreeMap<String, u64>,
}

impl AspRustScenarioObservation {
    #[must_use]
    pub fn with_memory_bytes(mut self, memory_bytes: u64) -> Self {
        self.memory_bytes = memory_bytes;
        self
    }

    #[must_use]
    pub fn with_timing(mut self, name: impl Into<String>, duration: Duration) -> Self {
        self.phase_timings.insert(name.into(), duration);
        self
    }

    #[must_use]
    pub fn with_metric(mut self, name: impl Into<String>, value: u64) -> Self {
        self.metrics.insert(name.into(), value);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AspRustScenarioBenchmarkSpec {
    pub harness: &'static str,
    pub test: &'static str,
    pub snapshot: &'static str,
    pub target_total: &'static str,
    pub max_total: &'static str,
    pub regression_budget: &'static str,
    pub memory_budget_bytes: u64,
    pub target_rationale: &'static str,
    pub warmup_iterations: usize,
    pub measure_iterations: usize,
    pub metrics: Vec<AspRustScenarioMetricSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AspRustScenario {
    pub name: &'static str,
    pub package_name: &'static str,
    pub description: &'static str,
    pub fixture_root: &'static str,
    pub tags: &'static [&'static str],
    pub commands: &'static [AspRustScenarioCommand],
    pub benchmark: Option<AspRustScenarioBenchmarkSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AspRustScenarioPackage {
    pub package_name: &'static str,
    pub scenarios: Vec<AspRustScenario>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AspRustScenarioMeasurement {
    pub observed_total: Duration,
    pub total_p50: Duration,
    pub total_p95: Duration,
    pub total_max: Duration,
    pub observed_memory_bytes: u64,
    pub observed_timings: BTreeMap<String, Duration>,
    pub metrics: BTreeMap<String, u64>,
    pub clock: &'static str,
    pub statistic: &'static str,
    pub warmup_iterations: usize,
    pub measure_iterations: usize,
}

/// Execute warmup and measured samples with a monotonic clock.
///
/// The generated observation is the p95 sample, never a hand-authored duration.
pub fn measure_asp_rust_scenario(
    scenario: &AspRustScenario,
    mut run: impl FnMut() -> AspRustScenarioObservation,
) -> Result<AspRustScenarioMeasurement, String> {
    let benchmark = scenario
        .benchmark
        .as_ref()
        .ok_or_else(|| format!("Scenario {} has no benchmark contract", scenario.name))?;
    if benchmark.measure_iterations == 0 {
        return Err("Scenario measure_iterations must be nonzero".to_string());
    }
    for _ in 0..benchmark.warmup_iterations {
        let _ = run();
    }
    let mut totals = Vec::with_capacity(benchmark.measure_iterations);
    let mut phase_samples = BTreeMap::<String, Vec<Duration>>::new();
    let mut phase_names = None;
    let mut stable_metrics = None;
    let mut observed_memory_bytes = 0;
    for _ in 0..benchmark.measure_iterations {
        let started_at = Instant::now();
        let observation = run();
        totals.push(started_at.elapsed());
        observed_memory_bytes = observed_memory_bytes.max(observation.memory_bytes);
        let current_phase_names = observation
            .phase_timings
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        match &phase_names {
            None => phase_names = Some(current_phase_names),
            Some(expected) if expected == &current_phase_names => {}
            Some(_) => return Err("Scenario timing phases changed between samples".to_string()),
        }
        for (name, duration) in observation.phase_timings {
            phase_samples.entry(name).or_default().push(duration);
        }
        match &stable_metrics {
            None => stable_metrics = Some(observation.metrics),
            Some(expected) if expected == &observation.metrics => {}
            Some(_) => return Err("Scenario custom metrics changed between samples".to_string()),
        }
    }
    let total_p50 = percentile(&mut totals.clone(), 50);
    let total_p95 = percentile(&mut totals, 95);
    let total_max = *totals
        .last()
        .ok_or_else(|| "Scenario measurement produced no samples".to_string())?;
    let observed_total = total_p95;
    if observed_total.is_zero() {
        return Err(
            "Scenario measurement resolution is insufficient: p95 elapsed time is zero".to_string(),
        );
    }
    let observed_timings = phase_samples
        .into_iter()
        .map(|(name, mut samples)| (name, percentile(&mut samples, 95)))
        .collect();
    let metrics = stable_metrics.unwrap_or_default();
    validate_metric_observations(&benchmark.metrics, &metrics)?;
    Ok(AspRustScenarioMeasurement {
        observed_total,
        total_p50,
        total_p95,
        total_max,
        observed_memory_bytes,
        observed_timings,
        metrics,
        clock: "std::time::Instant",
        statistic: "p95",
        warmup_iterations: benchmark.warmup_iterations,
        measure_iterations: benchmark.measure_iterations,
    })
}

/// Render `benchmark.toml` from a real Scenario measurement.
pub fn render_asp_rust_scenario_benchmark_toml(
    scenario: &AspRustScenario,
    measurement: &AspRustScenarioMeasurement,
) -> Result<String, String> {
    let benchmark = scenario
        .benchmark
        .as_ref()
        .ok_or_else(|| format!("Scenario {} has no benchmark contract", scenario.name))?;
    let mut output = String::new();
    push_toml_string(&mut output, "harness", benchmark.harness);
    push_toml_string(&mut output, "test", benchmark.test);
    push_toml_string(&mut output, "snapshot", benchmark.snapshot);
    push_toml_string(&mut output, "target_total", benchmark.target_total);
    push_toml_string(&mut output, "max_total", benchmark.max_total);
    push_toml_string(
        &mut output,
        "observed_total",
        &format_duration(measurement.observed_total),
    );
    push_toml_string(
        &mut output,
        "regression_budget",
        benchmark.regression_budget,
    );
    writeln!(
        output,
        "memory_budget_bytes = {}",
        benchmark.memory_budget_bytes
    )
    .expect("write String");
    writeln!(
        output,
        "observed_memory_bytes = {}",
        measurement.observed_memory_bytes
    )
    .expect("write String");
    push_toml_string(&mut output, "target_rationale", benchmark.target_rationale);
    output.push_str("\n[measurement]\n");
    push_toml_string(&mut output, "clock", measurement.clock);
    push_toml_string(&mut output, "statistic", measurement.statistic);
    writeln!(
        output,
        "warmup_iterations = {}",
        measurement.warmup_iterations
    )
    .expect("write String");
    writeln!(
        output,
        "measure_iterations = {}",
        measurement.measure_iterations
    )
    .expect("write String");
    push_toml_string(
        &mut output,
        "total_p50",
        &format_duration(measurement.total_p50),
    );
    push_toml_string(
        &mut output,
        "total_p95",
        &format_duration(measurement.total_p95),
    );
    push_toml_string(
        &mut output,
        "total_max",
        &format_duration(measurement.total_max),
    );
    output.push_str("\n[observed_timings]\n");
    for (name, duration) in &measurement.observed_timings {
        push_toml_string(&mut output, name, &format_duration(*duration));
    }
    for metric in &benchmark.metrics {
        let observed = measurement
            .metrics
            .get(metric.name)
            .ok_or_else(|| format!("Scenario metric {} is missing", metric.name))?;
        writeln!(output, "\n[metrics.{}]", metric.name).expect("write String");
        push_toml_string(&mut output, "unit", metric.unit);
        push_toml_string(&mut output, "kind", metric.kind.as_str());
        if let Some(target) = metric.target {
            writeln!(output, "target = {target}").expect("write String");
        }
        writeln!(output, "observed = {observed}").expect("write String");
    }
    Ok(output)
}

pub fn write_asp_rust_scenario_benchmark_toml(
    scenario_root: &Path,
    scenario: &AspRustScenario,
    measurement: &AspRustScenarioMeasurement,
) -> Result<(), String> {
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, measurement)?;
    let destination = scenario_root.join("benchmark.toml");
    let temporary = scenario_root.join(format!(".benchmark.toml.{}.tmp", std::process::id()));
    fs::write(&temporary, rendered)
        .map_err(|error| format!("write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &destination).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("publish {}: {error}", destination.display())
    })
}

fn validate_metric_observations(
    specs: &[AspRustScenarioMetricSpec],
    observations: &BTreeMap<String, u64>,
) -> Result<(), String> {
    for spec in specs {
        let observed = observations
            .get(spec.name)
            .ok_or_else(|| format!("Scenario metric {} is missing", spec.name))?;
        let admitted = match (spec.kind, spec.target) {
            (AspRustScenarioMetricKind::Stable, None) => true,
            (AspRustScenarioMetricKind::Exact, Some(target)) => *observed == target,
            (AspRustScenarioMetricKind::Maximum, Some(target)) => *observed <= target,
            (AspRustScenarioMetricKind::Minimum, Some(target)) => *observed >= target,
            (AspRustScenarioMetricKind::Stable, Some(_)) => {
                return Err(format!(
                    "Scenario stable metric {} must not declare a target",
                    spec.name
                ));
            }
            (_, None) => {
                return Err(format!(
                    "Scenario constrained metric {} must declare a target",
                    spec.name
                ));
            }
        };
        if !admitted {
            return Err(format!(
                "Scenario metric {} observed {} does not satisfy {} target {} {}",
                spec.name,
                observed,
                spec.kind.as_str(),
                spec.target.expect("validated constrained metric target"),
                spec.unit
            ));
        }
    }
    Ok(())
}

fn percentile(samples: &mut [Duration], percentile: usize) -> Duration {
    samples.sort_unstable();
    let index = (samples.len() * percentile).div_ceil(100).saturating_sub(1);
    samples[index]
}

fn format_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    if nanos >= 1_000_000 && nanos.is_multiple_of(1_000_000) {
        format!("{}ms", nanos / 1_000_000)
    } else if nanos >= 1_000 && nanos.is_multiple_of(1_000) {
        format!("{}us", nanos / 1_000)
    } else {
        format!("{nanos}ns")
    }
}

fn push_toml_string(output: &mut String, key: &str, value: &str) {
    writeln!(output, "{key} = \"{}\"", escape_toml_string(value)).expect("write String");
}

fn escape_toml_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            character => vec![character],
        })
        .collect()
}

#[macro_export]
macro_rules! asp_rust_scenario {
    (
        name: $name:expr,
        package: $package_name:expr,
        description: $description:expr,
        fixture_root: $fixture_root:expr,
        tags: [$($tag:expr),* $(,)?],
        commands: [$( { label: $label:expr, argv: [$($argv:expr),* $(,)?] } ),* $(,)?],
        benchmark: {
            harness: $harness:expr,
            test: $test:expr,
            snapshot: $snapshot:expr,
            target_total: $target_total:expr,
            max_total: $max_total:expr,
            regression_budget: $regression_budget:expr,
            memory_budget_bytes: $memory_budget_bytes:expr,
            target_rationale: $target_rationale:expr,
            warmup_iterations: $warmup_iterations:expr,
            measure_iterations: $measure_iterations:expr,
            metrics: [$( {
                name: $metric_name:expr,
                unit: $metric_unit:expr,
                kind: $metric_kind:ident,
                $(target: $metric_target:expr)?
            } ),* $(,)?]
        } $(,)?
    ) => {
        $crate::AspRustScenario {
            name: $name,
            package_name: $package_name,
            description: $description,
            fixture_root: $fixture_root,
            tags: &[$($tag),*],
            commands: &[$($crate::AspRustScenarioCommand { label: $label, argv: &[$($argv),*] }),*],
            benchmark: Some($crate::AspRustScenarioBenchmarkSpec {
                harness: $harness,
                test: $test,
                snapshot: $snapshot,
                target_total: $target_total,
                max_total: $max_total,
                regression_budget: $regression_budget,
                memory_budget_bytes: $memory_budget_bytes,
                target_rationale: $target_rationale,
                warmup_iterations: $warmup_iterations,
                measure_iterations: $measure_iterations,
                metrics: vec![$($crate::AspRustScenarioMetricSpec {
                    name: $metric_name,
                    unit: $metric_unit,
                    kind: $crate::AspRustScenarioMetricKind::$metric_kind,
                    target: None$(.or(Some($metric_target)))?,
                }),*],
            }),
        }
    };
    (
        name: $name:expr,
        package: $package_name:expr,
        description: $description:expr,
        fixture_root: $fixture_root:expr,
        tags: [$($tag:expr),* $(,)?],
        commands: [$( { label: $label:expr, argv: [$($argv:expr),* $(,)?] } ),* $(,)?] $(,)?
    ) => {
        $crate::AspRustScenario {
            name: $name,
            package_name: $package_name,
            description: $description,
            fixture_root: $fixture_root,
            tags: &[$($tag),*],
            commands: &[$($crate::AspRustScenarioCommand { label: $label, argv: &[$($argv),*] }),*],
            benchmark: None,
        }
    };
}

#[macro_export]
macro_rules! asp_rust_scenario_package {
    (package: $package_name:expr, scenarios: [$($scenario:expr),* $(,)?] $(,)?) => {
        $crate::AspRustScenarioPackage {
            package_name: $package_name,
            scenarios: vec![$($scenario),*],
        }
    };
}
