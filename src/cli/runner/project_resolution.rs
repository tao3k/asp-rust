use crate::project_resolution::{
    ProjectResolution, ProjectResolutionCollectionScope, ProjectResolutionError,
    ProjectResolutionInput, resolve_cargo_project_resolution,
};
use serde::{Deserialize, Serialize};
use std::io::Read;
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
    language_id: &'static str,
    provider_id: &'static str,
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
    Failed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderProjectResolutionFailure {
    reason_kind: &'static str,
    message: String,
    next_action: &'static str,
}

pub(super) fn run_project_resolution(_args: &[std::ffi::OsString]) -> Result<ExitCode, String> {
    let mut request_bytes = Vec::new();
    std::io::stdin()
        .lock()
        .read_to_end(&mut request_bytes)
        .map_err(|error| format!("failed to read project-resolution stdin request: {error}"))?;
    let request = serde_json::from_slice::<ProviderProjectResolutionRequest>(&request_bytes)
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
                language_id: "rust",
                provider_id: "rs-harness",
                state: ProviderProjectResolutionState::Resolved,
                scope: Some(resolution),
                failure: None,
            },
            ExitCode::SUCCESS,
        ),
        Err(error) => (
            ProviderProjectResolutionResponse {
                schema_id: RESPONSE_SCHEMA_ID,
                schema_version: "1",
                language_id: "rust",
                provider_id: "rs-harness",
                state: ProviderProjectResolutionState::Failed,
                scope: None,
                failure: Some(failure_from_error(error)),
            },
            ExitCode::from(2),
        ),
    };
    println!(
        "{}",
        serde_json::to_string(&response)
            .map_err(|error| format!("serialize project-resolution response: {error}"))?
    );
    Ok(exit_code)
}

fn validate_request(request: &ProviderProjectResolutionRequest) -> Result<(), String> {
    if request.schema_id != REQUEST_SCHEMA_ID || request.schema_version != "1" {
        return Err(format!(
            "unsupported project-resolution request schema: {}@{}",
            request.schema_id, request.schema_version
        ));
    }
    if request.language_id != "rust" || request.provider_id != "rs-harness" {
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
    {
        if owner_paths.is_empty()
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
                != owner_paths.len()
        {
            return Err(
                "explicit-owners collectionScope requires unique normalized workspace-relative ownerPaths"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn failure_from_error(error: ProjectResolutionError) -> ProviderProjectResolutionFailure {
    let reason_kind = match &error {
        ProjectResolutionError::ProjectEntryMissing { .. } => "project-entry-missing",
        ProjectResolutionError::ProjectEntryInvalid { .. } => "project-entry-invalid",
    };
    ProviderProjectResolutionFailure {
        reason_kind,
        message: error.to_string(),
        next_action: "refresh-project-resolution-candidates-or-select-project-entry",
    }
}
