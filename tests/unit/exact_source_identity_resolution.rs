use super::{
    ExactProjectionAuthority, ExactSelector, ExactSourceFailure, ParseArtifactItem, PinnedSource,
    PinnedWorkspace, RelocationOutcome, collect_parse_artifact_items, exact_source_failure_packet,
    relocate_live_item, resolve_live_item,
};

fn identity(selector: &str) -> crate::canonical_item_identity::CanonicalItemIdentityV1 {
    crate::structural_selector::parse_canonical_item_selector(selector)
        .expect("parse canonical selector fixture")
        .identity()
}

fn item(selector: &str) -> ParseArtifactItem {
    ParseArtifactItem {
        identity: identity(selector),
        source_byte_start: 0,
        source_byte_end: 0,
    }
}

fn source(items: Vec<ParseArtifactItem>) -> PinnedSource {
    PinnedSource {
        source: String::new(),
        blob_digest: "blake3-256:fixture".to_string(),
        parser_artifact_digest: Some("blake3-256:parser-fixture".to_string()),
        parse_error: None,
        items,
    }
}

fn workspace(owners: &[(&str, Vec<ParseArtifactItem>)]) -> PinnedWorkspace {
    PinnedWorkspace {
        provider_id: "asp-rust".to_string(),
        root_digest: "blake3-256:workspace-fixture".to_string(),
        exact_projection_authority: None,
        sources: owners
            .iter()
            .map(|(owner, items)| ((*owner).to_string(), source(items.clone())))
            .collect(),
    }
}

fn selector(owner: &str, canonical: &str) -> ExactSelector {
    let identity = identity(canonical);
    ExactSelector {
        owner_path: owner.to_string(),
        item_kind: identity.kind.as_str().to_string(),
        item_name: identity.symbol.as_str().to_string(),
        scopes: identity.scopes,
        segment: None,
    }
}

#[test]
fn owner_kind_validation_does_not_consume_unrelated_workspace_symbols() {
    let workspace = workspace(&[
        ("src/requested.rs", Vec::new()),
        (
            "src/unrelated.rs",
            vec![item("rust://src/unrelated.rs#item/struct/dispatch")],
        ),
    ]);
    let selector = selector(
        "src/requested.rs",
        "rust://src/requested.rs#item/function/dispatch",
    );

    assert_eq!(
        resolve_live_item(&workspace, &selector).expect("resolve owner"),
        RelocationOutcome::Missing
    );
    assert_eq!(
        relocate_live_item(&workspace, &selector).expect("relocate owner"),
        RelocationOutcome::Missing
    );
}

#[test]
fn true_owner_local_kind_mismatch_fails_closed() {
    let workspace = workspace(&[(
        "src/lib.rs",
        vec![item("rust://src/lib.rs#item/struct/dispatch")],
    )]);
    let selector = selector("src/lib.rs", "rust://src/lib.rs#item/function/dispatch");

    assert_eq!(
        resolve_live_item(&workspace, &selector).expect("resolve owner"),
        RelocationOutcome::KindMismatch(vec!["struct".to_string()])
    );
}

#[test]
fn method_without_impl_owner_is_identity_incomplete() {
    let workspace = workspace(&[(
        "src/cli.rs",
        vec![item(
            "rust://src/cli.rs#item/method/parse/scope/implementation-owner/type/CliOptions",
        )],
    )]);
    let selector = selector("src/cli.rs", "rust://src/cli.rs#item/method/parse");

    assert!(matches!(
        resolve_live_item(&workspace, &selector).expect("resolve owner"),
        RelocationOutcome::IdentityIncomplete(candidates)
            if candidates == vec![
                "rust://src/cli.rs#item/method/parse/scope/implementation-owner/type/CliOptions"
                    .to_string()
            ]
    ));
}

#[test]
fn impl_without_owner_prefers_identity_incomplete_over_same_name_struct() {
    let scoped_impl =
        "rust://src/cli.rs#item/impl/CliOptions/scope/implementation-owner/type/CliOptions";
    let workspace = workspace(&[(
        "src/cli.rs",
        vec![
            item("rust://src/cli.rs#item/struct/CliOptions"),
            item(scoped_impl),
        ],
    )]);
    let selector = selector("src/cli.rs", "rust://src/cli.rs#item/impl/CliOptions");

    assert_eq!(
        resolve_live_item(&workspace, &selector).expect("resolve owner"),
        RelocationOutcome::IdentityIncomplete(vec![scoped_impl.to_string()])
    );
}

#[test]
fn impl_owner_disambiguates_repeated_method_symbols() {
    let cli_parse =
        "rust://src/cli.rs#item/method/parse/scope/implementation-owner/type/CliOptions";
    let request_parse =
        "rust://src/cli.rs#item/method/parse/scope/implementation-owner/type/Request";
    let workspace = workspace(&[("src/cli.rs", vec![item(cli_parse), item(request_parse)])]);
    let selector = selector("src/cli.rs", cli_parse);

    assert!(matches!(
        resolve_live_item(&workspace, &selector).expect("resolve owner"),
        RelocationOutcome::Resolved(resolved)
            if resolved.canonical_selector.structural_selector == cli_parse
    ));
}

#[test]
fn parser_producer_and_exact_resolver_round_trip_impl_trait_and_methods() {
    let owner = "src/identity_fixture.rs";
    let source_text = r#"
struct CliOptions;

trait Parse {
    fn parse(&self);
}

impl CliOptions {
    fn parse(&self) {}
}

impl Parse for CliOptions {
    fn parse(&self) {}
}
"#;
    let items = crate::exact_source_parse_artifact::parse_owner_items_v1(source_text)
        .expect("parse canonical identity fixture");
    let workspace = PinnedWorkspace {
        provider_id: "asp-rust".to_string(),
        root_digest: "blake3-256:producer-consumer-round-trip".to_string(),
        exact_projection_authority: None,
        sources: [(
            owner.to_string(),
            PinnedSource {
                source: source_text.to_string(),
                blob_digest: "blake3-256:identity-fixture".to_string(),
                parser_artifact_digest: Some("blake3-256:parser-identity-fixture".to_string()),
                parse_error: None,
                items: items.clone(),
            },
        )]
        .into_iter()
        .collect(),
    };

    let parse_methods = items
        .iter()
        .filter(|item| {
            item.identity.kind.as_str() == "method" && item.identity.symbol.as_str() == "parse"
        })
        .collect::<Vec<_>>();
    assert_eq!(parse_methods.len(), 2);
    assert_ne!(
        parse_methods[0].identity.scopes,
        parse_methods[1].identity.scopes
    );
    assert!(parse_methods.iter().all(|item| {
        item.identity.scopes.iter().any(|scope| {
            scope.relation.as_str() == "implementation-owner"
                && scope.symbol.as_str() == "CliOptions"
        })
    }));
    assert!(parse_methods.iter().any(|item| {
        item.identity.scopes.iter().any(|scope| {
            scope.relation.as_str() == "trait-owner" && scope.symbol.as_str() == "Parse"
        })
    }));

    for item in &items {
        let canonical = format!(
            "rust://{owner}#{}",
            crate::structural_selector::encode_canonical_item_identity_path(&item.identity)
        );
        let exact_selector = selector(owner, &canonical);
        assert!(matches!(
            resolve_live_item(&workspace, &exact_selector).expect("resolve parser-produced identity"),
            RelocationOutcome::Resolved(resolved)
                if resolved.canonical_selector.structural_selector == canonical
        ));
    }
}

#[test]
fn trait_and_inherent_method_identities_do_not_alias() {
    let inherent = "rust://src/cli.rs#item/method/parse/scope/implementation-owner/type/CliOptions";
    let trait_method = "rust://src/cli.rs#item/method/parse/scope/implementation-owner/type/CliOptions/scope/trait-owner/trait/Parse";
    let workspace = workspace(&[("src/cli.rs", vec![item(inherent), item(trait_method)])]);
    let selector = selector("src/cli.rs", trait_method);

    assert!(matches!(
        resolve_live_item(&workspace, &selector).expect("resolve owner"),
        RelocationOutcome::Resolved(resolved)
            if resolved.canonical_selector.structural_selector == trait_method
    ));
}

#[test]
fn cfg_scope_is_part_of_exact_identity() {
    let cfg_item = "rust://src/lib.rs#item/function/run/scope/conditional-compilation/cfg/test";
    let workspace = workspace(&[("src/lib.rs", vec![item(cfg_item)])]);
    let selector = selector("src/lib.rs", cfg_item);

    assert!(matches!(
        resolve_live_item(&workspace, &selector).expect("resolve owner"),
        RelocationOutcome::Resolved(_)
    ));
}

#[test]
fn cfg_module_is_collected_and_resolved_from_live_owner_bytes() {
    let parsed = syn::parse_file("#[cfg(test)] mod tests { fn fixture() {} }")
        .expect("parse cfg module fixture");
    let mut items = Vec::new();
    collect_parse_artifact_items("", &parsed.items, &mut items);
    let workspace = workspace(&[("src/lib.rs", items)]);
    let selector = selector(
        "src/lib.rs",
        "rust://src/lib.rs#item/module/tests/scope/conditional-compilation/cfg/test",
    );

    assert!(matches!(
        resolve_live_item(&workspace, &selector).expect("resolve cfg module"),
        RelocationOutcome::Resolved(_)
    ));
}

#[test]
fn complete_owner_generation_preserves_true_item_missing() {
    let workspace = workspace(&[(
        "src/lib.rs",
        vec![item("rust://src/lib.rs#item/function/present")],
    )]);
    let selector = selector("src/lib.rs", "rust://src/lib.rs#item/module/tests");

    assert_eq!(
        resolve_live_item(&workspace, &selector).expect("resolve absent module"),
        RelocationOutcome::Missing
    );
}

#[test]
fn missing_owner_is_not_misclassified_as_a_stale_selector() {
    assert_eq!(
        super::missing_resolution_classification(false),
        ("owner-missing", "owner-not-in-workspace")
    );
    assert_eq!(
        super::missing_resolution_classification(true),
        ("item-missing", "item-not-in-live-owner")
    );
}

#[test]
fn owner_missing_packet_carries_generation_evidence_and_lexical_recovery() {
    let selector = selector(
        "src/moved.rs",
        "rust://src/moved.rs#item/function/inventory",
    );
    let pinned = PinnedWorkspace {
        provider_id: "asp-rust".to_string(),
        root_digest: "b".repeat(64),
        exact_projection_authority: Some(ExactProjectionAuthority {
            projection_kind: "source".to_string(),
            generation_identity_digest: "a".repeat(64),
            parser_identity_digest: "c".repeat(64),
            query_pack_digest: "d".repeat(64),
        }),
        sources: std::collections::BTreeMap::new(),
    };
    let packet = exact_source_failure_packet(&ExactSourceFailure {
        selector: &selector,
        pinned: &pinned,
        requested_structural_selector: "rust://src/moved.rs#item/function/inventory",
        state: "owner-missing",
        reason_kind: "owner-not-in-workspace",
        candidates: Vec::new(),
        actual_kinds: Vec::new(),
        json: true,
    });

    assert_eq!(packet["resolutionState"], "owner-missing");
    assert_eq!(packet["reasonKind"], "owner-not-in-workspace");
    assert_eq!(
        packet["activeGenerationDigest"],
        format!("blake3-256:{}", "a".repeat(64))
    );
    assert_eq!(packet["rootDigest"], "b".repeat(64));
    assert!(
        packet["recommendedNext"]["command"]
            .as_str()
            .expect("recommended command")
            .contains("search lexical")
    );
}

#[test]
fn legacy_kind_aliases_are_rejected_at_validation() {
    let workspace = workspace(&[("src/lib.rs", vec![item("rust://src/lib.rs#item/fn/run")])]);
    let selector = selector("src/lib.rs", "rust://src/lib.rs#item/function/run");

    assert!(matches!(
        resolve_live_item(&workspace, &selector).expect("resolve owner"),
        RelocationOutcome::KindMismatch(actual) if actual == vec!["fn".to_owned()]
    ));
}

#[test]
fn relocation_requires_the_complete_canonical_identity() {
    let requested =
        "rust://src/old.rs#item/method/parse/scope/implementation-owner/type/CliOptions";
    let moved = "rust://src/new.rs#item/method/parse/scope/implementation-owner/type/CliOptions";
    let unrelated = "rust://src/other.rs#item/method/parse/scope/implementation-owner/type/Request";
    let workspace = workspace(&[
        ("src/old.rs", Vec::new()),
        ("src/new.rs", vec![item(moved)]),
        ("src/other.rs", vec![item(unrelated)]),
    ]);
    let selector = selector("src/old.rs", requested);

    assert!(matches!(
        relocate_live_item(&workspace, &selector).expect("relocate owner"),
        RelocationOutcome::Resolved(resolved)
            if resolved.canonical_selector.structural_selector == moved
    ));
}
