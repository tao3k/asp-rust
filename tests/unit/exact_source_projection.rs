use crate::exact_source::{
    ExactSelector, PinnedSource, collect_parse_artifact_items, resolved_exact_item,
};
use super::provider_native_exact_projection_packet;

#[test]
fn provider_native_exact_projection_carries_exact_source_byte_range() {
    let source =
        "mod outer {\n    pub fn selected(value: u64) -> u64 {\n        value + 1\n    }\n}\n";
    let parsed = syn::parse_file(source).expect("parse fixture");
    let mut items = Vec::new();
    collect_parse_artifact_items(source, &parsed.items, &mut items);
    let selected = items
        .iter()
        .find(|item| item.identity.symbol.as_str() == "selected")
        .expect("nested function projection")
        .clone();
    let pinned = PinnedSource {
        source: source.to_string(),
        blob_digest: "fixture-blob".to_string(),
        parser_artifact_digest: None,
        parse_error: None,
        items,
    };
    let resolved = resolved_exact_item("src/lib.rs", &pinned, &selected);
    let requested_selector = "rust://src/lib.rs#item/function/selected";
    let authority = super::super::ExactProjectionAuthority {
        projection_kind: "source".to_string(),
        generation_identity_digest: "a".repeat(64),
        parser_identity_digest: "b".repeat(64),
        query_pack_digest: "c".repeat(64),
    };
    let packet = provider_native_exact_projection_packet(
        "rs-harness",
        requested_selector,
        &resolved,
        "live-hit",
        Some(&authority),
    )
    .expect("source projection");

    let source_byte_start = packet["sourceByteStart"].as_u64().expect("sourceByteStart") as usize;
    let source_byte_end = packet["sourceByteEnd"].as_u64().expect("sourceByteEnd") as usize;
    assert!(source_byte_start < source_byte_end);
    assert_eq!(
        packet["projectionText"].as_str(),
        source.get(source_byte_start..source_byte_end),
    );
    assert_eq!(
        packet["normalizedParserFacts"]["itemName"].as_str(),
        Some("selected"),
    );
}

#[test]
fn reexport_projection_carries_the_complete_use_item_byte_range() {
    let source = "pub use crate::model::{First, Second as Renamed};\n";
    let parsed = syn::parse_file(source).expect("parse fixture");
    let mut items = Vec::new();
    collect_parse_artifact_items(source, &parsed.items, &mut items);

    for item in &items {
        let pinned = PinnedSource {
            source: source.to_string(),
            blob_digest: "fixture-blob".to_string(),
            parser_artifact_digest: None,
            parse_error: None,
            items: items.clone(),
        };
        let resolved = resolved_exact_item("src/lib.rs", &pinned, item);
        let requested_selector = format!(
            "rust://src/lib.rs#item/{}/{}",
            item.identity.kind.as_str(),
            item.identity.symbol.as_str()
        );
        let authority = super::super::ExactProjectionAuthority {
            projection_kind: "source".to_string(),
            generation_identity_digest: "a".repeat(64),
            parser_identity_digest: "b".repeat(64),
            query_pack_digest: "c".repeat(64),
        };
        let packet = provider_native_exact_projection_packet(
            "rs-harness",
            &requested_selector,
            &resolved,
            "live-hit",
            Some(&authority),
        )
        .expect("source projection");
        let start = packet["sourceByteStart"].as_u64().expect("start") as usize;
        let end = packet["sourceByteEnd"].as_u64().expect("end") as usize;
        assert_eq!(packet["projectionText"].as_str(), source.get(start..end));
        assert_eq!(source.get(start..end), Some(source.trim_end()));
    }
}

#[test]
fn callable_skeleton_projection_returns_queryable_parser_nodes() {
    let source = r#"pub fn selected(values: &[u64]) -> u64 {
    let mut total = 0;
    let _length = values.len();
    let _optional = Some(total)?;
    for value in values {
        if *value > 1 {
            total += value;
        }
    }
    total
}
"#;
    let parsed = syn::parse_file(source).expect("parse fixture");
    let mut items = Vec::new();
    collect_parse_artifact_items(source, &parsed.items, &mut items);
    let selected = items
        .iter()
        .find(|item| item.identity.symbol.as_str() == "selected")
        .expect("function projection")
        .clone();
    let pinned = PinnedSource {
        source: source.to_string(),
        blob_digest: blake3::hash(source.as_bytes()).to_hex().to_string(),
        parser_artifact_digest: None,
        parse_error: None,
        items,
    };
    let resolved = resolved_exact_item("src/lib.rs", &pinned, &selected);
    let authority = super::super::ExactProjectionAuthority {
        projection_kind: "callable-skeleton".to_string(),
        generation_identity_digest: "a".repeat(64),
        parser_identity_digest: "b".repeat(64),
        query_pack_digest: "c".repeat(64),
    };
    let packet = provider_native_exact_projection_packet(
        "rs-harness",
        "rust://src/lib.rs#item/function/selected",
        &resolved,
        "live-hit",
        Some(&authority),
    )
    .expect("callable skeleton projection");

    assert_eq!(packet["schemaVersion"], "1");
    assert_eq!(packet["projectionMode"], "callable-skeleton");
    assert!(packet.get("projectionText").is_none());
    let payload = &packet["projectionPayload"];
    assert_eq!(payload["schemaVersion"], "1");
    assert_eq!(payload["projectionKind"], "callable-skeleton");
    assert_eq!(payload["rootNodeId"], "callable:root");
    let nodes = payload["nodes"].as_array().expect("skeleton nodes");
    assert!(nodes.len() >= 4);
    assert!(nodes.iter().all(|node| node["queryable"] == true));
    assert!(nodes.iter().all(|node| {
        node["exactSelector"]["generationIdentityDigest"] == "a".repeat(64)
            && node["exactSelector"]["parserIdentityDigest"] == "b".repeat(64)
            && node["exactSelector"]["queryPackDigest"] == "c".repeat(64)
    }));
    assert!(nodes.iter().any(|node| node["kind"] == "binding"));
    assert!(nodes.iter().any(|node| node["kind"] == "loop"));
    assert!(nodes.iter().any(|node| node["kind"] == "branch"));
    assert!(!nodes.iter().any(|node| node["kind"] == "invocation"));
    assert!(!nodes.iter().any(|node| node["kind"] == "exception"));

    let branch_selector = nodes
        .iter()
        .find(|node| node["kind"] == "branch")
        .and_then(|node| node["exactSelector"]["selector"].as_str())
        .expect("queryable branch selector");
    let parsed_selector = ExactSelector::parse(branch_selector).expect("parse branch selector");
    let materialized = crate::cli::runner::dispatch::exact_source::exact_source_projection::resolve_callable_segment(
        &resolved,
        parsed_selector.segment.as_ref().expect("branch segment"),
    )
    .expect("materialize branch segment");
    assert!(materialized.code.starts_with("if "));
    assert_eq!(
        materialized.canonical_selector.structural_selector,
        branch_selector
    );
}
