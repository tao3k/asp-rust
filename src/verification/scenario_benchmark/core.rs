//! Scenario benchmark contracts for ASP Rust fixtures.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::time::Duration;

use super::contract::{RustScenarioBenchmarkContract, RustScenarioBenchmarkDuration};
use super::contract_gate::{
    benchmark_entry_targets_contract_gate, default_benchmark_toml_template,
};
use super::discovery::discover_required_rust_scenario_benchmarks;
use super::types::{
    RustScenarioBenchmarkError, RustScenarioBenchmarkManifestKind,
    RustScenarioBenchmarkPolicyCoverage, RustScenarioBenchmarkPolicyId,
    RustScenarioBenchmarkPolicyRuleId, RustScenarioBenchmarkReceipt,
    RustScenarioBenchmarkRequirement, RustScenarioBenchmarkScenarioId, RustScenarioBenchmarkStatus,
    RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkViolation,
    RustScenarioBenchmarkViolationKind, RustScenarioMetadata,
};
use crate::rules::agent_policy::rust_agent_policy_scenario_requirements;
use serde::Deserialize;

const RUST_SCENARIO_BENCHMARK_HARD_MAX_TOTAL: RustScenarioBenchmarkDuration =
    RustScenarioBenchmarkDuration(Duration::from_millis(500));
const SCENARIO_POLICY_ID_GRAMMAR: &str = "RUST-AGENT-<TAGS>-<NUMBER>";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RustAstPatchScenarioManifest {
    mode: String,
    expected_status: String,
    expected_capability: String,
    #[serde(default)]
    expected_operation: Option<String>,
}

/// Validate one scenario root containing `scenario.toml` and `benchmark.toml`.
pub fn validate_rust_scenario_benchmark(
    root: impl AsRef<Path>,
) -> Result<RustScenarioBenchmarkReceipt, RustScenarioBenchmarkError> {
    let root = root.as_ref();
    let scenario_path = root.join("scenario.toml");
    let benchmark_path = root.join("benchmark.toml");
    let scenario: RustScenarioMetadata = read_toml(&scenario_path)?;
    let benchmark: RustScenarioBenchmarkContract = read_toml(&benchmark_path)?;
    let violations = scenario_benchmark_violations(&scenario, &benchmark);
    let status = scenario_benchmark_status(&violations);
    Ok(RustScenarioBenchmarkReceipt {
        root: root.to_path_buf(),
        scenario,
        benchmark,
        status,
        violations,
    })
}

/// Validate every required ASP Rust scenario benchmark contract.
pub fn validate_required_rust_scenario_benchmarks(
    crate_root: impl AsRef<Path>,
) -> Result<RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkError> {
    let crate_root = crate_root.as_ref();
    let requirements = discover_required_rust_scenario_benchmarks(crate_root)?;
    let mut receipts = Vec::new();
    let mut violations = Vec::new();
    for requirement in &requirements {
        let benchmark_path = requirement.root.join("benchmark.toml");
        if !benchmark_path.exists() {
            violations.push(RustScenarioBenchmarkViolation {
                kind: RustScenarioBenchmarkViolationKind::Contract,
                field: display_suite_path(crate_root, &benchmark_path),
                message: "required benchmark.toml is missing".to_string(),
            });
            continue;
        }
        receipts.push(validate_rust_scenario_benchmark_requirement(requirement)?);
    }
    let policy_coverage = if crate_root.join("src/rules/agent_policy/pack.rs").is_file() {
        validate_agent_policy_scenario_coverage(crate_root, &receipts, &mut violations)
    } else {
        Vec::new()
    };
    let status = scenario_benchmark_suite_status(&receipts, &violations);
    Ok(RustScenarioBenchmarkSuiteReceipt {
        root: crate_root.to_path_buf(),
        requirements,
        receipts,
        policy_coverage,
        violations,
        status,
    })
}

/// Assert that every discovered rule or scenario fixture has a benchmark contract.
///
/// This is the downstream unit-test entrypoint. It intentionally fails hard and
/// renders a repair template instead of returning an advisory warning.
pub fn assert_rule_fixture_scenario_benchmarks(crate_root: impl AsRef<Path>) {
    let crate_root = crate_root.as_ref();
    let receipt = validate_required_rust_scenario_benchmarks(crate_root).unwrap_or_else(|error| {
        panic!(
            "scenario benchmark hard gate could not read {}: {error}",
            crate_root.display()
        )
    });
    if receipt.status != RustScenarioBenchmarkStatus::Pass {
        panic!("{}", scenario_benchmark_hard_gate_failure_message(&receipt));
    }
}

fn scenario_benchmark_hard_gate_failure_message(
    receipt: &RustScenarioBenchmarkSuiteReceipt,
) -> String {
    let mut lines = vec![
        "scenario benchmark hard gate failed".to_string(),
        "preferred fix: add benchmark.toml next to the scenario fixture".to_string(),
        "do not add advisory mode, fixture-local opt-out, or expires".to_string(),
        format!("status: {}", receipt.status.as_str()),
    ];
    for violation in &receipt.violations {
        lines.push(format!(
            "- {}:{}: {}",
            violation.kind.as_str(),
            violation.field,
            violation.message
        ));
        if violation.field.ends_with("benchmark.toml") {
            lines.push(format!("create: {}", violation.field));
            lines.push(default_benchmark_toml_template());
        }
    }
    for scenario_receipt in &receipt.receipts {
        for violation in &scenario_receipt.violations {
            lines.push(format!(
                "- {}:{}:{}: {}",
                violation.kind.as_str(),
                display_suite_path(&receipt.root, &scenario_receipt.root),
                violation.field,
                violation.message
            ));
        }
    }
    lines.join("\n")
}

fn display_suite_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn validate_rust_scenario_benchmark_requirement(
    requirement: &RustScenarioBenchmarkRequirement,
) -> Result<RustScenarioBenchmarkReceipt, RustScenarioBenchmarkError> {
    match requirement.manifest_kind {
        RustScenarioBenchmarkManifestKind::ScenarioToml => {
            validate_rust_scenario_benchmark(&requirement.root)
        }
        RustScenarioBenchmarkManifestKind::AstPatchScenarioJson => {
            validate_ast_patch_scenario_benchmark(&requirement.root)
        }
    }
}

fn validate_agent_policy_scenario_coverage(
    crate_root: &Path,
    receipts: &[RustScenarioBenchmarkReceipt],
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
) -> Vec<RustScenarioBenchmarkPolicyCoverage> {
    let receipt_by_root = receipts
        .iter()
        .map(|receipt| (receipt.root.clone(), receipt))
        .collect::<BTreeMap<_, _>>();
    let policy_ids_by_root = receipts
        .iter()
        .map(|receipt| {
            (
                receipt.root.clone(),
                receipt
                    .scenario
                    .policy_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut coverage = Vec::new();
    for requirement in rust_agent_policy_scenario_requirements() {
        if !is_agent_policy_id(requirement.policy_id) {
            violations.push(RustScenarioBenchmarkViolation {
                kind: RustScenarioBenchmarkViolationKind::Contract,
                field: format!(
                    "agent_policy_requirements.{}.policy_id",
                    requirement.rule_id
                ),
                message: format!(
                    "policy id {} must match {}",
                    requirement.policy_id, SCENARIO_POLICY_ID_GRAMMAR
                ),
            });
            continue;
        }
        let scenario_root = crate_root.join(requirement.scenario_root);
        let Some(receipt) = receipt_by_root.get(&scenario_root) else {
            violations.push(RustScenarioBenchmarkViolation {
                kind: RustScenarioBenchmarkViolationKind::Contract,
                field: display_suite_path(crate_root, &scenario_root),
                message: format!(
                    "agent policy {} requires scenario {} with policy {}",
                    requirement.rule_id, requirement.scenario_id, requirement.policy_id
                ),
            });
            continue;
        };

        let mut missing = Vec::new();
        if receipt.scenario.id != requirement.scenario_id {
            missing.push(format!("scenario.id={}", requirement.scenario_id));
        }
        if !policy_ids_by_root
            .get(&scenario_root)
            .is_some_and(|policy_ids| policy_ids.contains(requirement.policy_id))
        {
            missing.push(format!("policy_ids contains {}", requirement.policy_id));
        }
        if missing.is_empty() {
            coverage.push(RustScenarioBenchmarkPolicyCoverage {
                rule_id: RustScenarioBenchmarkPolicyRuleId::new(requirement.rule_id),
                scenario_id: RustScenarioBenchmarkScenarioId::new(requirement.scenario_id),
                policy_id: RustScenarioBenchmarkPolicyId::new(requirement.policy_id),
                root: scenario_root,
            });
        } else {
            violations.push(RustScenarioBenchmarkViolation {
                kind: RustScenarioBenchmarkViolationKind::Contract,
                field: display_suite_path(crate_root, &scenario_root),
                message: format!(
                    "agent policy {} scenario coverage is incomplete: {}",
                    requirement.rule_id,
                    missing.join(", ")
                ),
            });
        }
    }
    coverage.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    coverage
}

fn validate_ast_patch_scenario_benchmark(
    root: &Path,
) -> Result<RustScenarioBenchmarkReceipt, RustScenarioBenchmarkError> {
    let manifest: RustAstPatchScenarioManifest = read_json(&root.join("scenario.json"))?;
    let benchmark: RustScenarioBenchmarkContract = read_toml(&root.join("benchmark.toml"))?;
    let scenario = RustScenarioMetadata {
        id: root
            .file_name()
            .map(|file_name| file_name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ast-patch-scenario".to_string()),
        title: ast_patch_scenario_title(root),
        policy_ids: vec!["RUST-AGENT-AST-PATCH-SCENARIO-001".to_string()],
        agent_goal: format!(
            "Keep AST patch {mode} scenario bounded for {status}/{capability}.",
            mode = manifest.mode,
            status = manifest.expected_status,
            capability = manifest.expected_capability
        ),
        reference_repositories: vec!["rust-lang/rust".to_string()],
        reference_patterns: vec![
            "AST edits are validated through explicit operation receipts before mutation"
                .to_string(),
        ],
        inputs: "input".to_string(),
        expected: "expected".to_string(),
    };
    let mut violations = scenario_benchmark_violations(&scenario, &benchmark);
    require_non_empty(&mut violations, "scenario.mode", &manifest.mode);
    require_non_empty(
        &mut violations,
        "scenario.expected_status",
        &manifest.expected_status,
    );
    require_non_empty(
        &mut violations,
        "scenario.expected_capability",
        &manifest.expected_capability,
    );
    if manifest
        .expected_operation
        .as_deref()
        .unwrap_or("")
        .is_empty()
        && manifest.expected_status == "applied"
    {
        violations.push(contract_violation(
            "scenario.expected_operation",
            "applied AST patch scenarios must name the expected operation",
        ));
    }
    let status = scenario_benchmark_status(&violations);
    Ok(RustScenarioBenchmarkReceipt {
        root: root.to_path_buf(),
        scenario,
        benchmark,
        status,
        violations,
    })
}

fn read_toml<T>(path: &Path) -> Result<T, RustScenarioBenchmarkError>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fs::read_to_string(path)
        .map_err(|error| RustScenarioBenchmarkError::new(path, error.to_string()))?;
    toml::from_str(&text).map_err(|error| RustScenarioBenchmarkError::new(path, error.to_string()))
}

fn read_json<T>(path: &Path) -> Result<T, RustScenarioBenchmarkError>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fs::read_to_string(path)
        .map_err(|error| RustScenarioBenchmarkError::new(path, error.to_string()))?;
    serde_json::from_str(&text)
        .map_err(|error| RustScenarioBenchmarkError::new(path, error.to_string()))
}

fn scenario_benchmark_status(
    violations: &[RustScenarioBenchmarkViolation],
) -> RustScenarioBenchmarkStatus {
    if violations
        .iter()
        .any(|violation| violation.kind == RustScenarioBenchmarkViolationKind::Contract)
    {
        return RustScenarioBenchmarkStatus::Invalid;
    }
    if violations.is_empty() {
        RustScenarioBenchmarkStatus::Pass
    } else {
        RustScenarioBenchmarkStatus::Fail
    }
}

fn scenario_benchmark_suite_status(
    receipts: &[RustScenarioBenchmarkReceipt],
    violations: &[RustScenarioBenchmarkViolation],
) -> RustScenarioBenchmarkStatus {
    if !violations.is_empty()
        || receipts
            .iter()
            .any(|receipt| receipt.status == RustScenarioBenchmarkStatus::Invalid)
    {
        return RustScenarioBenchmarkStatus::Invalid;
    }
    if receipts
        .iter()
        .any(|receipt| receipt.status == RustScenarioBenchmarkStatus::Fail)
    {
        return RustScenarioBenchmarkStatus::Fail;
    }
    RustScenarioBenchmarkStatus::Pass
}

fn scenario_benchmark_violations(
    scenario: &RustScenarioMetadata,
    benchmark: &RustScenarioBenchmarkContract,
) -> Vec<RustScenarioBenchmarkViolation> {
    let mut violations = Vec::new();
    require_non_empty(&mut violations, "scenario.id", &scenario.id);
    require_non_empty(&mut violations, "scenario.title", &scenario.title);
    require_non_empty(&mut violations, "scenario.agent_goal", &scenario.agent_goal);
    require_non_empty(&mut violations, "scenario.inputs", &scenario.inputs);
    require_non_empty(&mut violations, "scenario.expected", &scenario.expected);
    require_non_empty_list(
        &mut violations,
        "scenario.reference_repositories",
        &scenario.reference_repositories,
    );
    require_non_empty_list(
        &mut violations,
        "scenario.reference_patterns",
        &scenario.reference_patterns,
    );
    require_policy_ids(&mut violations, "scenario.policy_ids", &scenario.policy_ids);
    require_non_empty(&mut violations, "benchmark.harness", &benchmark.harness);
    require_supported_harness(&mut violations, &benchmark.harness);
    require_benchmark_entry(&mut violations, benchmark);
    if benchmark_entry_targets_contract_gate(benchmark) {
        violations.push(contract_violation(
            "benchmark.entry",
            "benchmark entry must name a focused Rust test or bench case, not the scenario benchmark contract gate",
        ));
    }
    require_non_empty(
        &mut violations,
        "benchmark.target_rationale",
        &benchmark.target_rationale,
    );
    require_input_expected_comparison(&mut violations, benchmark);
    require_positive(
        &mut violations,
        "benchmark.target_total",
        benchmark.target_total.is_zero(),
    );
    require_positive(
        &mut violations,
        "benchmark.max_total",
        benchmark.max_total.is_zero(),
    );
    require_positive(
        &mut violations,
        "benchmark.regression_budget",
        benchmark.regression_budget.is_zero(),
    );
    require_positive(
        &mut violations,
        "benchmark.memory_budget_bytes",
        benchmark.memory_budget_bytes.as_u64() == 0,
    );
    if benchmark.observed_timings.is_empty() {
        violations.push(contract_violation(
            "benchmark.observed_timings",
            "at least one timing phase is required",
        ));
    }
    require_measurement_provenance(&mut violations, benchmark);
    if benchmark.target_total > benchmark.max_total {
        violations.push(contract_violation(
            "benchmark.target_total",
            "target_total must be less than or equal to max_total",
        ));
    }
    if benchmark.max_total > RUST_SCENARIO_BENCHMARK_HARD_MAX_TOTAL {
        violations.push(contract_violation(
            "benchmark.max_total",
            &format!(
                "max_total must be <= {RUST_SCENARIO_BENCHMARK_HARD_MAX_TOTAL} for the hard gate",
            ),
        ));
    }
    if benchmark.observed_total > benchmark.max_total {
        violations.push(RustScenarioBenchmarkViolation {
            kind: RustScenarioBenchmarkViolationKind::Performance,
            field: "benchmark.observed_total".to_string(),
            message: format!(
                "observed {} exceeds max {}",
                benchmark.observed_total, benchmark.max_total
            ),
        });
    }
    if benchmark.observed_memory_bytes > benchmark.memory_budget_bytes {
        violations.push(RustScenarioBenchmarkViolation {
            kind: RustScenarioBenchmarkViolationKind::Memory,
            field: "benchmark.observed_memory_bytes".to_string(),
            message: format!(
                "observed {} bytes exceeds budget {} bytes",
                benchmark.observed_memory_bytes, benchmark.memory_budget_bytes
            ),
        });
    }
    require_custom_metrics(&mut violations, benchmark);
    violations
}

fn require_custom_metrics(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    benchmark: &RustScenarioBenchmarkContract,
) {
    for (name, metric) in &benchmark.metrics {
        let field = format!("benchmark.metrics.{name}");
        require_non_empty(violations, &format!("{field}.name"), name);
        require_non_empty(violations, &format!("{field}.unit"), &metric.unit);
        let passes = match (metric.kind, metric.target) {
            (super::contract::RustScenarioBenchmarkMetricKind::Stable, None) => true,
            (super::contract::RustScenarioBenchmarkMetricKind::Exact, Some(target)) => {
                metric.observed == target
            }
            (super::contract::RustScenarioBenchmarkMetricKind::Maximum, Some(target)) => {
                metric.observed <= target
            }
            (super::contract::RustScenarioBenchmarkMetricKind::Minimum, Some(target)) => {
                metric.observed >= target
            }
            (super::contract::RustScenarioBenchmarkMetricKind::Stable, Some(_)) => {
                violations.push(contract_violation(
                    &format!("{field}.target"),
                    "stable observations must not duplicate a fixture value as a target",
                ));
                continue;
            }
            (_, None) => {
                violations.push(contract_violation(
                    &format!("{field}.target"),
                    "exact, maximum, and minimum metrics require a declared target",
                ));
                continue;
            }
        };
        if !passes {
            violations.push(RustScenarioBenchmarkViolation {
                kind: RustScenarioBenchmarkViolationKind::Performance,
                field,
                message: format!(
                    "observed {} does not satisfy {:?} target {} {}",
                    metric.observed,
                    metric.kind,
                    metric.target.expect("validated constrained metric target"),
                    metric.unit
                ),
            });
        }
    }
}

fn require_measurement_provenance(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    benchmark: &RustScenarioBenchmarkContract,
) {
    if benchmark.metrics.is_empty() && benchmark.measurement.is_none() {
        return;
    }
    let Some(measurement) = &benchmark.measurement else {
        violations.push(contract_violation(
            "benchmark.measurement",
            "generated metrics require monotonic-clock measurement provenance",
        ));
        return;
    };
    if measurement.clock != "std::time::Instant" {
        violations.push(contract_violation(
            "benchmark.measurement.clock",
            "generated Scenario observations must use std::time::Instant",
        ));
    }
    if measurement.statistic != "p95" {
        violations.push(contract_violation(
            "benchmark.measurement.statistic",
            "observed_total must be the measured p95 sample",
        ));
    }
    if measurement.measure_iterations == 0 {
        violations.push(contract_violation(
            "benchmark.measurement.measure_iterations",
            "measurement must contain at least one real sample",
        ));
    }
    if measurement.total_p50 > measurement.total_p95
        || measurement.total_p95 > measurement.total_max
    {
        violations.push(contract_violation(
            "benchmark.measurement",
            "measured distribution must satisfy p50 <= p95 <= max",
        ));
    }
    if benchmark.observed_total != measurement.total_p95 {
        violations.push(contract_violation(
            "benchmark.observed_total",
            "observed_total must equal the runner-generated p95 measurement",
        ));
    }
    if measurement.total_p95.is_zero() {
        violations.push(contract_violation(
            "benchmark.measurement.total_p95",
            "measured p95 is zero; clock resolution or sample workload is insufficient, rerun the Scenario instead of inventing a duration",
        ));
    }
    for (name, timing) in &benchmark.observed_timings {
        if name.ends_with("_count") {
            violations.push(contract_violation(
                &format!("benchmark.observed_timings.{name}"),
                "counts are typed metrics, not durations",
            ));
        }
        if timing.is_zero() {
            violations.push(contract_violation(
                &format!("benchmark.observed_timings.{name}"),
                "measured phase resolved to zero; increase the sample workload and remeasure",
            ));
        }
    }
}

fn require_input_expected_comparison(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    benchmark: &RustScenarioBenchmarkContract,
) {
    let Some(comparison) = &benchmark.input_expected_comparison else {
        return;
    };
    require_positive(
        violations,
        "benchmark.input_expected_comparison.input_total",
        comparison.input_total.is_zero(),
    );
    require_positive(
        violations,
        "benchmark.input_expected_comparison.expected_total",
        comparison.expected_total.is_zero(),
    );
    require_positive(
        violations,
        "benchmark.input_expected_comparison.input_memory_bytes",
        comparison.input_memory_bytes.as_u64() == 0,
    );
    require_positive(
        violations,
        "benchmark.input_expected_comparison.expected_memory_bytes",
        comparison.expected_memory_bytes.as_u64() == 0,
    );
    require_non_empty(
        violations,
        "benchmark.input_expected_comparison.interpretation",
        &comparison.interpretation,
    );
    if comparison.expected_total >= comparison.input_total
        || comparison.expected_memory_bytes >= comparison.input_memory_bytes
    {
        require_non_empty(
            violations,
            "benchmark.input_expected_comparison.expected_not_faster_annotation",
            comparison
                .expected_not_faster_annotation
                .as_deref()
                .unwrap_or(""),
        );
    }
}

fn require_non_empty(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    field: &str,
    value: &str,
) {
    if value.trim().is_empty() {
        violations.push(contract_violation(field, "field must not be empty"));
    }
}

fn require_non_empty_list(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    field: &str,
    values: &[String],
) {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        violations.push(contract_violation(
            field,
            "list must contain at least one non-empty value",
        ));
    }
}

fn require_policy_ids(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    field: &str,
    policy_ids: &[String],
) {
    if policy_ids.is_empty() {
        violations.push(contract_violation(
            field,
            "at least one policy id is required",
        ));
        return;
    }
    for (index, policy_id) in policy_ids.iter().enumerate() {
        if !is_scenario_policy_id(policy_id) {
            violations.push(contract_violation(
                &format!("{field}[{index}]"),
                &format!("policy id must match {SCENARIO_POLICY_ID_GRAMMAR}"),
            ));
        }
    }
}

fn is_scenario_policy_id(value: &str) -> bool {
    is_agent_policy_id(value)
}

fn is_agent_policy_id(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() < 4 {
        return false;
    }
    let Some(agent_index) = parts.iter().position(|part| *part == "AGENT") else {
        return false;
    };
    if agent_index == 0 || agent_index + 2 >= parts.len() {
        return false;
    }
    parts[..agent_index].iter().all(|part| is_upper_token(part))
        && parts[agent_index + 1..parts.len() - 1]
            .iter()
            .all(|part| is_upper_token(part))
        && parts
            .last()
            .is_some_and(|number| number.len() >= 3 && is_ascii_digits(number))
}

fn is_upper_token(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_uppercase())
        && chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn is_ascii_digits(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
}

fn require_positive(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    field: &str,
    is_zero: bool,
) {
    if is_zero {
        violations.push(contract_violation(field, "field must be greater than zero"));
    }
}

fn require_supported_harness(violations: &mut Vec<RustScenarioBenchmarkViolation>, harness: &str) {
    if harness.trim().is_empty() {
        return;
    }
    if !matches!(
        harness.trim(),
        "libtest" | "criterion" | "divan" | "iai-callgrind"
    ) {
        violations.push(contract_violation(
            "benchmark.harness",
            "harness must be one of libtest, criterion, divan, or iai-callgrind",
        ));
    }
}

fn require_benchmark_entry(
    violations: &mut Vec<RustScenarioBenchmarkViolation>,
    benchmark: &RustScenarioBenchmarkContract,
) {
    let test = non_empty_opt(benchmark.test.as_deref());
    let bench = non_empty_opt(benchmark.bench.as_deref());
    let case = non_empty_opt(benchmark.case.as_deref());

    match (test, bench) {
        (None, None) => violations.push(contract_violation(
            "benchmark.entry",
            "set either test for libtest gates or bench plus case for benchmark harnesses",
        )),
        (Some(_), Some(_)) => violations.push(contract_violation(
            "benchmark.entry",
            "set either test or bench, not both",
        )),
        (Some(_), None) => {
            if benchmark.harness.trim() != "libtest" {
                violations.push(contract_violation(
                    "benchmark.test",
                    "test entries must use harness = \"libtest\"",
                ));
            }
            if case.is_some() {
                violations.push(contract_violation(
                    "benchmark.case",
                    "case is only valid with bench targets",
                ));
            }
        }
        (None, Some(_)) => {
            if benchmark.harness.trim() == "libtest" {
                violations.push(contract_violation(
                    "benchmark.bench",
                    "bench entries must use criterion, divan, or iai-callgrind",
                ));
            }
            if case.is_none() {
                violations.push(contract_violation(
                    "benchmark.case",
                    "bench entries must name a focused benchmark case, group, or function",
                ));
            }
        }
    }
}

fn non_empty_opt(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn contract_violation(field: &str, message: &str) -> RustScenarioBenchmarkViolation {
    RustScenarioBenchmarkViolation {
        kind: RustScenarioBenchmarkViolationKind::Contract,
        field: field.to_string(),
        message: message.to_string(),
    }
}

fn ast_patch_scenario_title(root: &Path) -> String {
    let name = root
        .file_name()
        .map(|file_name| file_name.to_string_lossy())
        .unwrap_or_else(|| "ast_patch_scenario".into());
    format!("AST patch scenario {name}")
}
