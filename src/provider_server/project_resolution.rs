use crate::project_resolution::{
    ProjectResolution, ProjectResolutionCollectionScope, ProjectResolutionError,
    ProjectResolutionInput, resolve_cargo_project_resolution,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::ExitCode;

const REQUEST_SCHEMA_ID: &str = "agent.semantic-protocols.provider-project-resolution-request";
const RESPONSE_SCHEMA_ID: &str = "agent.semantic-protocols.provider-project-resolution-response";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderProjectResolutionRequest {
    schema_id: String,
    schema_version: String,
    language_id: String,
    provider_id: String,
    candidate_base: String,
    #[serde(flatten)]
    input: ProjectResolutionInput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderProjectResolutionResponse {
    schema_id: &'static str,
    schema_version: &'static str,
    language_id: String,
    provider_id: String,
    state: ProviderProjectResolutionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<ProjectResolution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<ProviderProjectResolutionFailure>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ProviderProjectResolutionState {
    Resolved,
    NotApplicable,
    Failed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderProjectResolutionFailure {
    reason_kind: &'static str,
    message: String,
    next_action: &'static str,
}
pub(crate) fn handle_project_resolution_request(
    request_bytes: &[u8],
) -> Result<(Vec<u8>, ExitCode), String> {
    let request = serde_json::from_slice::<ProviderProjectResolutionRequest>(request_bytes)
        .map_err(|error| format!("parse project-resolution stdin request: {error}"))?;
    validate_request(&request)?;
    let (response, exit_code) = match resolve_cargo_project_resolution(
        PathBuf::from(&request.candidate_base).as_path(),
        &request.input,
    ) {
        Ok(resolution) => (
            ProviderProjectResolutionResponse {
                schema_id: RESPONSE_SCHEMA_ID,
                schema_version: "1",
                language_id: request.language_id.clone(),
                provider_id: request.provider_id.clone(),
                state: ProviderProjectResolutionState::Resolved,
                scope: Some(resolution),
                failure: None,
            },
            ExitCode::SUCCESS,
        ),
        Err(crate::project_resolution::ProjectResolutionError::NotApplicable { .. }) => (
            ProviderProjectResolutionResponse {
                schema_id: RESPONSE_SCHEMA_ID,
                schema_version: "1",
                language_id: request.language_id.clone(),
                provider_id: request.provider_id.clone(),
                state: ProviderProjectResolutionState::NotApplicable,
                scope: None,
                failure: None,
            },
            ExitCode::SUCCESS,
        ),
        Err(error) => (
            ProviderProjectResolutionResponse {
                schema_id: RESPONSE_SCHEMA_ID,
                schema_version: "1",
                language_id: request.language_id.clone(),
                provider_id: request.provider_id.clone(),
                state: ProviderProjectResolutionState::Failed,
                scope: None,
                failure: Some(failure_from_error(error)),
            },
            ExitCode::from(2),
        ),
    };
    let response = serde_json::to_vec(&response)
        .map_err(|error| format!("serialize project-resolution response: {error}"))?;
    Ok((response, exit_code))
}

pub(super) fn handle_project_resolution_request_value(
    request: &serde_json::Value,
) -> Result<(Vec<u8>, ExitCode), String> {
    let bytes = serde_json::to_vec(request)
        .map_err(|error| format!("encode structured project-resolution request: {error}"))?;
    handle_project_resolution_request(&bytes)
}

fn validate_request(request: &ProviderProjectResolutionRequest) -> Result<(), String> {
    if request.schema_id != REQUEST_SCHEMA_ID || request.schema_version != "1" {
        return Err(format!(
            "unsupported project-resolution request schema: {}@{}",
            request.schema_id, request.schema_version
        ));
    }
    if request.language_id != "rust" || request.provider_id != "asp-rust" {
        return Err(format!(
            "project-resolution provider mismatch: language={} provider={}",
            request.language_id, request.provider_id
        ));
    }
    if request.candidate_base != "." || request.input.candidate_generation.digest.is_empty() {
        return Err(
            "project-resolution requires candidateBase=. and candidateGeneration.digest"
                .to_string(),
        );
    }
    if let ProjectResolutionCollectionScope::ExplicitOwners { owner_paths } =
        &request.input.collection_scope
        && (owner_paths.is_empty()
            || owner_paths.iter().any(|path| {
                path.is_absolute()
                    || path
                        .components()
                        .any(|component| !matches!(component, std::path::Component::Normal(_)))
            })
            || owner_paths
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != owner_paths.len())
    {
        return Err(
            "explicit-owners collectionScope requires unique normalized workspace-relative ownerPaths"
                .to_owned(),
        );
    }
    Ok(())
}

fn failure_from_error(error: ProjectResolutionError) -> ProviderProjectResolutionFailure {
    let reason_kind = match &error {
        ProjectResolutionError::NotApplicable { .. } => "provider-not-applicable",
        ProjectResolutionError::ProjectEntryMissing { .. } => "project-entry-missing",
        ProjectResolutionError::ProjectEntryInvalid { .. } => "project-entry-invalid",
    };
    ProviderProjectResolutionFailure {
        reason_kind,
        message: error.to_string(),
        next_action: "refresh-project-resolution-candidates-or-select-project-entry",
    }
}
