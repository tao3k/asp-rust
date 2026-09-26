use super::{admit_provider_server_argv, encode_ready_response_frame};
use std::ffi::OsString;

#[test]
fn provider_process_surface_admits_only_catalog_declared_serve() {
    assert!(admit_provider_server_argv([OsString::from("serve")]).is_ok());
    assert!(admit_provider_server_argv([]).is_err());
    assert!(
        admit_provider_server_argv([OsString::from("serve"), OsString::from("unexpected")])
            .is_err()
    );
}

#[test]
fn ready_frame_embeds_provider_json_without_a_value_round_trip() {
    let payload =
        br#"{"owner":"src/lib.rs","items":[{"selector":"rust://src/lib.rs#function/ready"}]}"#;
    let encoded = encode_ready_response_frame("request-\"quoted", payload)
        .expect("encode ready provider frame");
    let frame: crate::provider_server::contract::ProviderRuntimeResponseFrame =
        serde_json::from_slice(&encoded).expect("decode ready provider frame");

    assert_eq!(frame.request_id, "request-\"quoted");
    assert_eq!(
        frame.outcome,
        crate::provider_server::contract::ProviderRuntimeResponseOutcome::Ready
    );
    assert_eq!(frame.payload.expect("payload")["owner"], "src/lib.rs");
}
