use std::process::ExitCode;

use crate::cli::ExactSourceQuery;

use super::model::ParseArtifactItem;
use super::parse_artifact;

pub(in super::super) fn run_exact_source_query(
    options: ExactSourceQuery,
) -> Result<ExitCode, String> {
    let source_snapshot_envelope = options.source_snapshot_envelope.as_ref().ok_or_else(|| {
        "exact source query state=source-unavailable reasonKind=wrapper-snapshot-required"
            .to_string()
    })?;
    let selector = ExactSelector::parse(&options.selector)
        .map_err(|error| format!("exact source query state=invalid-selector {error}"))?;
    let pinned = PinnedWorkspace::load(source_snapshot_envelope).map_err(|error| {
        format!(
            "exact source query state=source-unavailable reasonKind=snapshot-envelope-invalid {error}"
        )
    })?;
    let requested_owner_exists = pinned.sources.contains_key(&selector.owner_path);
    let (resolved, state) = if let Some(resolved) = resolve_live_item(&pinned, &selector)? {
        (resolved, "live-hit")
    } else {
        match relocate_live_item(&pinned, &selector)? {
            RelocationOutcome::Resolved(resolved) => (*resolved, "live-relocated"),
            RelocationOutcome::Ambiguous(candidates) => {
                return exact_source_failure(
                    &selector,
                    &pinned,
                    "ambiguous",
                    "multiple-snapshot-items",
                    candidates,
                    Vec::new(),
                );
            }
            RelocationOutcome::KindMismatch(actual_kinds) => {
                return exact_source_failure(
                    &selector,
                    &pinned,
                    "kind-mismatch",
                    "snapshot-item-kind-mismatch",
                    Vec::new(),
                    actual_kinds,
                );
            }
            RelocationOutcome::Missing => {
                let state = if requested_owner_exists {
                    "item-missing"
                } else {
                    "owner-missing"
                };
                let reason_kind = if requested_owner_exists {
                    "item-not-in-live-owner"
                } else {
                    "owner-not-in-snapshot"
                };
                let (candidates, actual_kinds) = if requested_owner_exists {
                    pinned
                        .sources
                        .get(&selector.owner_path)
                        .map(|source| {
                            owner_live_item_diagnostics(source, &selector.owner_path, &selector)
                        })
                        .unwrap_or_else(|| (Vec::new(), Vec::new()))
                } else {
                    (Vec::new(), Vec::new())
                };
                return exact_source_failure(
                    &selector,
                    &pinned,
                    state,
                    reason_kind,
                    candidates,
                    actual_kinds,
                );
            }
        }
    };
    let code = resolved.code.trim_end_matches('\n').to_string();

    if options.json {
        let provider_id = options
            .provider_id
            .as_deref()
            .ok_or_else(|| "exact source projection requires --asp-provider-id".to_string())?;
        if provider_id != pinned.provider_id {
            return Err(format!(
                "exact source projection provider mismatch: expected={} actual={provider_id}",
                pinned.provider_id
            ));
        }
        let parser_identity_digest =
            crate::semantic_identity::exact_selector_merkle::parse_content_digest_v1(
                options.parser_identity_digest.as_deref().ok_or_else(|| {
                    "exact source projection requires --asp-parser-identity-digest".to_string()
                })?,
            )?;
        let query_pack_digest =
            crate::semantic_identity::exact_selector_merkle::parse_content_digest_v1(
                options.query_pack_digest.as_deref().ok_or_else(|| {
                    "exact source projection requires --asp-query-pack-digest".to_string()
                })?,
            )?;
        let source = pinned.sources.get(&resolved.owner_path).ok_or_else(|| {
            format!(
                "exact source projection resolved an owner outside the pinned snapshot: {}",
                resolved.owner_path
            )
        })?;
        let normalized_parser_facts = serde_json::to_vec(&serde_json::json!({
            "itemKind": resolved.canonical_selector.kind,
            "itemName": resolved.canonical_selector.symbol,
            "ownerPath": resolved.owner_path,
            "resolvedSelector": resolved.canonical_selector.structural_selector,
            "resolutionState": state,
        }))
        .map_err(|error| format!("serialize exact source parser facts: {error}"))?;
        let (projection_mode, projection_payload) = if options.names_only {
            (
                crate::semantic_identity::exact_selector_merkle::ExactProjectionModeV1::Names,
                resolved.canonical_selector.symbol.as_str().as_bytes(),
            )
        } else {
            (
                crate::semantic_identity::exact_selector_merkle::ExactProjectionModeV1::Code,
                code.as_bytes(),
            )
        };
        let packet_language_id =
            crate::semantic_identity::exact_selector_projection_packet::ProjectionPacketLanguageIdV1::from(
                "rust",
            );
        let packet_provider_id =
            crate::semantic_identity::exact_selector_projection_packet::ProjectionPacketProviderIdV1::from(
                provider_id,
            );
        let packet_owner_path =
            crate::semantic_identity::exact_selector_projection_packet::ProjectionPacketOwnerPathV1::from(
                resolved.owner_path.as_str(),
            );
        let packet_structural_selector =
            crate::semantic_identity::exact_selector_projection_packet::ProjectionPacketStructuralSelectorV1::from(
                options.selector.as_str(),
            );
        let packet = crate::semantic_identity::exact_selector_projection_packet::build_exact_selector_projection_packet_v1(
            crate::semantic_identity::exact_selector_projection_packet::ExactSelectorProjectionPacketV1Input {
                language_id: &packet_language_id,
                provider_id: &packet_provider_id,
                canonical_item_selector: resolved.canonical_selector.clone(),
                parser_identity_digest: &parser_identity_digest,
                query_pack_digest: &query_pack_digest,
                owner_path: &packet_owner_path,
                structural_selector: &packet_structural_selector,
                projection_mode,
                source: source.source.as_bytes(),
                normalized_parser_facts: &normalized_parser_facts,
                projection: projection_payload,
            },
        );
        println!(
            "{}",
            serde_json::to_string(&packet)
                .map_err(|error| format!("serialize exact source projection packet: {error}"))?
        );
    } else if options.names_only {
        println!("{}", resolved.canonical_selector.symbol.as_str());
    } else if options.code {
        println!("{code}");
    } else {
        println!("{}", resolved.canonical_selector.structural_selector);
    }

    Ok(ExitCode::SUCCESS)
}

fn parse_exact_source_selector(
    selector: &str,
) -> Result<
    (
        &str,
        crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1,
    ),
    String,
> {
    let selector = selector
        .strip_prefix("rust://")
        .ok_or_else(|| format!("exact source selector `{selector}` must start with rust://"))?;
    let (owner_path, item_selector) = selector
        .split_once('#')
        .ok_or_else(|| format!("exact source selector `{selector}` must include #item/"))?;
    if owner_path.is_empty() || item_selector.is_empty() {
        return Err(format!("exact source selector `{selector}` is incomplete"));
    }
    let language_id =
        crate::semantic_identity::structural_selector::StructuralSelectorLanguageId::from("rust");
    let identity_path =
        crate::semantic_identity::structural_selector::CanonicalItemIdentityPath::from(
            item_selector,
        );
    let identity =
        crate::semantic_identity::structural_selector::decode_canonical_item_identity_path(
            &language_id,
            &identity_path,
        )
        .map_err(|error| format!("exact source selector `{selector}` is invalid: {error}"))?;
    Ok((owner_path, identity))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExactSelector {
    owner_path: String,
    item_kind: String,
    item_name: String,
    scopes: Vec<crate::semantic_identity::canonical_item_identity::CanonicalItemScopeV1>,
}

impl ExactSelector {
    fn parse(selector: &str) -> Result<Self, String> {
        let (owner_path, identity) = parse_exact_source_selector(selector)?;
        let owner = std::path::Path::new(owner_path);
        if owner.is_absolute()
            || owner.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            })
        {
            return Err(format!(
                "exact source selector `{selector}` escapes workspace"
            ));
        }
        let owner_path = owner_path.replace('\\', "/");
        if owner_path
            .split('/')
            .any(|segment| segment.is_empty() || segment == ".")
        {
            return Err(format!(
                "exact source selector `{selector}` has a non-canonical owner path"
            ));
        }
        Ok(Self {
            owner_path,
            item_kind: exact_source_canonical_rust_item_kind(identity.kind.as_str()).to_string(),
            item_name: identity.symbol.as_str().to_string(),
            scopes: identity.scopes,
        })
    }
}

#[cfg(test)]
fn collect_parse_artifact_items(
    _source: &str,
    items: &[syn::Item],
    output: &mut Vec<ParseArtifactItem>,
) {
    output.extend(parse_artifact::collect_parse_artifact_items(items));
}

#[derive(Clone, Debug)]
struct PinnedSource {
    source: String,
    blob_digest: String,
    parser_artifact_digest: Option<String>,
    parse_error: Option<String>,
    items: Vec<ParseArtifactItem>,
}

#[derive(Clone, Debug)]
struct PinnedWorkspace {
    provider_id: String,
    root_digest: String,
    sources: std::collections::BTreeMap<String, PinnedSource>,
}

fn snapshot_digest_is_valid(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl PinnedWorkspace {
    fn load(envelope_path: &std::path::Path) -> Result<Self, String> {
        let envelope = std::fs::read(envelope_path).map_err(|error| {
            format!(
                "failed to read source snapshot envelope {}: {error}",
                envelope_path.display()
            )
        })?;
        let envelope: ExactSourceSnapshotEnvelopeV1 =
            serde_json::from_slice(&envelope).map_err(|error| {
                format!(
                    "failed to decode source snapshot envelope {}: {error}",
                    envelope_path.display()
                )
            })?;
        if envelope.schema_id != "asp.exact-source-snapshot-envelope.v1"
            || envelope.schema_version != "1"
        {
            return Err(format!(
                "unsupported source snapshot envelope schemaId={} schemaVersion={}",
                envelope.schema_id, envelope.schema_version
            ));
        }
        if envelope.provider_id.is_empty()
            || envelope.source_snapshot.schema_id != "asp.source-snapshot.v1"
            || envelope.source_snapshot.root_digest.len() != 64
            || envelope
                .source_snapshot
                .root_digest
                .chars()
                .any(|character| !character.is_ascii_hexdigit())
            || envelope.source_snapshot.algorithm != "blake3-merkle-v1"
            || envelope.source_snapshot.provider_digest.len() != 64
            || envelope
                .source_snapshot
                .provider_digest
                .chars()
                .any(|character| !character.is_ascii_hexdigit())
        {
            return Err(
                "source snapshot envelope lacks complete v1 authority evidence".to_string(),
            );
        }
        let mut sources = std::collections::BTreeMap::new();
        for owner in envelope.owners {
            let relative_path = normalize_snapshot_owner_path(&owner.path)?;
            if !snapshot_digest_is_valid(owner.snapshot_leaf_digest.as_str())
                || !snapshot_digest_is_valid(owner.blob_digest.as_str())
            {
                return Err(format!(
                    "source snapshot owner {} has an invalid blob digest",
                    owner.path
                ));
            }
            let cas_path = normalize_snapshot_owner_path(&owner.cas_path)?;
            let source_path = envelope.cas_root.join(cas_path);
            let bytes = std::fs::read(&source_path).map_err(|error| {
                format!(
                    "failed to read pinned source blob {} for owner {}: {error}",
                    source_path.display(),
                    relative_path
                )
            })?;
            let source = String::from_utf8(bytes).map_err(|error| {
                format!(
                    "failed to decode pinned source blob for owner {} as UTF-8: {error}",
                    relative_path
                )
            })?;
            let (items, parse_error) = match crate::parser::parse_rust_source_syntax(&source) {
                Ok(syntax) => (
                    parse_artifact::collect_parse_artifact_items(&syntax.items),
                    None,
                ),
                Err(error) => (Vec::new(), Some(error.to_string())),
            };
            sources.insert(
                relative_path,
                PinnedSource {
                    source,
                    blob_digest: owner.blob_digest,
                    parser_artifact_digest: owner.parser_artifact_digest,
                    parse_error,
                    items,
                },
            );
        }
        if envelope.source_snapshot.leaf_count < sources.len() {
            return Err(format!(
                "source snapshot leaf count is smaller than provider owner count: leafCount={} ownerCount={}",
                envelope.source_snapshot.leaf_count,
                sources.len()
            ));
        }
        Ok(Self {
            provider_id: envelope.provider_id,
            root_digest: envelope.source_snapshot.root_digest,
            sources,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExactSourceSnapshotEnvelopeV1 {
    schema_id: String,
    schema_version: String,
    provider_id: String,
    source_snapshot: ExactSourceSnapshotEvidenceV1,
    cas_root: std::path::PathBuf,
    owners: Vec<ExactSourceSnapshotOwnerV1>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExactSourceSnapshotEvidenceV1 {
    schema_id: String,
    algorithm: String,
    root_digest: String,
    leaf_count: usize,
    provider_digest: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExactSourceSnapshotOwnerV1 {
    path: String,
    snapshot_leaf_digest: String,
    blob_digest: String,
    cas_path: String,
    #[serde(default)]
    parser_artifact_digest: Option<String>,
}

fn normalize_snapshot_owner_path(path: &str) -> Result<String, String> {
    let path = std::path::Path::new(path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(format!(
            "source snapshot owner path escapes workspace: {}",
            path.display()
        ));
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.is_empty()
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == ".")
    {
        return Err(format!(
            "source snapshot owner path is not canonical: {normalized}"
        ));
    }
    Ok(normalized)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedExactItem {
    canonical_selector: crate::semantic_identity::canonical_item_identity::CanonicalItemSelectorV1,
    owner_path: String,
    identity: crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1,
    code: String,
    owner_blob_digest: String,
    parser_artifact_digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RelocationOutcome {
    Resolved(Box<ResolvedExactItem>),
    Ambiguous(Vec<String>),
    KindMismatch(Vec<String>),
    Missing,
}

enum RelocationCandidate {
    Resolved(Box<ResolvedExactItem>),
    KindMismatch(String),
}

fn resolve_live_item(
    workspace: &PinnedWorkspace,
    selector: &ExactSelector,
) -> Result<Option<ResolvedExactItem>, String> {
    let Some(source) = workspace.sources.get(&selector.owner_path) else {
        return Ok(None);
    };
    if let Some(error) = source.parse_error.as_deref() {
        return Err(format!(
            "exact source query state=parser-failed rootDigest={} ownerPath={} error={error}",
            workspace.root_digest, selector.owner_path
        ));
    }
    let matches = source
        .items
        .iter()
        .filter(|item| exact_item_name_matches(item, &selector.item_name))
        .filter(|item| exact_item_kind_matches(item.identity.kind.as_str(), &selector.item_kind))
        .filter(|item| exact_item_scopes_match(item, &selector.scopes))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(format!(
            "exact source query state=ambiguous rootDigest={} ownerPath={} itemKind={} itemName={} matches={}",
            workspace.root_digest,
            selector.owner_path,
            selector.item_kind,
            selector.item_name,
            matches.len()
        ));
    }
    if let Some(item) = matches.into_iter().next() {
        return Ok(Some(resolved_exact_item(
            &selector.owner_path,
            source,
            item,
        )));
    }

    if selector.scopes.is_empty() {
        let scope_relaxed_matches = source
            .items
            .iter()
            .filter(|item| exact_item_name_matches(item, &selector.item_name))
            .filter(|item| {
                exact_item_kind_matches(item.identity.kind.as_str(), &selector.item_kind)
            })
            .collect::<Vec<_>>();
        if scope_relaxed_matches.len() > 1 {
            return Err(format!(
                "exact source query state=ambiguous rootDigest={} ownerPath={} itemKind={} itemName={} scopeRelaxedMatches={}",
                workspace.root_digest,
                selector.owner_path,
                selector.item_kind,
                selector.item_name,
                scope_relaxed_matches.len()
            ));
        }
        if let Some(item) = scope_relaxed_matches.into_iter().next() {
            return Ok(Some(resolved_exact_item(
                &selector.owner_path,
                source,
                item,
            )));
        }
    }

    Ok(None)
}

fn relocate_live_item(
    workspace: &PinnedWorkspace,
    selector: &ExactSelector,
) -> Result<RelocationOutcome, String> {
    let (mut resolved, actual_kinds) = relocation_candidates(workspace, selector).fold(
        (Vec::new(), std::collections::BTreeSet::new()),
        |(mut resolved, mut actual_kinds), candidate| {
            match candidate {
                RelocationCandidate::Resolved(item) => resolved.push(*item),
                RelocationCandidate::KindMismatch(kind) => {
                    actual_kinds.insert(kind);
                }
            }
            (resolved, actual_kinds)
        },
    );
    resolved.sort_by(|left, right| {
        left.canonical_selector
            .structural_selector
            .cmp(&right.canonical_selector.structural_selector)
    });
    match resolved.len() {
        0 if !actual_kinds.is_empty() => Ok(RelocationOutcome::KindMismatch(
            actual_kinds.into_iter().collect(),
        )),
        0 => Ok(RelocationOutcome::Missing),
        1 => Ok(RelocationOutcome::Resolved(Box::new(
            resolved.pop().expect("one relocation candidate"),
        ))),
        _ => Ok(RelocationOutcome::Ambiguous(
            resolved
                .into_iter()
                .map(|item| item.canonical_selector.structural_selector)
                .collect(),
        )),
    }
}

fn relocation_candidates<'a>(
    workspace: &'a PinnedWorkspace,
    selector: &'a ExactSelector,
) -> impl Iterator<Item = RelocationCandidate> + 'a {
    workspace
        .sources
        .iter()
        .filter(|(_, source)| source.parse_error.is_none())
        .flat_map(move |(owner_path, source)| {
            source
                .items
                .iter()
                .filter_map(move |item| relocation_candidate(owner_path, source, item, selector))
        })
}

fn relocation_candidate(
    owner_path: &str,
    source: &PinnedSource,
    item: &ParseArtifactItem,
    selector: &ExactSelector,
) -> Option<RelocationCandidate> {
    if !exact_item_name_matches(item, &selector.item_name) {
        return None;
    }
    if !exact_item_kind_matches(item.identity.kind.as_str(), &selector.item_kind) {
        return Some(RelocationCandidate::KindMismatch(
            item.identity.kind.as_str().to_string(),
        ));
    }
    exact_item_scopes_match(item, &selector.scopes).then(|| {
        RelocationCandidate::Resolved(Box::new(resolved_exact_item(owner_path, source, item)))
    })
}

fn resolved_exact_item(
    owner_path: &str,
    source: &PinnedSource,
    item: &ParseArtifactItem,
) -> ResolvedExactItem {
    ResolvedExactItem {
        canonical_selector: {
            let structural_selector = rust_structural_selector(owner_path, &item.identity);
            crate::semantic_identity::canonical_item_identity::CanonicalItemSelectorV1::new(
                item.identity.clone(),
                structural_selector,
            )
        },
        owner_path: owner_path.to_string(),
        identity: item.identity.clone(),
        code: source_line_window(&source.source, item.start_line, item.end_line),
        owner_blob_digest: source.blob_digest.clone(),
        parser_artifact_digest: source.parser_artifact_digest.clone(),
    }
}

fn source_line_window(source: &str, start_line: usize, end_line: usize) -> String {
    source
        .lines()
        .skip(start_line.saturating_sub(1))
        .take(end_line.saturating_sub(start_line).saturating_add(1))
        .collect::<Vec<_>>()
        .join("\n")
}

fn exact_item_kind_matches(actual: &str, requested: &str) -> bool {
    exact_source_canonical_rust_item_kind(actual)
        == exact_source_canonical_rust_item_kind(requested)
}

fn exact_item_name_matches(item: &ParseArtifactItem, requested: &str) -> bool {
    let canonical = rust_canonical_item_name(&item.identity);
    canonical == requested || item.identity.symbol.as_str() == requested
}

fn exact_item_scopes_match(
    item: &ParseArtifactItem,
    requested: &[crate::semantic_identity::canonical_item_identity::CanonicalItemScopeV1],
) -> bool {
    item.identity.scopes == requested
}

fn rust_canonical_item_name(
    identity: &crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1,
) -> String {
    let impl_owner = identity
        .scopes
        .iter()
        .find(|scope| scope.relation.as_str() == "implementation-owner")
        .map(|scope| scope.symbol.as_str());
    let trait_owner = identity
        .scopes
        .iter()
        .find(|scope| scope.relation.as_str() == "trait-owner")
        .map(|scope| scope.symbol.as_str());
    let owner = match (impl_owner, trait_owner) {
        (Some(impl_owner), Some(trait_owner)) => Some(format!("<{impl_owner} as {trait_owner}>")),
        (Some(impl_owner), None) => Some(impl_owner.to_string()),
        (None, Some(trait_owner)) => Some(trait_owner.to_string()),
        (None, None) => None,
    };
    match owner {
        Some(owner) if identity.kind.as_str() == "impl" => owner,
        Some(owner) => format!("{owner}::{}", identity.symbol.as_str()),
        None => identity.symbol.as_str().to_string(),
    }
}

fn rust_structural_selector(
    owner_path: &str,
    identity: &crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1,
) -> String {
    format!(
        "rust://{owner_path}#{}",
        crate::semantic_identity::structural_selector::encode_canonical_item_identity_path(
            identity
        )
    )
}

fn owner_live_item_diagnostics(
    source: &PinnedSource,
    owner_path: &str,
    selector: &ExactSelector,
) -> (Vec<String>, Vec<String>) {
    const MAX_DIAGNOSTIC_CANDIDATES: usize = 32;

    let mut same_name_candidates = Vec::new();
    let mut actual_kinds = std::collections::BTreeSet::new();
    for item in &source.items {
        if exact_item_name_matches(item, &selector.item_name) {
            same_name_candidates.push(rust_structural_selector(owner_path, &item.identity));
            if !exact_item_kind_matches(item.identity.kind.as_str(), &selector.item_kind) {
                actual_kinds.insert(
                    exact_source_canonical_rust_item_kind(item.identity.kind.as_str()).to_string(),
                );
            }
        }
    }

    let mut candidates = if same_name_candidates.is_empty() {
        source
            .items
            .iter()
            .map(|item| rust_structural_selector(owner_path, &item.identity))
            .take(MAX_DIAGNOSTIC_CANDIDATES)
            .collect::<Vec<_>>()
    } else {
        same_name_candidates
    };
    candidates.sort();
    candidates.dedup();
    candidates.truncate(MAX_DIAGNOSTIC_CANDIDATES);

    (candidates, actual_kinds.into_iter().collect())
}

fn exact_source_failure(
    selector: &ExactSelector,
    pinned: &PinnedWorkspace,
    state: &str,
    reason_kind: &str,
    candidates: Vec<String>,
    actual_kinds: Vec<String>,
) -> Result<ExitCode, String> {
    Err(format!(
        "exact source query state={state} reasonKind={reason_kind} rootDigest={} ownerPath={} itemKind={} itemName={} candidates={} actualKinds={}",
        pinned.root_digest,
        selector.owner_path,
        selector.item_kind,
        selector.item_name,
        candidates.join(","),
        actual_kinds.join(",")
    ))
}

fn exact_source_canonical_rust_item_kind(kind: &str) -> &str {
    match kind {
        "fn" => "function",
        "mod" => "module",
        "use" | "import" => "reexport",
        other => other,
    }
}
#[cfg(test)]
#[path = "../../../../tests/unit/cli/runner/exact_source.rs"]
mod tests;
