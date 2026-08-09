//! Unit role: canonical structural-selector scope codec.

use rust_lang_project_harness::structural_selector::{
    encode_canonical_item_identity_path, parse_canonical_item_selector,
};

#[test]
fn round_trips_conditional_compilation_scope() {
    let selector = "rust://src/lib.rs#item/function/run_search_view/scope/conditional-compilation/cfg/not%20%28feature%20%3D%20%22json%22%29";
    let parsed = parse_canonical_item_selector(selector).expect("parse selector");

    assert_eq!(parsed.identity().scopes.len(), 1);
    assert_eq!(
        parsed.identity().scopes[0].symbol.as_str(),
        r#"not (feature = "json")"#
    );
    assert_eq!(
        encode_canonical_item_identity_path(&parsed.identity()),
        selector.split_once("#").expect("selector fragment").1
    );
}

#[test]
fn round_trips_implementation_owner_scope() {
    let selector = "rust://src/lib.rs#item/method/parse/scope/implementation-owner/type/CliOptions";
    let parsed = parse_canonical_item_selector(selector).expect("parse selector");

    assert_eq!(parsed.identity().scopes.len(), 1);
    assert_eq!(
        parsed.identity().scopes[0].relation.as_str(),
        "implementation-owner"
    );
    assert_eq!(
        encode_canonical_item_identity_path(&parsed.identity()),
        selector.split_once("#").expect("selector fragment").1
    );
}
