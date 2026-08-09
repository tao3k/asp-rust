use std::process::ExitCode;

use crate::cli::query::ExactSourceQuery;

pub(crate) fn run_exact_source_query(options: ExactSourceQuery) -> Result<ExitCode, String> {
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
        PinnedWorkspace::load(source_snapshot_envelope).map_err(|error| {
            format!(
                "exact source query state=source-unavailable reasonKind=snapshot-envelope-invalid {error}"
            )
        })?
    };
    let requested_owner_exists = pinned.sources.contains_key(&selector.owner_path);
    let local_resolution = resolve_live_item(&pinned, &selector)?;
    let (resolved, state) = match local_resolution {
        RelocationOutcome::Resolved(resolved) => (resolved, "live-hit"),
        RelocationOutcome::Ambiguous(candidates) => {
            return exact_source_failure(ExactSourceFailure {
                selector: &selector,
                pinned: &pinned,
                requested_structural_selector: &options.selector,
                state: "ambiguous",
                reason_kind: "multiple-owner-items",
                candidates,
                actual_kinds: Vec::new(),
                json: options.json,
            });
        }
        RelocationOutcome::IdentityIncomplete(candidates) => {
            return exact_source_failure(ExactSourceFailure {
                selector: &selector,
                pinned: &pinned,
                requested_structural_selector: &options.selector,
                state: "identity-incomplete",
                reason_kind: "canonical-item-scope-required",
                candidates,
                actual_kinds: Vec::new(),
                json: options.json,
            });
        }
        RelocationOutcome::KindMismatch(actual_kinds) => {
            return exact_source_failure(ExactSourceFailure {
                selector: &selector,
                pinned: &pinned,
                requested_structural_selector: &options.selector,
                state: "kind-mismatch",
                reason_kind: "owner-item-kind-mismatch",
                candidates: Vec::new(),
                actual_kinds,
                json: options.json,
            });
        }
        RelocationOutcome::Missing => match relocate_live_item(&pinned, &selector)? {
            RelocationOutcome::Resolved(resolved) => (resolved, "live-relocated"),
            RelocationOutcome::Ambiguous(candidates) => {
                return exact_source_failure(ExactSourceFailure {
                    selector: &selector,
                    pinned: &pinned,
                    requested_structural_selector: &options.selector,
                    state: "ambiguous",
                    reason_kind: "multiple-snapshot-items",
                    candidates,
                    actual_kinds: Vec::new(),
                    json: options.json,
                });
            }
            RelocationOutcome::KindMismatch(actual_kinds) => {
                unreachable!(
                    "relocation never classifies kind mismatches; actualKinds={actual_kinds:?}"
                );
            }
            RelocationOutcome::IdentityIncomplete(candidates) => {
                unreachable!(
                    "relocation requires a complete canonical identity; candidates={candidates:?}"
                );
            }
            RelocationOutcome::Missing => {
                let (state, reason_kind) =
                    missing_resolution_classification(requested_owner_exists);
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
                return exact_source_failure(ExactSourceFailure {
                    selector: &selector,
                    pinned: &pinned,
                    requested_structural_selector: &options.selector,
                    state,
                    reason_kind,
                    candidates,
                    actual_kinds,
                    json: options.json,
                });
            }
        },
    };
    let resolved = if let Some(segment) = selector.segment.as_ref() {
        exact_source_projection::resolve_callable_segment(&resolved, segment)?
    } else {
        resolved
    };
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
        let packet = exact_source_projection::provider_native_exact_projection_packet(
            provider_id,
            &options.selector,
            &resolved,
            state,
            pinned.exact_projection_authority.as_ref(),
        )?;
        println!(
            "{}",
            serde_json::to_string(&packet)
                .map_err(|error| format!("serialize exact source projection packet: {error}"))?
        );
    } else if options.projection == "source" {
        print!("{}", resolved.code);
    } else {
        let authority = pinned.exact_projection_authority.as_ref().ok_or_else(|| {
            "callable-skeleton projection requires typed projection authority".to_string()
        })?;
        if authority.projection_kind != options.projection {
            return Err(format!(
                "exact projection mismatch: requested={} authority={}",
                options.projection, authority.projection_kind
            ));
        }
        let packet = exact_source_projection::provider_native_exact_projection_packet(
            &pinned.provider_id,
            &options.selector,
            &resolved,
            state,
            Some(authority),
        )?;
        println!(
            "{}",
            serde_json::to_string(
                packet
                    .get("projectionPayload")
                    .ok_or_else(|| "callable-skeleton projection omitted payload".to_string())?
            )
            .map_err(|error| format!("serialize callable-skeleton projection: {error}"))?
        );
    }

    Ok(ExitCode::SUCCESS)
}

fn missing_resolution_classification(requested_owner_exists: bool) -> (&'static str, &'static str) {
    if requested_owner_exists {
        ("item-missing", "item-not-in-live-owner")
    } else {
        ("owner-missing", "owner-not-in-workspace")
    }
}

use crate::exact_source_projection;

fn parse_exact_source_selector(
    selector: &str,
) -> Result<
    (
        &str,
        crate::canonical_item_identity::CanonicalItemIdentityV1,
    ),
    String,
> {
    let canonical_selector = crate::structural_selector::parse_canonical_item_selector(selector)
        .map_err(|error| format!("exact source selector `{selector}` is invalid: {error}"))?;
    let selector_without_scheme = selector
        .strip_prefix("rust://")
        .ok_or_else(|| format!("exact source selector `{selector}` must start with rust://"))?;
    let (owner_path, item_selector) = selector_without_scheme
        .split_once('#')
        .ok_or_else(|| format!("exact source selector `{selector}` must include #item/"))?;
    if owner_path.is_empty() || item_selector.is_empty() {
        return Err(format!("exact source selector `{selector}` is incomplete"));
    }
    Ok((owner_path, canonical_selector.identity()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExactSelector {
    pub(crate) owner_path: String,
    pub(crate) item_kind: String,
    pub(crate) item_name: String,
    pub(crate) scopes: Vec<crate::canonical_item_identity::CanonicalItemScopeV1>,
    pub(crate) segment: Option<ExactSelectorSegment>,
}

pub(crate) use crate::exact_source_encoding::decode_canonical_base64;
pub(crate) use crate::exact_source_parse_artifact::{
    ParseArtifactItem, collect_parse_artifact_items, parse_owner_items_v1,
};
pub(crate) use crate::exact_source_projection::{
    ExactProjectionAuthority, ExactSelectorSegment, ResolvedExactItem,
};

impl ExactSelector {
    pub(crate) fn parse(selector: &str) -> Result<Self, String> {
        let (root_selector, segment) = parse_exact_selector_segment(selector)?;
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
            item_kind: identity.kind.to_string(),
            item_name: identity.symbol.as_str().to_string(),
            scopes: identity.scopes,
            segment,
        })
    }
}

fn parse_exact_selector_segment(
    selector: &str,
) -> Result<(&str, Option<ExactSelectorSegment>), String> {
    let Some((root_selector, segment)) = selector.rsplit_once("/segment/") else {
        return Ok((selector, None));
    };
    let (kind, identity) = segment.split_once('/').ok_or_else(|| {
        format!("exact structural selector `{selector}` has an incomplete segment")
    })?;
    if kind.is_empty() || identity.contains('/') {
        return Err(format!(
            "exact structural selector `{selector}` has a non-canonical segment"
        ));
    }
    let ordinal = identity
        .strip_prefix("ordinal-")
        .ok_or_else(|| {
            format!("exact structural selector `{selector}` lacks parser-owned ordinal identity")
        })?
        .parse::<u64>()
        .map_err(|_| {
            format!("exact structural selector `{selector}` has an invalid ordinal identity")
        })?;
    Ok((
        root_selector,
        Some(ExactSelectorSegment {
            kind: kind.to_string(),
            ordinal,
        }),
    ))
}

#[derive(Clone, Debug)]
pub(crate) struct PinnedSource {
    pub(crate) source: String,
    pub(crate) blob_digest: String,
    pub(crate) parser_artifact_digest: Option<String>,
    pub(crate) parse_error: Option<String>,
    pub(crate) items: Vec<ParseArtifactItem>,
}

#[derive(Clone, Debug)]
pub(crate) struct PinnedWorkspace {
    pub(crate) provider_id: String,
    pub(crate) root_digest: String,
    pub(crate) exact_projection_authority: Option<ExactProjectionAuthority>,
    pub(crate) sources: std::collections::BTreeMap<String, PinnedSource>,
}

fn snapshot_digest_is_valid(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl PinnedWorkspace {
    fn load_direct_request(
        options: &ExactSourceQuery,
        selector: &ExactSelector,
    ) -> Result<Self, String> {
        use std::io::Read;

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
            || options.provider_id.as_deref() != Some(request.provider_id.as_str())
        {
            return Err(
                "exact source stdin request identity does not match activated query".to_string(),
            );
        }
        if !matches!(
            request.projection_kind.as_str(),
            "source" | "callable-skeleton"
        ) || !snapshot_digest_is_valid(&request.generation_identity_digest)
            || !snapshot_digest_is_valid(&request.parser_identity_digest)
            || !snapshot_digest_is_valid(&request.query_pack_digest)
        {
            return Err(
                "exact source stdin request lacks complete v1 projection authority".to_string(),
            );
        }
        let source_bytes = decode_canonical_base64(request.source_bytes_base64.as_bytes())
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
            .map_err(|error| format!("exact source stdin owner is not UTF-8: {error}"))?;
        let mut items = Vec::new();
        let parse_error = match crate::parser::parse_rust_source_syntax(&source) {
            Ok(file) => {
                collect_parse_artifact_items(&source, &file.items, &mut items);
                None
            }
            Err(error) => Some(error.to_string()),
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
            exact_projection_authority: Some(ExactProjectionAuthority {
                projection_kind: request.projection_kind,
                generation_identity_digest: request.generation_identity_digest,
                parser_identity_digest: request.parser_identity_digest,
                query_pack_digest: request.query_pack_digest,
            }),
            sources,
        })
    }

    pub(crate) fn load(envelope_path: &std::path::Path) -> Result<Self, String> {
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
        validate_exact_source_envelope_coverage(&envelope)?;
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
                || !snapshot_digest_is_valid(owner.source_content_digest.as_str())
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
            let live_source_content_digest = blake3::hash(&bytes).to_hex().to_string();
            if live_source_content_digest != owner.source_content_digest {
                return Err(format!(
                    "pinned source content digest mismatch for owner {}: expected={} actual={}",
                    relative_path, owner.source_content_digest, live_source_content_digest
                ));
            }
            let source = String::from_utf8(bytes).map_err(|error| {
                format!(
                    "failed to decode pinned source blob for owner {} as UTF-8: {error}",
                    relative_path
                )
            })?;
            let mut items = Vec::new();
            let parse_error = match crate::parser::parse_rust_source_syntax(&source) {
                Ok(syntax) => {
                    collect_parse_artifact_items(&source, &syntax.items, &mut items);
                    None
                }
                Err(error) => Some(error.to_string()),
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
    root_depth: usize,
    materialization_state: String,
    owner_coverage: String,
    cas_root: std::path::PathBuf,
    owners: Vec<ExactSourceSnapshotOwnerV1>,
}

pub(crate) fn exact_source_merkle_root_depth(leaf_count: usize) -> usize {
    if leaf_count <= 1 {
        0
    } else {
        usize::BITS as usize - (leaf_count - 1).leading_zeros() as usize
    }
}

fn validate_exact_source_envelope_coverage(
    envelope: &ExactSourceSnapshotEnvelopeV1,
) -> Result<(), String> {
    let expected_root_depth = exact_source_merkle_root_depth(envelope.source_snapshot.leaf_count);
    if envelope.root_depth != expected_root_depth
        || envelope.materialization_state != "artifact-complete"
        || envelope.owner_coverage != "complete"
    {
        return Err(format!(
            "source snapshot envelope lacks complete generation coverage: rootDepth={} expectedRootDepth={} materializationState={} ownerCoverage={}",
            envelope.root_depth,
            expected_root_depth,
            envelope.materialization_state,
            envelope.owner_coverage
        ));
    }
    if envelope.source_snapshot.leaf_count != envelope.owners.len() {
        return Err(format!(
            "source snapshot provider coverage mismatch: leafCount={} ownerCount={}",
            envelope.source_snapshot.leaf_count,
            envelope.owners.len()
        ));
    }
    let reconstructed_snapshot =
        crate::provider_workspace_search_identity::WorkspaceSnapshot::from_file_hashes(
            envelope
                .owners
                .iter()
                .map(|owner| (owner.path.as_str(), owner.snapshot_leaf_digest.as_str())),
        );
    if reconstructed_snapshot.root_digest() != envelope.source_snapshot.root_digest {
        return Err(format!(
            "source snapshot provider generation mismatch: expectedRootDigest={} actualRootDigest={}",
            envelope.source_snapshot.root_digest,
            reconstructed_snapshot.root_digest()
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/cli/runner/exact_source_envelope_coverage.rs"]
mod envelope_coverage_tests;

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
    source_content_digest: String,
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
pub(crate) enum RelocationOutcome {
    Resolved(ResolvedExactItem),
    Ambiguous(Vec<String>),
    IdentityIncomplete(Vec<String>),
    KindMismatch(Vec<String>),
    Missing,
}

pub(crate) fn resolve_live_item(
    workspace: &PinnedWorkspace,
    selector: &ExactSelector,
) -> Result<RelocationOutcome, String> {
    let Some(source) = workspace.sources.get(&selector.owner_path) else {
        return Ok(RelocationOutcome::Missing);
    };
    if let Some(error) = source.parse_error.as_deref() {
        return Err(format!(
            "exact source query state=parser-failed rootDigest={} ownerPath={} error={error}",
            workspace.root_digest, selector.owner_path
        ));
    }
    let identity_matches = source
        .items
        .iter()
        .filter(|item| exact_item_name_matches(item, &selector.item_name))
        .filter(|item| exact_item_scopes_match(item, &selector.scopes))
        .collect::<Vec<_>>();
    let kind_matches = identity_matches
        .iter()
        .copied()
        .filter(|item| exact_item_kind_matches(item.identity.kind.as_str(), &selector.item_kind))
        .collect::<Vec<_>>();
    match kind_matches.len() {
        1 => {
            return Ok(RelocationOutcome::Resolved(resolved_exact_item(
                &selector.owner_path,
                source,
                kind_matches[0],
            )));
        }
        count if count > 1 => {
            return Ok(RelocationOutcome::Ambiguous(
                kind_matches
                    .into_iter()
                    .map(|item| rust_structural_selector(&selector.owner_path, &item.identity))
                    .collect(),
            ));
        }
        _ => {}
    }
    if selector.scopes.is_empty() {
        let scoped_candidates = source
            .items
            .iter()
            .filter(|item| exact_item_name_matches(item, &selector.item_name))
            .filter(|item| {
                exact_item_kind_matches(item.identity.kind.as_str(), &selector.item_kind)
            })
            .filter(|item| !item.identity.scopes.is_empty())
            .collect::<Vec<_>>();
        if !scoped_candidates.is_empty() {
            return Ok(RelocationOutcome::IdentityIncomplete(
                scoped_candidates
                    .into_iter()
                    .map(|item| rust_structural_selector(&selector.owner_path, &item.identity))
                    .collect(),
            ));
        }
    }
    if !identity_matches.is_empty() {
        let actual_kinds = identity_matches
            .into_iter()
            .map(|item| item.identity.kind.to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        return Ok(RelocationOutcome::KindMismatch(actual_kinds));
    }
    Ok(RelocationOutcome::Missing)
}

pub(crate) fn relocate_live_item(
    workspace: &PinnedWorkspace,
    selector: &ExactSelector,
) -> Result<RelocationOutcome, String> {
    let mut resolved = Vec::new();
    for (owner_path, source) in &workspace.sources {
        if source.parse_error.is_some() {
            continue;
        }
        for item in &source.items {
            if !exact_item_name_matches(item, &selector.item_name) {
                continue;
            }
            if exact_item_kind_matches(item.identity.kind.as_str(), &selector.item_kind)
                && exact_item_scopes_match(item, &selector.scopes)
            {
                resolved.push(resolved_exact_item(owner_path, source, item));
            }
        }
    }
    resolved.sort_by(|left, right| {
        left.canonical_selector
            .structural_selector
            .cmp(&right.canonical_selector.structural_selector)
    });
    match resolved.len() {
        0 => Ok(RelocationOutcome::Missing),
        1 => Ok(RelocationOutcome::Resolved(
            resolved.pop().expect("one relocation candidate"),
        )),
        _ => Ok(RelocationOutcome::Ambiguous(
            resolved
                .into_iter()
                .map(|item| item.canonical_selector.structural_selector)
                .collect(),
        )),
    }
}

fn resolved_exact_item(
    owner_path: &str,
    source: &PinnedSource,
    item: &ParseArtifactItem,
) -> ResolvedExactItem {
    ResolvedExactItem {
        canonical_selector: {
            let structural_selector = rust_structural_selector(owner_path, &item.identity);
            let selector = crate::canonical_item_identity::CanonicalItemSelectorV1::new(
                item.identity.clone(),
                structural_selector,
            );
            selector
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
    actual == requested
}

fn exact_item_name_matches(item: &ParseArtifactItem, requested: &str) -> bool {
    let canonical = rust_canonical_item_name(&item.identity);
    canonical == requested || (canonical != requested && item.identity.symbol.as_str() == requested)
}

fn exact_item_scopes_match(
    item: &ParseArtifactItem,
    requested: &[crate::canonical_item_identity::CanonicalItemScopeV1],
) -> bool {
    item.identity.scopes == requested
}

fn rust_canonical_item_name(
    identity: &crate::canonical_item_identity::CanonicalItemIdentityV1,
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

pub(super) fn rust_structural_selector(
    owner_path: &str,
    identity: &crate::canonical_item_identity::CanonicalItemIdentityV1,
) -> String {
    format!(
        "rust://{owner_path}#{}",
        crate::structural_selector::encode_canonical_item_identity_path(identity)
    )
}

pub(crate) fn owner_live_item_diagnostics(
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
                actual_kinds.insert(item.identity.kind.to_string());
            }
        }
    }

    let mut candidates = if same_name_candidates.is_empty() {
        source
            .items
            .iter()
            .filter(|item| {
                exact_item_kind_matches(item.identity.kind.as_str(), &selector.item_kind)
            })
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

struct ExactSourceFailure<'a> {
    selector: &'a ExactSelector,
    pinned: &'a PinnedWorkspace,
    requested_structural_selector: &'a str,
    state: &'a str,
    reason_kind: &'a str,
    candidates: Vec<String>,
    actual_kinds: Vec<String>,
    json: bool,
}

fn exact_source_failure_packet(failure: &ExactSourceFailure<'_>) -> serde_json::Value {
    let mut packet = serde_json::json!({
        "schemaId": "agent.semantic-protocols.provider-native-exact-projection",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": failure.pinned.provider_id,
        "requestedStructuralSelector": failure.requested_structural_selector,
        "resolutionState": failure.state,
        "reasonKind": failure.reason_kind,
        "activeGenerationDigest": format!(
            "blake3-256:{}",
            failure
                .pinned
                .exact_projection_authority
                .as_ref()
                .map(|authority| authority.generation_identity_digest.as_str())
                .unwrap_or(failure.pinned.root_digest.as_str())
        ),
        "rootDigest": failure.pinned.root_digest,
        "ownerPath": failure.selector.owner_path,
        "itemKind": failure.selector.item_kind,
        "itemName": failure.selector.item_name,
        "candidates": failure.candidates,
        "actualKinds": failure.actual_kinds,
    });
    if matches!(failure.state, "selector-stale" | "owner-missing") {
        packet["recommendedNext"] = serde_json::json!({
            "command": format!(
                "asp rust search lexical --query '{}' --query '{} {}' --workspace . --view seeds",
                failure.selector.item_name,
                failure.selector.item_kind,
                failure.selector.item_name,
            ),
        });
    }
    packet
}

fn exact_source_failure(failure: ExactSourceFailure<'_>) -> Result<ExitCode, String> {
    if failure.json {
        let packet = exact_source_failure_packet(&failure);
        println!(
            "{}",
            serde_json::to_string(&packet)
                .map_err(|error| format!("serialize exact source miss packet: {error}"))?
        );
        return Ok(ExitCode::SUCCESS);
    }

    Err(format!(
        "exact source query state={state} reasonKind={reason_kind} rootDigest={} ownerPath={} itemKind={} itemName={} candidates={} actualKinds={}",
        failure.pinned.root_digest,
        failure.selector.owner_path,
        failure.selector.item_kind,
        failure.selector.item_name,
        failure.candidates.join(","),
        failure.actual_kinds.join(","),
        state = failure.state,
        reason_kind = failure.reason_kind,
    ))
}

#[cfg(test)]
#[path = "../tests/unit/exact_source_identity_resolution.rs"]
mod canonical_identity_resolution_tests;
