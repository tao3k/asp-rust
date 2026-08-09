//! Unit role: exact-source snapshot envelope coverage and relocation.

use super::{
    ExactSelector, ExactSourceQuery, ExactSourceSnapshotEnvelopeV1, ExactSourceSnapshotEvidenceV1,
    ExactSourceSnapshotOwnerV1, PinnedSource, PinnedWorkspace, RelocationOutcome,
    collect_parse_artifact_items, exact_source_merkle_root_depth, relocate_live_item,
    run_exact_source_query, validate_exact_source_envelope_coverage,
};

use super::{ExactSourceFailure, exact_source_failure_packet, owner_live_item_diagnostics};

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn owner(path: &str) -> ExactSourceSnapshotOwnerV1 {
    ExactSourceSnapshotOwnerV1 {
        path: path.to_owned(),
        snapshot_leaf_digest: digest('1'),
        blob_digest: digest('2'),
        source_content_digest: digest('3'),
        cas_path: digest('2'),
        parser_artifact_digest: None,
    }
}

fn envelope(
    leaf_count: usize,
    owners: Vec<ExactSourceSnapshotOwnerV1>,
) -> ExactSourceSnapshotEnvelopeV1 {
    let root_digest =
        crate::provider_workspace_search_identity::WorkspaceSnapshot::from_file_hashes(
            owners
                .iter()
                .map(|owner| (owner.path.as_str(), owner.snapshot_leaf_digest.as_str())),
        )
        .root_digest()
        .to_owned();
    ExactSourceSnapshotEnvelopeV1 {
        schema_id: "asp.exact-source-snapshot-envelope.v1".to_owned(),
        schema_version: "1".to_owned(),
        provider_id: "rust-lang-project-harness".to_owned(),
        source_snapshot: ExactSourceSnapshotEvidenceV1 {
            schema_id: "asp.source-snapshot.v1".to_owned(),
            algorithm: "blake3-merkle-v1".to_owned(),
            root_digest,
            leaf_count,
            provider_digest: digest('5'),
        },
        root_depth: exact_source_merkle_root_depth(leaf_count),
        materialization_state: "artifact-complete".to_owned(),
        owner_coverage: "complete".to_owned(),
        cas_root: std::path::PathBuf::from("/cas"),
        owners,
    }
}

#[test]
fn complete_envelope_coverage_is_accepted() {
    let envelope = envelope(2, vec![owner("src/a.rs"), owner("src/b.rs")]);
    assert_eq!(validate_exact_source_envelope_coverage(&envelope), Ok(()));
}

#[test]
fn source_content_identity_does_not_replace_workspace_generation_identity() {
    let mut envelope = envelope(2, vec![owner("src/a.rs"), owner("src/b.rs")]);
    envelope.owners[0].source_content_digest = digest('9');
    assert_eq!(validate_exact_source_envelope_coverage(&envelope), Ok(()));

    envelope.owners[0].snapshot_leaf_digest = digest('8');
    let error = validate_exact_source_envelope_coverage(&envelope).unwrap_err();
    assert!(error.contains("source snapshot provider generation mismatch"));
}

#[test]
fn partial_envelope_coverage_is_rejected() {
    let envelope = envelope(2, vec![owner("src/a.rs")]);
    let error = validate_exact_source_envelope_coverage(&envelope).unwrap_err();
    assert!(error.contains("leafCount=2 ownerCount=1"));
}

#[test]
fn self_consistent_partial_metadata_cannot_impersonate_a_generation() {
    let mut envelope = envelope(1, vec![owner("src/a.rs")]);
    envelope.source_snapshot.root_digest = digest('4');
    let error = validate_exact_source_envelope_coverage(&envelope).unwrap_err();
    assert!(error.contains("source snapshot provider generation mismatch"));
    assert!(error.contains("expectedRootDigest="));
    assert!(error.contains("actualRootDigest="));
}

#[test]
fn root_depth_and_materialization_drift_are_rejected() {
    let mut envelope = envelope(2, vec![owner("src/a.rs"), owner("src/b.rs")]);
    envelope.root_depth = 0;
    envelope.materialization_state = "resident-memory".to_owned();
    let error = validate_exact_source_envelope_coverage(&envelope).unwrap_err();
    assert!(error.contains("expectedRootDepth=1"));
    assert!(error.contains("materializationState=resident-memory"));
}

#[test]
fn moved_owner_relocates_from_complete_workspace_generation() {
    let source = "pub fn install_archive_binary() -> &'static str { \"ok\" }\n".to_owned();
    let syntax = crate::parser::parse_rust_source_syntax(&source).unwrap();
    let mut items = Vec::new();
    collect_parse_artifact_items(&source, &syntax.items, &mut items);
    let workspace = PinnedWorkspace {
        provider_id: "rust-lang-project-harness".to_owned(),
        root_digest: digest('4'),
        exact_projection_authority: None,
        sources: std::collections::BTreeMap::from([(
            "src/command/install_provider_archive.rs".to_owned(),
            PinnedSource {
                source,
                blob_digest: digest('2'),
                parser_artifact_digest: None,
                parse_error: None,
                items,
            },
        )]),
    };
    let stale_selector = ExactSelector {
        owner_path: "src/command/install_provider.rs".to_owned(),
        item_kind: "function".to_owned(),
        item_name: "install_archive_binary".to_owned(),
        scopes: Vec::new(),
        segment: None,
    };

    let RelocationOutcome::Resolved(resolved) =
        relocate_live_item(&workspace, &stale_selector).unwrap()
    else {
        panic!("moved owner did not resolve uniquely");
    };
    assert_eq!(
        resolved.owner_path,
        "src/command/install_provider_archive.rs"
    );
    assert!(resolved.code.contains("install_archive_binary"));
}

#[test]
fn missing_item_candidates_prefer_the_requested_kind() {
    let source = "pub mod alpha {}\npub fn beta() {}\npub use alpha as gamma;\n".to_owned();
    let syntax = crate::parser::parse_rust_source_syntax(&source).unwrap();
    let mut items = Vec::new();
    collect_parse_artifact_items(&source, &syntax.items, &mut items);
    let pinned_source = PinnedSource {
        source,
        blob_digest: digest('2'),
        parser_artifact_digest: None,
        parse_error: None,
        items,
    };
    let selector = ExactSelector {
        owner_path: "src/lib.rs".to_owned(),
        item_kind: "function".to_owned(),
        item_name: "missing".to_owned(),
        scopes: Vec::new(),
        segment: None,
    };

    let (candidates, actual_kinds) =
        owner_live_item_diagnostics(&pinned_source, "src/lib.rs", &selector);

    assert_eq!(
        candidates,
        vec!["rust://src/lib.rs#item/function/beta".to_owned()]
    );
    assert!(actual_kinds.is_empty());
}

#[test]
fn missing_item_json_is_a_typed_projection_result() {
    let selector = ExactSelector {
        owner_path: "src/lib.rs".to_owned(),
        item_kind: "function".to_owned(),
        item_name: "missing".to_owned(),
        scopes: Vec::new(),
        segment: None,
    };
    let pinned = PinnedWorkspace {
        provider_id: "rs-harness".to_owned(),
        root_digest: digest('4'),
        exact_projection_authority: None,
        sources: std::collections::BTreeMap::from([(
            "src/lib.rs".to_owned(),
            PinnedSource {
                source: String::new(),
                blob_digest: digest('2'),
                parser_artifact_digest: None,
                parse_error: None,
                items: Vec::new(),
            },
        )]),
    };
    let failure = ExactSourceFailure {
        selector: &selector,
        pinned: &pinned,
        requested_structural_selector: "rust://src/lib.rs#item/function/missing",
        state: "item-missing",
        reason_kind: "item-not-in-live-owner",
        candidates: vec!["rust://src/lib.rs#item/function/beta".to_owned()],
        actual_kinds: Vec::new(),
        json: true,
    };

    let packet = exact_source_failure_packet(&failure);

    assert_eq!(
        packet["schemaId"],
        "agent.semantic-protocols.provider-native-exact-projection"
    );
    assert_eq!(packet["resolutionState"], "item-missing");
    assert_eq!(packet["reasonKind"], "item-not-in-live-owner");
    assert_eq!(
        packet["candidates"],
        serde_json::json!(["rust://src/lib.rs#item/function/beta"])
    );
    assert!(
        packet.get("recommendedNext").is_none(),
        "an item miss in a live owner is terminal active-generation evidence"
    );
}

#[test]
fn missing_owner_is_generation_bound_selector_stale() {
    let selector = ExactSelector {
        owner_path: "src/removed.rs".to_owned(),
        item_kind: "struct".to_owned(),
        item_name: "ClientHookConfig".to_owned(),
        scopes: Vec::new(),
        segment: None,
    };
    let pinned = PinnedWorkspace {
        provider_id: "rs-harness".to_owned(),
        root_digest: digest('4'),
        exact_projection_authority: None,
        sources: std::collections::BTreeMap::new(),
    };
    let failure = ExactSourceFailure {
        selector: &selector,
        pinned: &pinned,
        requested_structural_selector: "rust://src/removed.rs#item/struct/ClientHookConfig",
        state: "selector-stale",
        reason_kind: "selector-not-in-active-generation",
        candidates: Vec::new(),
        actual_kinds: Vec::new(),
        json: true,
    };

    let packet = exact_source_failure_packet(&failure);

    assert_eq!(packet["resolutionState"], "selector-stale");
    assert_eq!(packet["reasonKind"], "selector-not-in-active-generation");
    assert_eq!(
        packet["activeGenerationDigest"],
        format!("blake3-256:{}", digest('4'))
    );
    assert_eq!(
        packet["recommendedNext"]["command"],
        "asp rust search lexical --query 'ClientHookConfig' --query 'struct ClientHookConfig' --workspace . --view seeds"
    );
}

#[test]
fn invalid_selector_is_rejected_before_snapshot_requirement() {
    let error = run_exact_source_query(ExactSourceQuery {
        projection: "source".to_owned(),
        selector: "rust://src/lib.rs#syntax:identifier/declaration.name@1:1".to_owned(),
        source_snapshot_envelope: None,
        exact_request_stdin: false,
        json: false,
        provider_id: None,
    })
    .unwrap_err();
    assert!(error.contains("state=invalid-selector"), "{error}");
    assert!(!error.contains("wrapper-snapshot-required"), "{error}");
}
