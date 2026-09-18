use std::collections::BTreeSet;

use asp_rust::{
    AspRustConfig, RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationSkillBinding, RustVerificationSkillDescriptor, RustVerificationTaskKind,
    RustVerificationTaskState, RustVerificationWaiver, default_asp_rust_config,
    plan_rust_project_verification_with_config, render_rust_verification_plan,
    render_rust_verification_skill_contracts,
};
use tempfile::TempDir;

use crate::verification::support::{normalize_temp_root, write_api_project};

const SKILL_CONTRACT_SCENARIOS: &[VerificationSkillContractScenario] = &[
    VerificationSkillContractScenario {
        id: "stress_k6_triggered_by_public_api",
        kind: RustVerificationTaskKind::Stress,
        responsibility: Some(RustOwnerResponsibility::PublicApi),
        descriptor: SkillDescriptorCase::K6Stress,
        resolution: ScenarioResolution::None,
    },
    VerificationSkillContractScenario {
        id: "performance_criterion_triggered_by_latency_sensitive",
        kind: RustVerificationTaskKind::Performance,
        responsibility: Some(RustOwnerResponsibility::LatencySensitive),
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::None,
    },
    VerificationSkillContractScenario {
        id: "chaos_fault_triggered_by_external_dependency",
        kind: RustVerificationTaskKind::Chaos,
        responsibility: Some(RustOwnerResponsibility::ExternalDependency),
        descriptor: SkillDescriptorCase::ChaosFault,
        resolution: ScenarioResolution::None,
    },
    VerificationSkillContractScenario {
        id: "security_scan_triggered_by_security_boundary",
        kind: RustVerificationTaskKind::Security,
        responsibility: Some(RustOwnerResponsibility::SecurityBoundary),
        descriptor: SkillDescriptorCase::SecurityScan,
        resolution: ScenarioResolution::None,
    },
    VerificationSkillContractScenario {
        id: "regression_harness_triggered_by_owner_override",
        kind: RustVerificationTaskKind::Regression,
        responsibility: Some(RustOwnerResponsibility::PureDomainLogic),
        descriptor: SkillDescriptorCase::RegressionHarness,
        resolution: ScenarioResolution::OwnerOverride,
    },
    VerificationSkillContractScenario {
        id: "descriptor_config_without_responsibility_is_not_triggered",
        kind: RustVerificationTaskKind::Performance,
        responsibility: None,
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::None,
    },
    VerificationSkillContractScenario {
        id: "public_api_profile_suppressed_with_rationale_is_not_triggered",
        kind: RustVerificationTaskKind::Stress,
        responsibility: Some(RustOwnerResponsibility::PublicApi),
        descriptor: SkillDescriptorCase::K6Stress,
        resolution: ScenarioResolution::ProfileSuppressed,
    },
    VerificationSkillContractScenario {
        id: "stress_k6_solved_by_passed_receipt",
        kind: RustVerificationTaskKind::Stress,
        responsibility: Some(RustOwnerResponsibility::PublicApi),
        descriptor: SkillDescriptorCase::K6Stress,
        resolution: ScenarioResolution::PassedReceipt,
    },
    VerificationSkillContractScenario {
        id: "performance_criterion_solved_by_structured_receipt",
        kind: RustVerificationTaskKind::Performance,
        responsibility: Some(RustOwnerResponsibility::LatencySensitive),
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::StructuredPerformanceReceipt,
    },
    VerificationSkillContractScenario {
        id: "performance_criterion_failed_receipt_stays_active",
        kind: RustVerificationTaskKind::Performance,
        responsibility: Some(RustOwnerResponsibility::LatencySensitive),
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::FailedReceipt,
    },
    VerificationSkillContractScenario {
        id: "performance_criterion_stale_receipt_is_not_solved",
        kind: RustVerificationTaskKind::Performance,
        responsibility: Some(RustOwnerResponsibility::LatencySensitive),
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::StalePassedReceipt,
    },
    VerificationSkillContractScenario {
        id: "stress_k6_incomplete_waiver_stays_active",
        kind: RustVerificationTaskKind::Stress,
        responsibility: Some(RustOwnerResponsibility::PublicApi),
        descriptor: SkillDescriptorCase::K6Stress,
        resolution: ScenarioResolution::IncompleteWaiver,
    },
    VerificationSkillContractScenario {
        id: "performance_criterion_solved_by_complete_waiver",
        kind: RustVerificationTaskKind::Performance,
        responsibility: Some(RustOwnerResponsibility::LatencySensitive),
        descriptor: SkillDescriptorCase::CriterionPerformance,
        resolution: ScenarioResolution::CompleteWaiver,
    },
];

#[derive(Clone, Copy)]
struct VerificationSkillContractScenario {
    id: &'static str,
    kind: RustVerificationTaskKind,
    responsibility: Option<RustOwnerResponsibility>,
    descriptor: SkillDescriptorCase,
    resolution: ScenarioResolution,
}

#[derive(Clone, Copy)]
enum SkillDescriptorCase {
    K6Stress,
    CriterionPerformance,
    ChaosFault,
    SecurityScan,
    RegressionHarness,
}

impl SkillDescriptorCase {
    fn descriptor(self) -> RustVerificationSkillDescriptor {
        match self {
            Self::K6Stress => RustVerificationSkillDescriptor::k6_stress(),
            Self::CriterionPerformance => RustVerificationSkillDescriptor::criterion_performance(),
            Self::ChaosFault => RustVerificationSkillDescriptor::new("rust-verification-chaos")
                .with_adapter("fault-injection")
                .with_tool("fault-injection")
                .with_command("project-owned chaos command")
                .with_standard("degradation and recovery stay within declared bounds")
                .with_required_inputs(["dependency", "failure_mode", "recovery_threshold"])
                .with_pass_criteria(["recovery=pass"])
                .with_receipt_fields(["injected_failures", "degradation", "recovery"]),
            Self::SecurityScan => {
                RustVerificationSkillDescriptor::new("rust-verification-security")
                    .with_adapter("security-scan")
                    .with_tool("security-scan")
                    .with_command("project-owned security probe")
                    .with_standard("attack classes and authorization boundary have explicit result")
                    .with_required_inputs(["attack_classes", "target", "authz_model"])
                    .with_pass_criteria(["findings=none_or_triaged"])
                    .with_receipt_fields(["attack_classes", "authorization_boundary", "findings"])
            }
            Self::RegressionHarness => {
                RustVerificationSkillDescriptor::new("rust-verification-regression")
                    .with_adapter("harness")
                    .with_tool("asp-rust")
                    .with_command("cargo test verification::regression")
                    .with_standard("architecture drift remains within configured baseline")
                    .with_required_inputs(["baseline", "owner", "thresholds"])
                    .with_pass_criteria(["drift=within_threshold"])
                    .with_receipt_fields(["source_growth", "dependency_drift", "module_cycles"])
            }
        }
    }

    fn binding(self) -> RustVerificationSkillBinding {
        let descriptor = self.descriptor();
        let mut binding = RustVerificationSkillBinding::new(descriptor.skill_id);
        if let Some(adapter) = descriptor.adapter {
            binding = binding.with_adapter(adapter);
        }
        binding
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScenarioResolution {
    None,
    OwnerOverride,
    ProfileSuppressed,
    PassedReceipt,
    StructuredPerformanceReceipt,
    FailedReceipt,
    StalePassedReceipt,
    IncompleteWaiver,
    CompleteWaiver,
}

#[test]
fn verification_skill_contract_scenario_ids_are_unique() {
    let mut ids = BTreeSet::new();

    for scenario in SKILL_CONTRACT_SCENARIOS {
        assert!(
            ids.insert(scenario.id),
            "duplicate scenario id: {}",
            scenario.id
        );
    }
}

#[test]
fn verification_skill_contract_scenarios_snapshot() {
    let rendered = SKILL_CONTRACT_SCENARIOS
        .iter()
        .map(render_skill_contract_scenario)
        .collect::<Vec<_>>()
        .join("\n");

    insta::assert_snapshot!("verification_skill_contract_scenarios", rendered);
}

fn render_skill_contract_scenario(scenario: &VerificationSkillContractScenario) -> String {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = scenario.initial_config();
    let initial_plan =
        plan_rust_project_verification_with_config(root, &config).expect("initial scenario plan");
    let final_config = match scenario.resolution {
        ScenarioResolution::None
        | ScenarioResolution::OwnerOverride
        | ScenarioResolution::ProfileSuppressed => config,
        ScenarioResolution::PassedReceipt | ScenarioResolution::StructuredPerformanceReceipt => {
            let task = initial_plan
                .active_tasks()
                .into_iter()
                .find(|task| task.kind == scenario.kind)
                .expect("active task for receipt scenario");
            config
                .with_verification_receipt(passed_receipt_for_scenario(scenario, &task.fingerprint))
        }
        ScenarioResolution::FailedReceipt => {
            let task = initial_plan
                .active_tasks()
                .into_iter()
                .find(|task| task.kind == scenario.kind)
                .expect("active task for failed receipt scenario");
            config.with_verification_receipt(
                RustVerificationReceipt::failed(
                    task.fingerprint.clone(),
                    scenario.kind,
                    "benchmark regression exceeded threshold",
                )
                .with_evidence("benchmark_command", "cargo bench --bench parser_hot_path")
                .with_evidence("regression_threshold", "5%")
                .with_evidence("latency_or_throughput", "+11.2% latency"),
            )
        }
        ScenarioResolution::StalePassedReceipt => config.with_verification_receipt(
            RustVerificationReceipt::passed("rustv:stale", scenario.kind),
        ),
        ScenarioResolution::IncompleteWaiver => {
            let task = initial_plan
                .active_tasks()
                .into_iter()
                .find(|task| task.kind == scenario.kind)
                .expect("active task for incomplete waiver scenario");
            config.with_verification_waiver(RustVerificationWaiver::new(
                task.fingerprint.clone(),
                "",
                "",
                "",
            ))
        }
        ScenarioResolution::CompleteWaiver => {
            let task = initial_plan
                .active_tasks()
                .into_iter()
                .find(|task| task.kind == scenario.kind)
                .expect("active task for waiver scenario");
            config.with_verification_waiver(RustVerificationWaiver::new(
                task.fingerprint.clone(),
                "platform",
                "scenario coverage handles this verification slice",
                "2026-06-01",
            ))
        }
    };
    let final_plan = plan_rust_project_verification_with_config(root, &final_config)
        .expect("final scenario plan");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&final_plan), root);
    let contracts = render_rust_verification_skill_contracts(&final_plan);
    let initial_status = scenario_status(&initial_plan, scenario.kind);
    let final_status = scenario_status(&final_plan, scenario.kind);

    assert_scenario_expectations(
        scenario,
        &initial_status,
        &final_status,
        &rendered,
        &contracts,
    );
    render_scenario_audit(
        scenario,
        &initial_status,
        &final_status,
        &rendered,
        &contracts,
    )
}

impl VerificationSkillContractScenario {
    fn initial_config(self) -> AspRustConfig {
        let mut config = default_asp_rust_config()
            .with_verification_skill_binding(self.kind, self.descriptor.binding())
            .with_verification_skill_descriptor(self.descriptor.descriptor());
        if let Some(responsibility) = self.responsibility {
            let mut hint = RustVerificationProfileHint::new("src/api.rs", [responsibility]);
            match self.resolution {
                ScenarioResolution::OwnerOverride => {
                    hint = hint
                        .with_task_kinds([self.kind])
                        .with_rationale("scenario explicitly requests regression verification");
                }
                ScenarioResolution::ProfileSuppressed => {
                    hint = hint
                        .without_verification_tasks()
                        .with_rationale("scenario intentionally suppresses external verification");
                }
                ScenarioResolution::None
                | ScenarioResolution::PassedReceipt
                | ScenarioResolution::StructuredPerformanceReceipt
                | ScenarioResolution::FailedReceipt
                | ScenarioResolution::StalePassedReceipt
                | ScenarioResolution::IncompleteWaiver
                | ScenarioResolution::CompleteWaiver => {}
            }
            config = config.with_verification_profile_hint(hint);
        }
        config
    }
}

fn passed_receipt_for_scenario(
    scenario: &VerificationSkillContractScenario,
    fingerprint: &str,
) -> RustVerificationReceipt {
    let receipt = RustVerificationReceipt::passed(fingerprint.to_string(), scenario.kind);
    if scenario.resolution != ScenarioResolution::StructuredPerformanceReceipt {
        return receipt;
    }
    receipt
        .with_evidence("benchmark_command", "cargo bench --bench parser_hot_path")
        .with_evidence("baseline", "main@b0a8a7a")
        .with_evidence("regression_threshold", "5%")
        .with_evidence("latency_or_throughput", "-1.4% latency")
        .with_evidence("allocation_profile", "allocs/op unchanged")
        .with_evidence(
            "profile_artifact",
            "target/criterion/parser_hot_path/report/index.html",
        )
        .with_evidence_uri("target/criterion/parser_hot_path/report/index.html")
        .with_observed_at("2026-05-01T20:00:00Z")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScenarioStatus {
    active_count: usize,
    task_state: Option<RustVerificationTaskState>,
    contract_ref: Option<String>,
    receipt_evidence: Vec<String>,
}

fn scenario_status(
    plan: &asp_rust::RustVerificationPlan,
    kind: RustVerificationTaskKind,
) -> ScenarioStatus {
    let task = plan.tasks.iter().find(|task| task.kind == kind);
    ScenarioStatus {
        active_count: plan.active_tasks().len(),
        task_state: task.map(|task| task.state),
        contract_ref: task
            .filter(|task| task.is_active())
            .and_then(|task| task.skill_contract_ref.clone()),
        receipt_evidence: task.map_or_else(Vec::new, |task| {
            task.receipt_evidence
                .iter()
                .map(|evidence| format!("{}={}", evidence.label, evidence.value))
                .collect()
        }),
    }
}

fn assert_scenario_expectations(
    scenario: &VerificationSkillContractScenario,
    initial_status: &ScenarioStatus,
    final_status: &ScenarioStatus,
    rendered: &str,
    contracts: &str,
) {
    match scenario.resolution {
        ScenarioResolution::None if scenario.responsibility.is_some() => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 1, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Pending),
                "{}",
                scenario.id
            );
            assert!(
                rendered.contains("contract_ref="),
                "triggered scenario should keep a compact contract ref: {}",
                scenario.id
            );
            assert!(
                contracts.contains("[skill-contract]"),
                "triggered scenario should expose an expandable contract: {}",
                scenario.id
            );
        }
        ScenarioResolution::OwnerOverride => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 1, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Pending),
                "{}",
                scenario.id
            );
            assert!(
                rendered.contains("contract_ref="),
                "owner override should trigger a compact contract ref: {}",
                scenario.id
            );
            assert!(
                contracts.contains("[skill-contract]"),
                "owner override should expose an expandable contract: {}",
                scenario.id
            );
        }
        ScenarioResolution::None => {
            assert_eq!(initial_status.active_count, 0, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
        ScenarioResolution::ProfileSuppressed => {
            assert_eq!(initial_status.active_count, 0, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
        ScenarioResolution::PassedReceipt => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Satisfied),
                "{}",
                scenario.id
            );
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
        ScenarioResolution::StructuredPerformanceReceipt => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Satisfied),
                "{}",
                scenario.id
            );
            assert!(
                final_status
                    .receipt_evidence
                    .iter()
                    .any(|evidence| evidence.starts_with("benchmark_command=")),
                "structured performance receipt should stay searchable: {}",
                scenario.id
            );
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
        ScenarioResolution::FailedReceipt => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 1, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Failed),
                "{}",
                scenario.id
            );
            assert!(rendered.contains("|receipt:"), "{}", scenario.id);
            assert!(
                contracts.contains("[skill-contract]"),
                "failed active receipt should keep contract available: {}",
                scenario.id
            );
        }
        ScenarioResolution::StalePassedReceipt => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 1, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Pending),
                "{}",
                scenario.id
            );
            assert!(rendered.contains("contract_ref="), "{}", scenario.id);
            assert!(
                final_status.receipt_evidence.is_empty(),
                "stale receipt should not attach searchable evidence: {}",
                scenario.id
            );
        }
        ScenarioResolution::IncompleteWaiver => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 1, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Pending),
                "{}",
                scenario.id
            );
            assert!(
                rendered.contains("resolution: stress.waiver=incomplete"),
                "{}",
                scenario.id
            );
        }
        ScenarioResolution::CompleteWaiver => {
            assert_eq!(initial_status.active_count, 1, "{}", scenario.id);
            assert_eq!(final_status.active_count, 0, "{}", scenario.id);
            assert_eq!(
                final_status.task_state,
                Some(RustVerificationTaskState::Waived),
                "{}",
                scenario.id
            );
            assert!(rendered.is_empty(), "{}", scenario.id);
            assert!(contracts.is_empty(), "{}", scenario.id);
        }
    }
}

fn render_scenario_audit(
    scenario: &VerificationSkillContractScenario,
    initial_status: &ScenarioStatus,
    final_status: &ScenarioStatus,
    rendered: &str,
    contracts: &str,
) -> String {
    let mut audit = format!(
        "[scenario] {}\n   |kind: {} resolution={}\n   |initial: active={} state={} contract_ref={}\n   |final: active={} state={} contract_ref={}\n",
        scenario.id,
        scenario.kind.as_str(),
        scenario.resolution.label(),
        initial_status.active_count,
        state_label(initial_status.task_state),
        option_label(initial_status.contract_ref.as_deref()),
        final_status.active_count,
        state_label(final_status.task_state),
        option_label(final_status.contract_ref.as_deref()),
    );
    append_evidence_line(
        &mut audit,
        "initial-receipt",
        &initial_status.receipt_evidence,
    );
    append_evidence_line(&mut audit, "final-receipt", &final_status.receipt_evidence);
    append_block(&mut audit, "verify", rendered);
    append_block(&mut audit, "contracts", contracts);
    audit
}

impl ScenarioResolution {
    const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::OwnerOverride => "owner_override",
            Self::ProfileSuppressed => "profile_suppressed",
            Self::PassedReceipt => "passed_receipt",
            Self::StructuredPerformanceReceipt => "structured_performance_receipt",
            Self::FailedReceipt => "failed_receipt",
            Self::StalePassedReceipt => "stale_passed_receipt",
            Self::IncompleteWaiver => "incomplete_waiver",
            Self::CompleteWaiver => "complete_waiver",
        }
    }
}

fn state_label(state: Option<RustVerificationTaskState>) -> &'static str {
    match state {
        Some(RustVerificationTaskState::Pending) => "pending",
        Some(RustVerificationTaskState::Satisfied) => "satisfied",
        Some(RustVerificationTaskState::Failed) => "failed",
        Some(RustVerificationTaskState::Waived) => "waived",
        None => "-",
    }
}

fn option_label(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}

fn append_evidence_line(audit: &mut String, label: &str, evidence: &[String]) {
    if !evidence.is_empty() {
        audit.push_str(&format!("   |{label}: {}\n", evidence.join(",")));
    }
}

fn append_block(audit: &mut String, label: &str, block: &str) {
    if block.is_empty() {
        return;
    }
    for line in block.lines() {
        audit.push_str(&format!("   |{label}: {line}\n"));
    }
}
