use asp_rust::{
    RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID, RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION,
    RustVerificationReportManifestCompatibility, RustVerificationReportManifestSchema,
    check_rust_verification_report_manifest_schema,
    render_rust_verification_report_manifest_compatibility,
    render_rust_verification_report_manifest_schema_compatibility,
};

#[test]
fn manifest_schema_compatibility_accepts_current_schema() {
    let schema = RustVerificationReportManifestSchema::default();

    let compatibility = check_rust_verification_report_manifest_schema(&schema);

    assert!(compatibility.is_supported());
    assert_eq!(compatibility.as_str(), "supported");
    assert_eq!(compatibility.action(), "read_manifest_payloads");
    assert_eq!(compatibility.reason(), None);
    assert_eq!(
        render_rust_verification_report_manifest_compatibility(&compatibility),
        "[verify-report-manifest] state=supported action=read_manifest_payloads"
    );
    assert_eq!(
        render_rust_verification_report_manifest_schema_compatibility(&schema),
        "[verify-report-manifest] state=supported schema=rust_verification_report_manifest/1 action=read_manifest_payloads"
    );
}

#[test]
fn manifest_schema_compatibility_rejects_unknown_schema_id() {
    let schema = RustVerificationReportManifestSchema {
        schema_id: "other_manifest".to_string(),
        schema_version: RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION.to_string(),
    };

    let compatibility = schema.compatibility();

    assert_eq!(
        compatibility,
        RustVerificationReportManifestCompatibility::UnsupportedSchemaId {
            expected: RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID.to_string(),
            actual: "other_manifest".to_string(),
        }
    );
    assert!(!compatibility.is_supported());
    assert_eq!(compatibility.as_str(), "unsupported_schema_id");
    assert_eq!(
        compatibility.reason(),
        Some("unsupported manifest schema id")
    );
    assert_eq!(
        render_rust_verification_report_manifest_schema_compatibility(&schema),
        "[verify-report-manifest] state=unsupported_schema_id expected=rust_verification_report_manifest actual=other_manifest reason=\"unsupported manifest schema id\" action=stop_and_refresh_harness_contract"
    );
}

#[test]
fn manifest_schema_compatibility_rejects_unknown_schema_version() {
    let schema = RustVerificationReportManifestSchema {
        schema_id: RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID.to_string(),
        schema_version: "2".to_string(),
    };

    let compatibility = schema.compatibility();

    assert_eq!(
        compatibility,
        RustVerificationReportManifestCompatibility::UnsupportedSchemaVersion {
            expected: RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION.to_string(),
            actual: "2".to_string(),
        }
    );
    assert!(!compatibility.is_supported());
    assert_eq!(compatibility.as_str(), "unsupported_schema_version");
    assert_eq!(
        compatibility.reason(),
        Some("unsupported manifest schema version")
    );
    assert_eq!(
        render_rust_verification_report_manifest_schema_compatibility(&schema),
        "[verify-report-manifest] state=unsupported_schema_version expected=1 actual=2 reason=\"unsupported manifest schema version\" action=stop_and_refresh_harness_contract"
    );
}

#[test]
fn manifest_schema_compatibility_json_is_agent_stable() {
    let compatibility = RustVerificationReportManifestCompatibility::UnsupportedSchemaVersion {
        expected: "1".to_string(),
        actual: "2".to_string(),
    };

    let json = serde_json::to_string(&compatibility).expect("json");

    assert_eq!(
        json,
        r#"{"state":"unsupported_schema_version","expected":"1","actual":"2"}"#
    );
}
