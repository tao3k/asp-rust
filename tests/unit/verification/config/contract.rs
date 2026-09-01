use asp_rust::{
    RustOwnerResponsibility, RustVerificationPhase, RustVerificationProfileHint,
    RustVerificationRequirement, RustVerificationTaskContract, RustVerificationTaskKind,
    default_asp_rust_config, plan_rust_project_verification_with_config,
    render_rust_verification_plan,
};
use tempfile::TempDir;

use crate::verification::support::{
    normalize_temp_root, public_api_profile_config, write_api_project,
};

#[test]
fn verification_task_contract_is_configurable() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let config = public_api_profile_config().with_verification_task_contract(
        RustVerificationTaskKind::Stress,
        RustVerificationTaskContract::new(
            RustVerificationPhase::BeforeRelease,
            "custom stress skill must report tenant SLO and saturation step",
            [
                RustVerificationRequirement::new("tenant_slo", "tenant-specific SLO result"),
                RustVerificationRequirement::new(
                    "saturation_step",
                    "first pressure step that saturates the owner",
                ),
            ],
        ),
    );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let stress = plan
        .active_tasks()
        .into_iter()
        .find(|task| task.kind == RustVerificationTaskKind::Stress)
        .expect("stress task");
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(stress.phase, RustVerificationPhase::BeforeRelease);
    assert_eq!(stress.required_evidence[0].key, "tenant_slo");
    assert_eq!(stress.required_evidence[1].key, "saturation_step");
    assert!(
        rendered.contains("contract: stress=custom stress skill must report tenant SLO"),
        "{rendered}"
    );
    insta::assert_snapshot!("verification_custom_stress_contract", rendered);
}

#[test]
fn verification_profile_task_contract_beats_global_contract() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let global_contract = RustVerificationTaskContract::new(
        RustVerificationPhase::BeforeRelease,
        "global security skill must report broad authz result",
        [RustVerificationRequirement::new(
            "global_authz",
            "global authorization result",
        )],
    );
    let owner_contract = RustVerificationTaskContract::new(
        RustVerificationPhase::AfterUnitTestsPass,
        "owner security skill must report route-level authz matrix",
        [RustVerificationRequirement::new(
            "route_authz_matrix",
            "route-level authorization matrix",
        )],
    );
    let config = default_asp_rust_config()
        .with_verification_task_contract(RustVerificationTaskKind::Security, global_contract)
        .with_verification_profile_hint(
            RustVerificationProfileHint::new("src/api.rs", [RustOwnerResponsibility::PublicApi])
                .with_task_kinds([RustVerificationTaskKind::Security])
                .with_task_contract(RustVerificationTaskKind::Security, owner_contract)
                .with_rationale("owner route authorization differs from global default"),
        );

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");
    let task = plan.active_tasks()[0];
    let rendered = normalize_temp_root(&render_rust_verification_plan(&plan), root);

    assert_eq!(task.phase, RustVerificationPhase::AfterUnitTestsPass);
    assert_eq!(task.required_evidence[0].key, "route_authz_matrix");
    assert!(
        rendered.contains("contract: security=owner security skill must report route-level authz"),
        "{rendered}"
    );
    assert!(!rendered.contains("global security skill"), "{rendered}");
    insta::assert_snapshot!("verification_profile_contract_beats_global", rendered);
}
