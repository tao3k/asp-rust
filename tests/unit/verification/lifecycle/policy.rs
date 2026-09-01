use asp_rust::{
    RustOwnerResponsibility, RustVerificationPolicy, RustVerificationProfileHint,
    RustVerificationTaskKind, default_asp_rust_config, plan_rust_project_verification_with_config,
    render_rust_verification_plan_json,
};
use tempfile::TempDir;

use crate::verification::support::{public_api_profile_config, write_api_project};

#[test]
fn verification_policy_can_disable_task_kind() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let policy = RustVerificationPolicy::default()
        .with_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::PublicApi],
        ))
        .with_disabled_task_kind(RustVerificationTaskKind::Stress);
    let config = default_asp_rust_config().with_verification_policy(policy);

    let plan = plan_rust_project_verification_with_config(root, &config).expect("plan");

    assert!(plan.tasks.is_empty(), "{plan:?}");
}

#[test]
fn verification_json_preserves_structured_plan_fields() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    let plan = plan_rust_project_verification_with_config(root, &public_api_profile_config())
        .expect("plan");

    let json = render_rust_verification_plan_json(&plan).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert!(value["project_root"].as_str().is_some());
    assert_eq!(
        value["report_obligations"][0]["key"],
        "verification_plan_json"
    );
    assert_eq!(
        value["report_obligations"][0]["renderer"],
        "render_rust_verification_plan_json"
    );
    assert_eq!(
        value["report_obligations"][0]["suggested_artifact_name"],
        "verification_plan.json"
    );
    assert_eq!(value["report_obligations"][0]["task_kinds"][0], "stress");
    assert_eq!(
        value["report_obligations"][0]["task_fingerprints"][0],
        value["tasks"][0]["fingerprint"]
    );
    assert_eq!(value["tasks"][0]["kind"], "stress");
    assert_eq!(value["tasks"][0]["state"], "pending");
    assert_eq!(value["tasks"][0]["required_evidence"][0]["key"], "p50");
    assert_eq!(
        value["tasks"][0]["required_evidence"][4]["key"],
        "sla_result"
    );
    assert_eq!(value["tasks"][0]["owner_namespace"][0], "src");
    assert_eq!(value["tasks"][0]["owner_namespace"][1], "api");
    assert!(value["tasks"][0]["resolution_notes"].is_null());
}
