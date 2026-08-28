use std::process::ExitCode;

use crate::cli::ExactSourceQuery;
use crate::exact_source_projection::ResolvedExactItem;

use super::model::ParseArtifactItem;

pub(in super::super) fn run_exact_source_query(
    options: ExactSourceQuery,
) -> Result<ExitCode, String> {
    let selector = ExactSelector::parse(&options.selector)
        .map_err(|error| format!("exact source query state=invalid-selector {error}"))?;
    let pinned = if options.exact_request_stdin {
        if options.source_snapshot_envelope.is_some() {
            return Err(
                "exact source query accepts exactly one byte authority: stdin request or source envelope"
                    .to_string(),
            );
        }
        PinnedWorkspace::load_direct_request(&options, &selector)?
    } else {
        let source_snapshot_envelope = options.source_snapshot_envelope.as_ref().ok_or_else(|| {
            "exact source query state=source-unavailable reasonKind=wrapper-byte-authority-required"
                .to_string()
        })?;
        PinnedWorkspace::load(source_snapshot_envelope, &options.provider_id).map_err(|error| {
            format!(
                "exact source query state=source-unavailable reasonKind=snapshot-envelope-invalid {error}"
            )
        })?
    };
    let requested_owner_exists = pinned.sources.contains_key(&selector.owner_path);
    let resolved = if let Some(resolved) = resolve_live_item(&pinned, &selector)? {
        resolved
    } else {
        match relocate_live_item(&pinned, &selector)? {
            RelocationOutcome::Resolved(resolved) => *resolved,
            RelocationOutcome::IdentityIncomplete(candidates) => {
                return exact_source_failure(
                    &options,
                    &selector,
                    &pinned,
                    "identity-incomplete",
                    "canonical-item-scope-required",
                    candidates,
                    Vec::new(),
                );
            }
            RelocationOutcome::Ambiguous(candidates) => {
                return exact_source_failure(
                    &options,
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
                    &options,
                    &selector,
                    &pinned,
                    "kind-mismatch",
                    "owner-item-kind-mismatch",
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
                    &options,
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
    let resolved = if let Some(segment_selector) = selector.segment_selector.as_deref() {
        let authority = pinned.exact_projection_authority.as_ref().ok_or_else(|| {
            "exact callable segment requires typed v1 projection authority".to_string()
        })?;
        resolve_callable_segment(&pinned, resolved, segment_selector, authority)?
    } else {
        resolved
    };
    let code = resolved.code.trim_end_matches('\n').to_string();
    if options.json {
        let authority = pinned.exact_projection_authority.as_ref().ok_or_else(|| {
            "provider-native exact projection requires typed v1 projection authority".to_string()
        })?;
        let packet = crate::exact_source_projection::provider_native_exact_projection_packet(
            &pinned.provider_id,
            &options.selector,
            &options.projection,
            &resolved,
            "resolved",
            authority,
        )?;
        println!(
            "{}",
            serde_json::to_string(&packet)
                .map_err(|error| format!("serialize exact source projection packet: {error}"))?
        );
    } else {
        println!("{code}");
    }

    Ok(ExitCode::SUCCESS)
}

fn resolve_callable_segment(
    workspace: &PinnedWorkspace,
    mut resolved: ResolvedExactItem,
    requested_selector: &str,
    authority: &crate::exact_source_projection::ExactProjectionAuthority,
) -> Result<ResolvedExactItem, String> {
    let payload =
        crate::exact_source_projection::callable_skeleton_projection(&resolved, authority)?;
    let node = payload["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes.iter().find(|node| {
            node["selector"].as_str() == Some(requested_selector)
            })
        })
        .ok_or_else(|| {
            format!(
                "exact source query state=item-missing reasonKind=callable-segment-not-found selector={requested_selector}"
            )
        })?;
    let source_byte_start = node["sourceLocatorHint"]["sourceByteStart"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| "callable segment omitted sourceByteStart".to_string())?;
    let source_byte_end = node["sourceLocatorHint"]["sourceByteEnd"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| "callable segment omitted sourceByteEnd".to_string())?;
    let source = workspace
        .sources
        .get(&resolved.owner_path)
        .ok_or_else(|| format!("callable segment owner is missing: {}", resolved.owner_path))?;
    resolved.code = source
        .source
        .get(source_byte_start..source_byte_end)
        .ok_or_else(|| {
            "exact source query state=parser-failed reasonKind=callable-segment-span-invalid"
                .to_string()
        })?
        .to_string();
    resolved.source_byte_start = source_byte_start;
    resolved.source_byte_end = source_byte_end;
    resolved.canonical_selector.structural_selector = requested_selector.to_string();
    Ok(resolved)
}

fn parse_exact_source_selector(
    selector: &str,
) -> Result<(&str, agent_semantic_content_identity::CanonicalItemIdentity), String> {
    let identity = crate::structural_selector::parse_canonical_item_selector(selector)
        .map_err(|error| format!("exact source selector `{selector}` is invalid: {error}"))?
        .identity();
    let selector = selector
        .strip_prefix("rust://")
        .ok_or_else(|| format!("exact source selector `{selector}` must start with rust://"))?;
    let (owner_path, item_selector) = selector
        .split_once('#')
        .ok_or_else(|| format!("exact source selector `{selector}` must include #item/"))?;
    if owner_path.is_empty() || item_selector.is_empty() {
        return Err(format!("exact source selector `{selector}` is incomplete"));
    }
    Ok((owner_path, identity))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExactSelector {
    owner_path: String,
    item_kind: String,
    item_name: String,
    scopes: Vec<agent_semantic_content_identity::CanonicalItemScope>,
    segment_selector: Option<String>,
}

impl ExactSelector {
    fn parse(selector: &str) -> Result<Self, String> {
        let (root_selector, segment_selector) = parse_exact_selector_segment(selector)?;
        let (owner_path, identity) = parse_exact_source_selector(root_selector)?;
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
            segment_selector,
        })
    }
}

fn parse_exact_selector_segment(selector: &str) -> Result<(&str, Option<String>), String> {
    let Some((root_selector, segment)) = selector.rsplit_once("/segment/") else {
        return Ok((selector, None));
    };
    let (kind, identity) = segment.split_once('/').ok_or_else(|| {
        format!("exact structural selector `{selector}` has an incomplete segment")
    })?;
    if kind.is_empty()
        || identity.contains('/')
        || identity
            .strip_prefix("ordinal-")
            .and_then(|ordinal| ordinal.parse::<u64>().ok())
            .is_none()
    {
        return Err(format!(
            "exact structural selector `{selector}` has a non-canonical segment"
        ));
    }
    Ok((root_selector, Some(selector.to_string())))
}

#[cfg(test)]
fn collect_parse_artifact_items(
    _source: &str,
    items: &[syn::Item],
    output: &mut Vec<ParseArtifactItem>,
) {
    crate::exact_source_parse_artifact::collect_parse_artifact_items(_source, items, output);
}

#[derive(Clone, Debug)]
struct PinnedSource {
    source: String,
    blob_digest: String,
    parser_artifact_digest: Option<String>,
    parse_error: Option<String>,
    items: Vec<crate::exact_source_parse_artifact::ParseArtifactItem>,
}

#[derive(Clone, Debug)]
struct PinnedWorkspace {
    provider_id: String,
    root_digest: String,
    exact_projection_authority: Option<crate::exact_source_projection::ExactProjectionAuthority>,
    sources: std::collections::BTreeMap<String, PinnedSource>,
}

fn snapshot_digest_is_valid(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl PinnedWorkspace {
    fn load_direct_request(
        options: &ExactSourceQuery,
        selector: &ExactSelector,
    ) -> Result<Self, String> {
        use std::io::Read as _;

        let mut request_bytes = Vec::new();
        std::io::stdin()
            .lock()
            .read_to_end(&mut request_bytes)
            .map_err(|error| format!("failed to read exact source stdin request: {error}"))?;
        let request: ProviderNativeExactRequest = serde_json::from_slice(&request_bytes)
            .map_err(|error| format!("invalid exact source stdin request: {error}"))?;
        if request.schema_id != "agent.semantic-protocols.provider-native-exact-request"
            || request.schema_version != "1"
            || request.transport != "stdin-json"
            || request.source_encoding != "base64"
        {
            return Err(
                "exact source stdin request has an unsupported v1 contract identity".to_string(),
            );
        }
        if request.language_id != "rust"
            || request.structural_selector != options.selector
            || request.owner_path != selector.owner_path
            || request.projection_kind != options.projection
            || request.provider_id != options.provider_id
        {
            return Err(
                "exact source stdin request identity does not match activated query".to_string(),
            );
        }
        let source_bytes = super::decode_canonical_base64(request.source_bytes_base64.as_bytes())
            .ok_or_else(|| {
            "exact source stdin request contains invalid base64 bytes".to_string()
        })?;
        if source_bytes.len() as u64 != request.source_byte_length {
            return Err(format!(
                "exact source stdin byte length mismatch: expected={} actual={}",
                request.source_byte_length,
                source_bytes.len()
            ));
        }
        let actual_digest = blake3::hash(&source_bytes).to_hex().to_string();
        if actual_digest != request.source_digest {
            return Err(format!(
                "exact source stdin digest mismatch: expected={} actual={actual_digest}",
                request.source_digest
            ));
        }
        let source = String::from_utf8(source_bytes)
            .map_err(|error| format!("exact source stdin bytes are not UTF-8: {error}"))?;
        let (items, parse_error) =
            match crate::exact_source_parse_artifact::parse_owner_items_v1(&source) {
                Ok(items) => (items, None),
                Err(error) => (Vec::new(), Some(error.to_string())),
            };
        let mut sources = std::collections::BTreeMap::new();
        sources.insert(
            request.owner_path,
            PinnedSource {
                source,
                blob_digest: actual_digest.clone(),
                parser_artifact_digest: None,
                parse_error,
                items,
            },
        );
        Ok(Self {
            provider_id: request.provider_id,
            root_digest: actual_digest,
            exact_projection_authority: Some(
                crate::exact_source_projection::ExactProjectionAuthority {
                    generation_identity_digest: request.generation_identity_digest,
                    parser_identity_digest: request.parser_identity_digest,
                    query_pack_digest: request.query_pack_digest,
                },
            ),
            sources,
        })
    }

    fn load(envelope_path: &std::path::Path, provider_id: &str) -> Result<Self, String> {
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
        if envelope.provider_id != provider_id {
            return Err(format!(
                "source snapshot envelope provider identity drift: expected={provider_id} actual={}",
                envelope.provider_id
            ));
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
            let (items, parse_error) =
                match crate::exact_source_parse_artifact::parse_owner_items_v1(&source) {
                    Ok(items) => (items, None),
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
            exact_projection_authority: None,
            sources,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderNativeExactRequest {
    schema_id: String,
    schema_version: String,
    language_id: String,
    provider_id: String,
    structural_selector: String,
    owner_path: String,
    projection_kind: String,
    generation_identity_digest: String,
    parser_identity_digest: String,
    query_pack_digest: String,
    source_digest: String,
    source_byte_length: u64,
    source_encoding: String,
    source_bytes_base64: String,
    transport: String,
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
enum RelocationOutcome {
    Resolved(Box<ResolvedExactItem>),
    IdentityIncomplete(Vec<String>),
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

    Ok(None)
}

fn relocate_live_item(
    workspace: &PinnedWorkspace,
    selector: &ExactSelector,
) -> Result<RelocationOutcome, String> {
    if selector.scopes.is_empty() {
        let mut candidates = workspace
            .sources
            .iter()
            .flat_map(|(owner_path, source)| {
                source.items.iter().filter_map(move |item| {
                    (exact_item_name_matches(item, &selector.item_name)
                        && exact_item_kind_matches(
                            item.identity.kind.as_str(),
                            &selector.item_kind,
                        )
                        && !item.identity.scopes.is_empty())
                    .then(|| {
                        resolved_exact_item(owner_path, source, item)
                            .canonical_selector
                            .structural_selector
                    })
                })
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.dedup();
        if !candidates.is_empty() {
            return Ok(RelocationOutcome::IdentityIncomplete(candidates));
        }
    }
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
    item: &crate::exact_source_parse_artifact::ParseArtifactItem,
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
            agent_semantic_content_identity::CanonicalItemSelector::new(
                item.identity.clone(),
                structural_selector,
            )
        },
        owner_path: owner_path.to_string(),
        identity: item.identity.clone(),
        code: source
            .source
            .get(item.source_byte_start..item.source_byte_end)
            .unwrap_or_default()
            .to_string(),
        source_byte_start: item.source_byte_start,
        source_byte_end: item.source_byte_end,
        owner_blob_digest: source.blob_digest.clone(),
        parser_artifact_digest: source.parser_artifact_digest.clone(),
    }
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
    requested: &[agent_semantic_content_identity::CanonicalItemScope],
) -> bool {
    item.identity.scopes == requested
}

fn rust_canonical_item_name(
    identity: &agent_semantic_content_identity::CanonicalItemIdentity,
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

pub(crate) fn rust_structural_selector(
    owner_path: &str,
    identity: &agent_semantic_content_identity::CanonicalItemIdentity,
) -> String {
    format!(
        "rust://{owner_path}#{}",
        crate::structural_selector::encode_canonical_item_identity_path(identity)
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
    options: &ExactSourceQuery,
    selector: &ExactSelector,
    pinned: &PinnedWorkspace,
    state: &str,
    reason_kind: &str,
    candidates: Vec<String>,
    actual_kinds: Vec<String>,
) -> Result<ExitCode, String> {
    if options.json {
        let packet = serde_json::json!({
            "schemaId": "agent.semantic-protocols.provider-native-exact-projection",
            "schemaVersion": "1",
            "languageId": "rust",
            "providerId": pinned.provider_id,
            "requestedStructuralSelector": options.selector,
            "resolutionState": state,
            "reasonKind": reason_kind,
            "activeGenerationDigest": pinned
                .exact_projection_authority
                .as_ref()
                .map(|authority| authority.generation_identity_digest.as_str())
                .unwrap_or(pinned.root_digest.as_str()),
            "rootDigest": pinned.root_digest,
            "ownerPath": selector.owner_path,
            "itemKind": selector.item_kind,
            "itemName": selector.item_name,
            "candidates": candidates,
            "actualKinds": actual_kinds,
        });
        println!(
            "{}",
            serde_json::to_string(&packet)
                .map_err(|error| format!("serialize exact source miss packet: {error}"))?
        );
        return Ok(ExitCode::SUCCESS);
    }
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
