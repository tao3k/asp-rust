use asp_rust::{
    RustOwnerResponsibility, RustVerificationPhase, RustVerificationPlan,
    RustVerificationProfileHint, RustVerificationReceipt, RustVerificationSkillBinding,
    RustVerificationSkillDescriptor, RustVerificationTaskKind, RustVerificationTaskState,
    default_asp_rust_config, plan_rust_project_verification_with_config,
    render_rust_verification_plan, render_rust_verification_plan_json,
    render_rust_verification_skill_contracts,
};
use std::path::PathBuf;
use tempfile::TempDir;

use crate::verification::support::{
    normalize_temp_root, public_api_profile_config, write_api_project,
};

#[test]
fn verification_skill_binding_renders_quiet_dispatch_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config().with_verification_skill_binding(
        RustVerificationTaskKind::Stress,
        RustVerificationSkillBinding::new("rust-verification-stress").with_adapter("k6"),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = plan.active_tasks()[0];
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(task.kind, RustVerificationTaskKind::Stress);
    assert_eq!(
        task.skill_binding.as_ref().expect("skill binding").skill_id,
        "rust-verification-stress"
    );
    assert!(task.skill_contract_ref.is_none());
    assert!(plan.skill_descriptors.is_empty());
    assert_eq!(render_rust_verification_skill_contracts(&plan), "");
    assert!(
        rendered.contains("skill=rust-verification-stress@k6"),
        "{rendered}"
    );
    assert!(!rendered.contains("|why:"), "{rendered}");
    assert!(!rendered.contains("|requires:"), "{rendered}");
    assert!(!rendered.contains("|fact:"), "{rendered}");
    assert!(!rendered.contains("|contract:"), "{rendered}");
    insta::assert_snapshot!(
        "verification_configured_skill_binding_stays_quiet",
        rendered
    );
}

#[test]
fn verification_skill_binding_trigger_audit_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config()
        .with_verification_skill_binding(
            RustVerificationTaskKind::Stress,
            RustVerificationSkillBinding::new("rust-verification-stress").with_adapter("k6"),
        )
        .with_verification_skill_descriptor(RustVerificationSkillDescriptor::k6_stress());

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = plan.active_tasks()[0];
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);
    let contracts = render_rust_verification_skill_contracts(&plan);
    let json = render_rust_verification_plan_json(&plan).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let required_evidence = task
        .required_evidence
        .iter()
        .map(|requirement| requirement.key.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let trigger_evidence = task
        .evidence
        .iter()
        .map(|evidence| format!("{}={}", evidence.label, evidence.value))
        .collect::<Vec<_>>()
        .join(" ");
    let skill_binding = task.skill_binding.as_ref().expect("skill binding");
    let skill_label = skill_binding.adapter.as_ref().map_or_else(
        || skill_binding.skill_id.clone(),
        |adapter| format!("{}@{adapter}", skill_binding.skill_id),
    );
    let phase = value["tasks"][0]["phase"]
        .as_str()
        .expect("serialized phase");

    assert_eq!(
        value["tasks"][0]["skill_binding"]["skill_id"],
        "rust-verification-stress"
    );
    assert_eq!(value["tasks"][0]["evidence"][1]["label"], "skill");
    assert_eq!(
        value["tasks"][0]["evidence"][1]["value"],
        "rust-verification-stress@k6"
    );
    assert_eq!(
        value["tasks"][0]["skill_contract_ref"],
        "rust-verification-stress@k6"
    );
    assert_eq!(plan.skill_descriptors.len(), 1);
    assert_eq!(
        value["skill_descriptors"][0]["skill_id"],
        "rust-verification-stress"
    );
    assert_eq!(value["skill_descriptors"][0]["adapter"], "k6");
    assert_eq!(value["tasks"][0]["required_evidence"][0]["key"], "p50");
    assert!(!rendered.contains("|why:"), "{rendered}");
    assert!(!rendered.contains("|requires:"), "{rendered}");
    assert!(!rendered.contains("|fact:"), "{rendered}");
    assert!(!rendered.contains("|contract:"), "{rendered}");
    let compact_audit = format!(
        "[verification-skill-trigger] src/api.rs\n   |{}: phase={phase} skill={skill_label}\n   |trigger: {trigger_evidence}\n   |requires: {required_evidence}\n   |quiet: omits=why,requires,fact,contract\n\n{rendered}\n{contracts}",
        task.kind.as_str(),
    );
    insta::assert_snapshot!("verification_skill_binding_trigger_audit", compact_audit);
}

#[test]
fn verification_skill_contracts_clear_with_receipt() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config()
        .with_verification_skill_binding(
            RustVerificationTaskKind::Stress,
            RustVerificationSkillBinding::new("rust-verification-stress").with_adapter("k6"),
        )
        .with_verification_skill_descriptor(RustVerificationSkillDescriptor::k6_stress());
    let initial_plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = initial_plan.active_tasks()[0];

    let resolved_config = config.with_verification_receipt(RustVerificationReceipt::passed(
        task.fingerprint.clone(),
        RustVerificationTaskKind::Stress,
    ));
    let resolved_plan =
        plan_rust_project_verification_with_config(root, &resolved_config).expect("resolved plan");
    let stress = resolved_plan
        .tasks
        .iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");

    assert_eq!(stress.state, RustVerificationTaskState::Satisfied);
    assert!(resolved_plan.is_clear());
    assert_eq!(render_rust_verification_plan(&resolved_plan), "");
    assert_eq!(render_rust_verification_skill_contracts(&resolved_plan), "");
    assert!(resolved_plan.report_obligations.is_empty());
    assert!(resolved_plan.skill_descriptors.is_empty());
}

#[test]
fn rust_native_performance_skill_descriptors_snapshot() {
    let plan = RustVerificationPlan {
        project_root: PathBuf::new(),
        tasks: Vec::new(),
        report_obligations: Vec::new(),
        skill_descriptors: vec![
            RustVerificationSkillDescriptor::criterion_performance(),
            RustVerificationSkillDescriptor::divan_performance(),
            RustVerificationSkillDescriptor::iai_callgrind_performance(),
        ],
    };

    insta::assert_snapshot!(
        "rust_native_performance_skill_descriptors",
        render_rust_verification_skill_contracts(&plan)
    );
}

#[test]
fn verification_performance_skill_binding_uses_rust_native_descriptor_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::LatencySensitive],
        ))
        .with_verification_skill_binding(
            RustVerificationTaskKind::Performance,
            RustVerificationSkillBinding::new("rust-verification-performance")
                .with_adapter("criterion"),
        )
        .with_verification_skill_descriptor(
            RustVerificationSkillDescriptor::criterion_performance(),
        );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = plan.active_tasks()[0];
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);
    let contracts = render_rust_verification_skill_contracts(&plan);

    assert_eq!(task.kind, RustVerificationTaskKind::Performance);
    assert_eq!(task.phase, RustVerificationPhase::AfterUnitTestsPass);
    assert_eq!(
        task.skill_contract_ref.as_deref(),
        Some("rust-verification-performance@criterion")
    );
    assert!(!rendered.contains("|why:"), "{rendered}");
    assert!(!rendered.contains("|requires:"), "{rendered}");
    assert!(!rendered.contains("|fact:"), "{rendered}");
    assert!(!rendered.contains("|contract:"), "{rendered}");
    insta::assert_snapshot!(
        "verification_performance_skill_binding_uses_rust_native_descriptor",
        format!("{rendered}\n{contracts}")
    );
}

#[test]
fn verification_performance_convenience_config_uses_library_api() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = default_asp_rust_config()
        .with_criterion_performance_verification()
        .with_latency_sensitive_performance_owner(
            "src/api.rs",
            "API dispatch is a latency-sensitive owner",
        );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = plan.active_tasks()[0];

    assert_eq!(task.kind, RustVerificationTaskKind::Performance);
    assert_eq!(
        task.skill_binding
            .as_ref()
            .and_then(|binding| binding.adapter.as_deref()),
        Some("criterion")
    );
    assert_eq!(
        task.skill_contract_ref.as_deref(),
        Some("rust-verification-performance@criterion")
    );
    assert!(
        task.evidence
            .iter()
            .any(|evidence| evidence.label == "rationale"
                && evidence.value == "API dispatch is a latency-sensitive owner")
    );
    assert!(
        plan.report_obligations
            .iter()
            .any(|obligation| obligation.key == "performance_index_json")
    );
}
