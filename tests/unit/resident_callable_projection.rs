use super::{
    ExactProjectionAuthority, ResolvedExactItem, callable_skeleton_projection_from_owner_syntax,
};

#[test]
fn resident_callable_projection_binds_v1_root_and_descendants() {
    let source = "pub fn selected(value: u64) -> u64 { if value > 1 { return value; } 0 }";
    let parsed = syn::parse_file(source).expect("parse owner");
    let mut items = Vec::new();
    crate::exact_source_parse_artifact::collect_parse_artifact_items(
        source,
        &parsed.items,
        &mut items,
    );
    let item = items.first().expect("callable item");
    let selector = "rust://src/lib.rs#item/function/selected";
    let resolved = ResolvedExactItem {
        canonical_selector: crate::content_identity::CanonicalItemSelector::new(
            item.identity.clone(),
            selector,
        ),
        owner_path: "src/lib.rs".into(),
        identity: item.identity.clone(),
        code: source[item.source_byte_start..item.source_byte_end].into(),
        source_byte_start: item.source_byte_start,
        source_byte_end: item.source_byte_end,
        owner_blob_digest: "a".repeat(64),
        parser_artifact_digest: None,
    };
    let authority = ExactProjectionAuthority {
        generation_identity_digest: "b".repeat(64),
        parser_identity_digest: "c".repeat(64),
        query_pack_digest: "d".repeat(64),
    };
    let payload = callable_skeleton_projection_from_owner_syntax(
        &resolved,
        &authority,
        item.callable_syntax.as_ref().unwrap(),
        source,
    )
    .expect("resident callable projection");
    assert_eq!(payload["rootSelector"], selector);
    let nodes = payload["nodes"].as_array().expect("nodes");
    let root = nodes
        .iter()
        .find(|node| node["nodeId"] == payload["rootNodeId"])
        .expect("root node exists");
    assert_eq!(root["selector"], selector);
    assert!(nodes.iter().any(|node| node["kind"] == "branch"));
    for node in nodes {
        let exact = node["selector"].as_str().expect("queryable selector");
        assert!(exact == selector || exact.starts_with(&format!("{selector}/segment/")));
    }
    assert_eq!(
        payload["cost"]["projectedBytes"].as_u64().unwrap(),
        serde_json::to_vec(&payload).unwrap().len() as u64
    );
}
