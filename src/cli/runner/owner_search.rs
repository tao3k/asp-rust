//! Single-owner native reasoning-search transport.

use std::{io::Read, process::ExitCode};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProviderNativeOwnerSearchRequest {
    schema_id: String,
    schema_version: String,
    language_id: String,
    provider_id: String,
    workspace_identity: String,
    provider_workspace_identity_digest: String,
    owner_path: String,
    source_fingerprint: ProviderOwnerFingerprint,
    source_encoding: String,
    source_bytes_base64: String,
    projection_mode: String,
    transport: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderOwnerFingerprint {
    file_identity: String,
    size_bytes: u64,
    modified_unix_nanos: i64,
    change_time_unix_nanos: i64,
    content_digest: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderNativeOwnerSearchResponse {
    schema_id: &'static str,
    schema_version: &'static str,
    language_id: &'static str,
    provider_id: String,
    requested_owner_path: String,
    requested_projection_mode: String,
    source_content_digest: String,
    parsed_owner_count: u32,
    projection_completeness: &'static str,
    projections: Vec<ProviderOwnerProjection>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderOwnerProjection {
    canonical_item_selector: agent_semantic_content_identity::CanonicalItemSelector,
    signature: String,
    capture_name: &'static str,
    source_byte_start: u64,
    source_byte_end: u64,
}

pub(super) fn run_owner_search(args: &[std::ffi::OsString]) -> Result<ExitCode, String> {
    let provider_id = provider_id_arg(args)?;
    let mut request_bytes = Vec::new();
    std::io::stdin()
        .lock()
        .read_to_end(&mut request_bytes)
        .map_err(|error| format!("failed to read owner-search stdin request: {error}"))?;
    let response = handle_owner_search_request(&request_bytes, provider_id)?;
    println!(
        "{}",
        serde_json::to_string(&response)
            .map_err(|error| format!("serialize owner-search response: {error}"))?
    );
    Ok(ExitCode::SUCCESS)
}

fn handle_owner_search_request(
    request_bytes: &[u8],
    activated_provider_id: &str,
) -> Result<ProviderNativeOwnerSearchResponse, String> {
    let request: ProviderNativeOwnerSearchRequest = serde_json::from_slice(request_bytes)
        .map_err(|error| format!("invalid owner-search stdin request: {error}"))?;
    validate_request_identity(&request, activated_provider_id)?;
    let source_bytes =
        super::exact_source::decode_canonical_base64(request.source_bytes_base64.as_bytes())
            .ok_or_else(|| {
                "owner-search stdin request contains invalid base64 bytes".to_string()
            })?;
    validate_source_fingerprint(&request, &source_bytes)?;
    let source = String::from_utf8(source_bytes)
        .map_err(|error| format!("owner-search stdin owner is not UTF-8: {error}"))?;
    let projections = crate::exact_source_parse_artifact::parse_owner_items_v1(&source)?
        .into_iter()
        .filter_map(|item| owner_projection(&request.owner_path, &source, item))
        .collect();
    Ok(ProviderNativeOwnerSearchResponse {
        schema_id: "agent.semantic-protocols.provider-native-owner-search-response",
        schema_version: "1",
        language_id: "rust",
        provider_id: request.provider_id,
        requested_owner_path: request.owner_path,
        requested_projection_mode: request.projection_mode,
        source_content_digest: request.source_fingerprint.content_digest,
        parsed_owner_count: 1,
        projection_completeness: "complete-owner",
        projections,
    })
}

fn validate_request_identity(
    request: &ProviderNativeOwnerSearchRequest,
    activated_provider_id: &str,
) -> Result<(), String> {
    if request.schema_id != "agent.semantic-protocols.provider-native-owner-search-request"
        || request.schema_version != "1"
        || request.language_id != "rust"
        || request.transport != "stdin-json"
        || request.source_encoding != "base64"
    {
        return Err(
            "owner-search stdin request has an unsupported v1 contract identity".to_string(),
        );
    }
    if request.provider_id != activated_provider_id
        || request.workspace_identity.is_empty()
        || !is_digest(&request.provider_workspace_identity_digest)
        || request.owner_path.is_empty()
        || request.owner_path.starts_with('/')
        || request
            .owner_path
            .split('/')
            .any(|component| component == "..")
        || request.projection_mode != "complete-owner"
    {
        return Err("owner-search stdin request identity is invalid".to_string());
    }
    Ok(())
}

fn validate_source_fingerprint(
    request: &ProviderNativeOwnerSearchRequest,
    source_bytes: &[u8],
) -> Result<(), String> {
    let fingerprint = &request.source_fingerprint;
    if fingerprint.file_identity.is_empty()
        || fingerprint.size_bytes != source_bytes.len() as u64
        || !is_digest(&fingerprint.content_digest)
    {
        return Err("owner-search stdin source fingerprint is invalid".to_string());
    }
    let actual_digest = blake3::hash(source_bytes).to_hex().to_string();
    if actual_digest != fingerprint.content_digest {
        return Err(format!(
            "owner-search stdin digest mismatch: expected={} actual={actual_digest}",
            fingerprint.content_digest
        ));
    }
    let _metadata_identity = (
        fingerprint.modified_unix_nanos,
        fingerprint.change_time_unix_nanos,
    );
    Ok(())
}

fn owner_projection(
    owner_path: &str,
    source: &str,
    item: crate::exact_source_parse_artifact::ParseArtifactItem,
) -> Option<ProviderOwnerProjection> {
    let code = source.get(item.source_byte_start..item.source_byte_end)?;
    let signature = item_signature(code);
    if signature.is_empty() {
        return None;
    }
    let structural_selector =
        super::exact_source::rust_structural_selector(owner_path, &item.identity);
    Some(ProviderOwnerProjection {
        canonical_item_selector: agent_semantic_content_identity::CanonicalItemSelector::new(
            item.identity,
            structural_selector,
        ),
        signature,
        capture_name: "declaration.name",
        source_byte_start: item.source_byte_start as u64,
        source_byte_end: item.source_byte_end as u64,
    })
}

fn item_signature(code: &str) -> String {
    let code = code.trim();
    let end = code
        .find('{')
        .or_else(|| code.find(';').map(|index| index + 1))
        .unwrap_or(code.len());
    code.get(..end).unwrap_or_default().trim().to_string()
}

fn provider_id_arg(args: &[std::ffi::OsString]) -> Result<&str, String> {
    args.windows(2)
        .find(|pair| pair[0] == "--asp-provider-id")
        .and_then(|pair| pair[1].to_str())
        .ok_or_else(|| "owner-search stdin requires --asp-provider-id".to_string())
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
#[path = "../../../tests/unit/cli/runner/owner_search.rs"]
mod tests;
