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
use super::{
    RustScenarioBenchmarkStatus, fixture_root, validate_required_rust_scenario_benchmarks,
    validate_rust_scenario_benchmark,
};
