//! Verification task planner derived from parser reasoning-tree facts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::model::AspRustConfig;
use crate::parser::{
    RustReasoningImportFacts, RustReasoningModuleFacts, RustReasoningOwnerBranchFacts,
    RustReasoningOwnerBranchRole,
};

use super::analysis::{
    RustVerificationCargoDependencyAnalysis, RustVerificationPackageAnalysis,
    RustVerificationProjectAnalysis, analyze_rust_verification_project,
};
use super::api_path::{collect_api_path_baseline_tasks, collect_unmatched_api_path_baselines};
use super::module_lookup::RustVerificationModuleLookup;
use super::profile::{
    hint_rationale_is_empty, profile_evidence, profile_task_reason, responsibility_labels,
    task_contract_for_profile, task_kind_labels, task_kinds_for_profile,
    task_kinds_for_responsibilities,
};
use super::report_options::STABILITY_PICTURE_ARTIFACT_KEY;
use super::task_builder::{
    ProfileReviewTaskSpec, VerificationTaskSpec, new_profile_review_task, new_skill_task,
    push_task, skill_descriptors_for_tasks,
};
use super::{
    RustOwnerResponsibility, RustVerificationEvidence, RustVerificationPlan,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationReportObligation,
    RustVerificationTask, RustVerificationTaskKind,
};

struct ProfileConfigReviewTaskSpec<'a> {
    module: &'a RustReasoningModuleFacts,
    hint: &'a RustVerificationProfileHint,
    reason: &'static str,
    evidence: Vec<RustVerificationEvidence>,
}

struct VerificationTaskCollections<'a> {
    matched_profile_hints: &'a mut BTreeSet<usize>,
    matched_api_path_baselines: &'a mut BTreeSet<usize>,
    tasks: &'a mut BTreeMap<String, RustVerificationTask>,
}

#[derive(Default)]
struct ReportObligationFacts {
    task_kinds: BTreeSet<RustVerificationTaskKind>,
    task_fingerprints: Vec<String>,
    configured_skill_task_kinds: BTreeSet<RustVerificationTaskKind>,
    configured_skill_task_fingerprints: Vec<String>,
    performance_fingerprints: Vec<String>,
    stability_fingerprints: Vec<String>,
}

impl ReportObligationFacts {
    fn from_tasks(tasks: &[RustVerificationTask]) -> Self {
        tasks
            .iter()
            .filter(|task| task.is_active())
            .fold(Self::default(), |mut facts, task| {
                facts.record_task(task);
                facts
            })
    }

    fn record_task(&mut self, task: &RustVerificationTask) {
        self.task_kinds.insert(task.kind);
        self.task_fingerprints.push(task.fingerprint.clone());
        if task.skill_binding.is_some() {
            self.configured_skill_task_kinds.insert(task.kind);
            self.configured_skill_task_fingerprints
                .push(task.fingerprint.clone());
        }
        if task.kind == RustVerificationTaskKind::Performance {
            self.performance_fingerprints.push(task.fingerprint.clone());
        }
        if task.kind == RustVerificationTaskKind::Stability {
            self.stability_fingerprints.push(task.fingerprint.clone());
        }
    }
}

/// Plan parser-native verification tasks for a conventional Rust project.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn plan_rust_project_verification(project_root: &Path) -> Result<RustVerificationPlan, String> {
    plan_rust_project_verification_with_config(project_root, &AspRustConfig::default())
}

/// Plan parser-native verification tasks with the verification policy embedded
/// in the harness config.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn plan_rust_project_verification_with_config(
    project_root: &Path,
    config: &AspRustConfig,
) -> Result<RustVerificationPlan, String> {
    plan_rust_project_verification_with_policy(project_root, config, &config.verification_policy)
}

/// Plan parser-native verification tasks with an explicit verification policy.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn plan_rust_project_verification_with_policy(
    project_root: &Path,
    config: &AspRustConfig,
    policy: &RustVerificationPolicy,
) -> Result<RustVerificationPlan, String> {
    let analysis = analyze_rust_verification_project(
        project_root,
        config,
        RustVerificationCargoDependencyAnalysis::Skip,
    )?;
    Ok(plan_rust_verification_from_analysis(
        project_root,
        policy,
        analysis,
    ))
}

/// Plan verification tasks from an existing parser-owned harness analysis.
#[must_use]
pub fn plan_rust_verification_from_harness_analysis(
    analysis: crate::runner::AspRustAnalysis,
    policy: &RustVerificationPolicy,
) -> RustVerificationPlan {
    let project_root = analysis.project_root.clone();
    let verification_analysis = super::analysis::analyze_rust_verification_from_harness_analysis(
        analysis,
        RustVerificationCargoDependencyAnalysis::Skip,
    );
    plan_rust_verification_from_analysis(&project_root, policy, verification_analysis)
}

fn plan_rust_verification_from_analysis(
    project_root: &Path,
    policy: &RustVerificationPolicy,
    analysis: RustVerificationProjectAnalysis,
) -> RustVerificationPlan {
    let mut tasks = BTreeMap::new();
    let mut matched_profile_hints = BTreeSet::new();
    let mut matched_api_path_baselines = BTreeSet::new();
    {
        let mut collections = VerificationTaskCollections {
            matched_profile_hints: &mut matched_profile_hints,
            matched_api_path_baselines: &mut matched_api_path_baselines,
            tasks: &mut tasks,
        };
        for package_analysis in &analysis.package_analyses {
            collect_package_verification_tasks(
                project_root,
                package_analysis,
                policy,
                &mut collections,
            );
        }
    }
    collect_unmatched_profile_hints(project_root, policy, &matched_profile_hints, &mut tasks);
    collect_unmatched_api_path_baselines(
        project_root,
        policy,
        &matched_api_path_baselines,
        &mut tasks,
    );
    let mut task_values = tasks.into_values().collect::<Vec<_>>();
    let mut plan = RustVerificationPlan {
        project_root: project_root.to_path_buf(),
        skill_descriptors: skill_descriptors_for_tasks(policy, &task_values),
        report_obligations: report_obligations_for_tasks(&task_values),
        tasks: std::mem::take(&mut task_values),
    };
    plan.tasks.sort_by(|left, right| {
        left.package_root
            .cmp(&right.package_root)
            .then_with(|| left.owner_path.cmp(&right.owner_path))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.fingerprint.cmp(&right.fingerprint))
    });
    plan
}

fn collect_package_verification_tasks(
    project_root: &Path,
    package_analysis: &RustVerificationPackageAnalysis,
    policy: &RustVerificationPolicy,
    collections: &mut VerificationTaskCollections<'_>,
) {
    let reasoning_tree = &package_analysis.reasoning_tree;
    let module_lookup = RustVerificationModuleLookup::new(
        project_root,
        &reasoning_tree.package_root,
        reasoning_tree
            .modules
            .iter()
            .filter(|module| module.is_source_module),
    );
    collect_profile_tasks(
        project_root,
        &reasoning_tree.package_root,
        &module_lookup,
        policy,
        &mut *collections.matched_profile_hints,
        &mut *collections.tasks,
    );
    collect_api_path_baseline_tasks(
        project_root,
        &reasoning_tree.package_root,
        &module_lookup,
        policy,
        &mut *collections.matched_api_path_baselines,
        &mut *collections.tasks,
    );
    collect_regression_tasks(
        project_root,
        &reasoning_tree.package_root,
        &reasoning_tree.owner_branches,
        policy,
        &mut *collections.tasks,
    );
}

fn report_obligations_for_tasks(
    tasks: &[RustVerificationTask],
) -> Vec<RustVerificationReportObligation> {
    let ReportObligationFacts {
        task_kinds,
        task_fingerprints,
        configured_skill_task_kinds,
        configured_skill_task_fingerprints,
        performance_fingerprints,
        stability_fingerprints,
    } = ReportObligationFacts::from_tasks(tasks);
    if task_fingerprints.is_empty() {
        return Vec::new();
    }

    let mut obligations = vec![RustVerificationReportObligation::new(
        "verification_plan_json",
        "render_rust_verification_plan_json",
        "verification_plan.json",
        "persist active verification policy state so receipts, waivers, and task drift stay comparable",
        task_kinds,
        task_fingerprints,
    )];
    if !configured_skill_task_fingerprints.is_empty() {
        obligations.push(RustVerificationReportObligation::new(
            "task_index_json",
            "build_rust_verification_task_index + render_rust_verification_task_index_json",
            "task_index.json",
            "persist compact configured-skill task state for security, performance, stress, chaos, and regression",
            configured_skill_task_kinds,
            configured_skill_task_fingerprints,
        ));
    }

    if !performance_fingerprints.is_empty() {
        obligations.push(RustVerificationReportObligation::new(
            "performance_index_json",
            "build_rust_verification_performance_index + render_rust_verification_performance_index_json",
            "performance_index.json",
            "persist Rust performance state for benchmark, receipt, and missing-evidence metrics",
            [RustVerificationTaskKind::Performance],
            performance_fingerprints,
        ));
    }

    if !stability_fingerprints.is_empty() {
        obligations.push(RustVerificationReportObligation::new(
            "stability_index_json",
            "build_rust_verification_stability_index + render_rust_verification_stability_index_json",
            "stability_index.json",
            "persist Rust stability state for long-run drift, resource growth, and missing-evidence metrics",
            [RustVerificationTaskKind::Stability],
            stability_fingerprints,
        ));
        obligations.push(RustVerificationReportObligation::new(
            STABILITY_PICTURE_ARTIFACT_KEY,
            "build_rust_verification_stability_picture_with_policy + render_rust_verification_stability_picture_json",
            "stability_picture.json",
            "persist Agent-facing stability action picture with project and owner-local configuration",
            [RustVerificationTaskKind::Stability],
            ReportObligationFacts::from_tasks(tasks).stability_fingerprints,
        ));
    }

    obligations
}

fn collect_profile_tasks(
    project_root: &Path,
    package_root: &Path,
    module_lookup: &RustVerificationModuleLookup<'_>,
    policy: &RustVerificationPolicy,
    matched_profile_hints: &mut BTreeSet<usize>,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    for (hint_index, hint) in policy.profile_hints.iter().enumerate() {
        let Some(module) = module_lookup.get_config_path(&hint.owner_path) else {
            continue;
        };
        matched_profile_hints.insert(hint_index);
        collect_profile_conflict_task(project_root, package_root, module, hint, policy, tasks);
        collect_profile_config_review_tasks(
            project_root,
            package_root,
            module,
            hint,
            policy,
            tasks,
        );
        collect_skill_tasks_from_profile(project_root, package_root, module, hint, policy, tasks);
    }
}

fn collect_unmatched_profile_hints(
    project_root: &Path,
    policy: &RustVerificationPolicy,
    matched_profile_hints: &BTreeSet<usize>,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    for (hint_index, hint) in policy.profile_hints.iter().enumerate() {
        if matched_profile_hints.contains(&hint_index) {
            continue;
        }
        let owner_path = if hint.owner_path.is_absolute() {
            hint.owner_path.clone()
        } else {
            project_root.join(&hint.owner_path)
        };
        push_task(
            tasks,
            policy,
            new_profile_review_task(
                project_root,
                project_root,
                ProfileReviewTaskSpec {
                    owner_path,
                    owner_namespace: Vec::new(),
                    reason: "profile hint target is not a parser-known Rust source module",
                    evidence: vec![RustVerificationEvidence::new(
                        "hint",
                        format!(
                            "responsibilities={}",
                            responsibility_labels(&hint.responsibilities)
                        ),
                    )],
                    hint: Some(hint),
                },
                policy,
            ),
        );
    }
}

fn collect_profile_conflict_task(
    project_root: &Path,
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    if !hint
        .responsibilities
        .contains(&RustOwnerResponsibility::PureDomainLogic)
    {
        return;
    }
    let non_test_owner_deps = non_test_owner_dependency_count(&module.import_summary);
    if module.import_summary.external_imports == 0 && non_test_owner_deps == 0 {
        return;
    }
    let task = new_profile_review_task(
        project_root,
        package_root,
        ProfileReviewTaskSpec {
            owner_path: module.path.clone(),
            owner_namespace: module.source_path.namespace_components.clone(),
            reason: "profile declares pure domain logic but parser facts show runtime or owner dependencies",
            evidence: vec![
                RustVerificationEvidence::new(
                    "profile",
                    responsibility_labels(&hint.responsibilities),
                ),
                RustVerificationEvidence::new(
                    "parser",
                    format!(
                        "external_imports={} owner_deps={non_test_owner_deps}",
                        module.import_summary.external_imports
                    ),
                ),
            ],
            hint: Some(hint),
        },
        policy,
    );
    push_task(tasks, policy, task);
}

fn collect_profile_config_review_tasks(
    project_root: &Path,
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    if hint.responsibilities.is_empty() {
        push_profile_config_review_task(
            project_root,
            package_root,
            policy,
            tasks,
            ProfileConfigReviewTaskSpec {
                module,
                hint,
                reason: "profile hint declares no responsibilities for owner",
                evidence: vec![RustVerificationEvidence::new(
                    "profile",
                    "responsibilities=<none>",
                )],
            },
        );
    }

    let effective_task_kinds = task_kinds_for_profile(hint, policy);
    if let Some(owner_task_kinds) = &hint.task_kinds {
        let derived_task_kinds = task_kinds_for_responsibilities(&hint.responsibilities, policy);
        if owner_task_kinds != &derived_task_kinds && hint_rationale_is_empty(hint) {
            push_profile_config_review_task(
                project_root,
                package_root,
                policy,
                tasks,
                ProfileConfigReviewTaskSpec {
                    module,
                    hint,
                    reason: "owner-local verification override needs compact rationale",
                    evidence: vec![
                        RustVerificationEvidence::new(
                            "profile",
                            responsibility_labels(&hint.responsibilities),
                        ),
                        RustVerificationEvidence::new(
                            "derived",
                            task_kind_labels(&derived_task_kinds),
                        ),
                        RustVerificationEvidence::new(
                            "configured",
                            task_kind_labels(owner_task_kinds),
                        ),
                    ],
                },
            );
        }

        let disabled_task_kinds = owner_task_kinds
            .intersection(&policy.disabled_task_kinds)
            .copied()
            .collect::<BTreeSet<_>>();
        if !disabled_task_kinds.is_empty() {
            push_profile_config_review_task(
                project_root,
                package_root,
                policy,
                tasks,
                ProfileConfigReviewTaskSpec {
                    module,
                    hint,
                    reason: "owner-local verification override references disabled task kind",
                    evidence: vec![
                        RustVerificationEvidence::new(
                            "configured",
                            task_kind_labels(owner_task_kinds),
                        ),
                        RustVerificationEvidence::new(
                            "disabled",
                            task_kind_labels(&disabled_task_kinds),
                        ),
                    ],
                },
            );
        }
    }

    let unused_contract_kinds = hint
        .task_contract_overrides
        .keys()
        .filter(|kind| !effective_task_kinds.contains(kind))
        .copied()
        .collect::<BTreeSet<_>>();
    if !unused_contract_kinds.is_empty() {
        push_profile_config_review_task(
            project_root,
            package_root,
            policy,
            tasks,
            ProfileConfigReviewTaskSpec {
                module,
                hint,
                reason: "owner-local task contract is not used by effective task kinds",
                evidence: vec![
                    RustVerificationEvidence::new(
                        "effective",
                        task_kind_labels(&effective_task_kinds),
                    ),
                    RustVerificationEvidence::new(
                        "unused_contracts",
                        task_kind_labels(&unused_contract_kinds),
                    ),
                ],
            },
        );
    }
}

fn collect_skill_tasks_from_profile(
    project_root: &Path,
    package_root: &Path,
    module: &RustReasoningModuleFacts,
    hint: &RustVerificationProfileHint,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    let responsibilities = &hint.responsibilities;
    let uses_owner_task_override = hint.task_kinds.is_some();
    for kind in task_kinds_for_profile(hint, policy) {
        push_task(
            tasks,
            policy,
            new_skill_task(
                project_root,
                package_root,
                VerificationTaskSpec {
                    kind,
                    owner_path: module.path.clone(),
                    owner_namespace: module.source_path.namespace_components.clone(),
                    line: None,
                    reason: profile_task_reason(kind, responsibilities, uses_owner_task_override),
                    contract: task_contract_for_profile(policy, Some(hint), kind),
                    evidence: profile_evidence(hint),
                },
                policy,
            ),
        );
    }
}

fn collect_regression_tasks(
    project_root: &Path,
    package_root: &Path,
    branches: &[RustReasoningOwnerBranchFacts],
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
) {
    for branch in branches {
        if branch.roles.contains(&RustReasoningOwnerBranchRole::Root) {
            continue;
        }
        let child_count = branch.declared_child_edges.len();
        let dependency_count = non_test_owner_dependency_count(&branch.import_summary);
        if child_count < 3 && dependency_count < 3 {
            continue;
        }
        push_task(
            tasks,
            policy,
            new_skill_task(
                project_root,
                package_root,
                VerificationTaskSpec {
                    kind: RustVerificationTaskKind::Regression,
                    owner_path: branch.path.clone(),
                    owner_namespace: branch.owner_namespace.clone(),
                    line: None,
                    reason: "parser facts show a branch coordinating several child modules or local owners"
                        .to_string(),
                    contract: task_contract_for_profile(
                        policy,
                        None,
                        RustVerificationTaskKind::Regression,
                    ),
                    evidence: vec![
                        RustVerificationEvidence::new("child_modules", child_count.to_string()),
                        RustVerificationEvidence::new("owner_deps", dependency_count.to_string()),
                    ],
                },
                policy,
            ),
        );
    }
}

fn push_profile_config_review_task(
    project_root: &Path,
    package_root: &Path,
    policy: &RustVerificationPolicy,
    tasks: &mut BTreeMap<String, RustVerificationTask>,
    spec: ProfileConfigReviewTaskSpec<'_>,
) {
    push_task(
        tasks,
        policy,
        new_profile_review_task(
            project_root,
            package_root,
            ProfileReviewTaskSpec {
                owner_path: spec.module.path.clone(),
                owner_namespace: spec.module.source_path.namespace_components.clone(),
                reason: spec.reason,
                evidence: spec.evidence,
                hint: Some(spec.hint),
            },
            policy,
        ),
    );
}

fn non_test_owner_dependency_count(imports: &RustReasoningImportFacts) -> usize {
    imports
        .local_owner_dependencies
        .iter()
        .filter(|dependency| !dependency.is_test_context)
        .count()
}
