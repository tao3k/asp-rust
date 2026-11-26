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

#[test]
fn ast_inspect_and_verify_operate_over_syn() {
    let source = "/// Keep doc comment.\npub fn demo(opt: Option<i32>) -> Option<i32> {\n    opt.map(|v| v + 1)\n}\n";
    let inspect_payload = serde_json::json!({
        "path": "src/lib.rs",
        "sourceUtf8": source,
        "sourceHash": format!("blake3-256:{}", blake3::hash(source.as_bytes()).to_hex()),
    });
    let inspected: serde_json::Value = serde_json::from_slice(
        &crate::provider_server::ast_ops::handle_ast_inspect_value(&inspect_payload)
            .expect("ast-inspect"),
    )
    .expect("decode inspect");
    assert_eq!(inspected["accepted"], true);
    assert_eq!(inspected["nodes"][0]["locator"], "src/lib.rs#fn:demo");
    assert!(
        inspected["nodes"][0]["innerSpan"]["start"]
            .as_u64()
            .expect("inner")
            > inspected["nodes"][0]["outerSpan"]["start"]
                .as_u64()
                .expect("outer")
    );

    let verify_payload = serde_json::json!({
        "path": "src/lib.rs",
        "candidates": [
            { "candidateId": "broken", "sourceUtf8": "pub fn x() { let y = 1;" },
            { "candidateId": "valid", "sourceUtf8": "pub fn x() { let y = 1; }" }
        ]
    });
    let verified: serde_json::Value = serde_json::from_slice(
        &crate::provider_server::ast_ops::handle_ast_verify_value(&verify_payload)
            .expect("ast-verify"),
    )
    .expect("decode verify");
    assert_eq!(verified["results"][0]["accepted"], false);
    assert_eq!(verified["results"][1]["accepted"], true);
}
