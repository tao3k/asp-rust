//! Runtime-managed process and transport boundary for the ASP Rust provider.
//!
//! This module admits only the catalog-declared `serve` process surface and
//! dispatches typed provider operations; it does not own a second business CLI.

use std::ffi::OsString;
use std::process::ExitCode;
use std::sync::Arc;

use super::contract::{
    ProviderRuntimeContractOperation, ProviderRuntimeRequestFrame, ProviderRuntimeResponseFrame,
    ProviderSchemaReference,
};
use super::http_json::{
    HttpJsonRequest as AspClientServerRequest, HttpJsonResponse as AspClientServerResponse,
    serve_http_json as serve_asp_client_server,
};

/// Run the sole catalog-declared provider entrypoint.
///
/// The provider binary is not a second ASP CLI. Its process argument contract
/// is exactly `serve`; all business operations arrive as typed HTTP/JSON
/// request frames from the ASP Server.
pub fn run_provider_server_from_env() -> ExitCode {
    match admit_provider_server_argv(std::env::args_os().skip(1))
        .and_then(|()| run_provider_server())
    {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn admit_provider_server_argv(args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args.len() == 1 && args[0] == "serve" {
        return Ok(());
    }
    Err(
        "asp-rust is a Runtime-managed provider server; the only admitted process argument is `serve`"
            .to_owned(),
    )
}

fn run_provider_server() -> Result<ExitCode, String> {
    let artifact_digest = required_env("ASP_PROVIDER_ARTIFACT_DIGEST")?;
    let registration_digest = required_env("ASP_PROVIDER_REGISTRATION_DIGEST")?;
    let contract_digest = required_env("ASP_PROVIDER_RUNTIME_CONTRACT_DIGEST")?;
    let provider_id = required_env("ASP_PROVIDER_ID")?;
    let language_id = required_env("ASP_PROVIDER_LANGUAGE_ID")?;
    let host = required_env("ASP_CLIENT_SERVER_HOST")?;
    let operations = runtime_contract_operations();
    let health = Arc::new(serde_json::json!({
        "schemaId": "agent.semantic-protocols.provider-runtime-contract-receipt",
        "schemaVersion": "1",
        "providerId": provider_id,
        "languageId": language_id,
        "artifactDigest": artifact_digest,
        "registrationDigest": registration_digest,
        "contractDigest": contract_digest,
        "transport": "http-json",
        "operations": operations,
    }));
    run_provider_runtime(async move {
        let listener = tokio::net::TcpListener::bind(&host)
            .await
            .map_err(|error| format!("bind asp-client-server {host}: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("read asp-client-server bound address: {error}"))?;
        let bootstrap = serde_json::to_string(&serde_json::json!({
            "schemaId": "agent.semantic-protocols.asp-client-server-bootstrap",
            "schemaVersion": "1",
            "transport": "http-json",
            "state": "ready",
            "endpoint": format!("http://{address}/"),
        }))
        .map_err(|error| format!("encode asp-client-server bootstrap: {error}"))?;
        println!("{bootstrap}");
        std::io::Write::flush(&mut std::io::stdout())
            .map_err(|error| format!("flush asp-client-server bootstrap: {error}"))?;
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        serve_asp_client_server(listener, shutdown_rx, move |request| {
            let health = Arc::clone(&health);
            let shutdown_tx = shutdown_tx.clone();
            async move { handle_http_request(request, health, shutdown_tx) }
        })
        .await
    })?;
    Ok(ExitCode::SUCCESS)
}

fn run_provider_runtime<F>(server: F) -> Result<(), String>
where
    F: std::future::Future<Output = Result<(), String>> + Send + 'static,
{
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("build provider Tokio runtime: {error}"))?
        .block_on(server)
}

fn runtime_contract_operations() -> Vec<ProviderRuntimeContractOperation> {
    [
        (
            "syntax-query",
            "agent.semantic-protocols.provider-syntax-query-request",
            "agent.semantic-protocols.provider-syntax-query-response",
        ),
        (
            "projection-batch",
            "agent.semantic-protocols.provider-language-projection-batch-request",
            "agent.semantic-protocols.provider-language-projection-batch-response",
        ),
        (
            "project-resolution",
            "agent.semantic-protocols.provider-project-resolution-request",
            "agent.semantic-protocols.provider-project-resolution-response",
        ),
    ]
    .into_iter()
    .map(
        |(operation, request_schema_id, response_schema_id)| ProviderRuntimeContractOperation {
            operation: operation.to_owned(),
            request_schema: ProviderSchemaReference {
                schema_id: request_schema_id.to_owned(),
                schema_version: "1".to_owned(),
            },
            response_schema: ProviderSchemaReference {
                schema_id: response_schema_id.to_owned(),
                schema_version: "1".to_owned(),
            },
        },
    )
    .collect()
}

fn handle_http_request(
    request: AspClientServerRequest,
    health: Arc<serde_json::Value>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
) -> Result<AspClientServerResponse, String> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => AspClientServerResponse::json(200, &health),
        ("POST", "/v1/provider-runtime") => {
            let request = serde_json::from_slice::<ProviderRuntimeRequestFrame>(&request.body)
                .map_err(|error| format!("decode asp-client-server request: {error}"))?;
            let response = serde_json::to_value(handle_request(request))
                .map_err(|error| format!("encode asp-client-server response: {error}"))?;
            AspClientServerResponse::json(200, &response)
        }
        ("POST", "/shutdown") => {
            shutdown_tx
                .send(true)
                .map_err(|_| "asp-client-server shutdown receiver closed".to_owned())?;
            AspClientServerResponse::json(200, &serde_json::json!({ "state": "draining" }))
        }
        _ => AspClientServerResponse::json(
            404,
            &serde_json::json!({ "error": "asp-client-server-route-not-found" }),
        ),
    }
}

fn handle_request(request: ProviderRuntimeRequestFrame) -> ProviderRuntimeResponseFrame {
    let request_id = request.request_id.clone();
    let result = request
        .validate()
        .and_then(|()| request.payload_bytes())
        .and_then(|payload| {
            let payload: serde_json::Value = serde_json::from_slice(&payload)
                .map_err(|error| format!("decode provider runtime operation payload: {error}"))?;
            match request.operation.as_str() {
                "syntax-query" => {
                    super::syntax_query::handle_syntax_query_operation_value(&payload)
                }
                "projection-batch" => {
                    super::projection::handle_language_projection_batch_value(&payload)
                }
                "project-resolution" => {
                    super::project_resolution::handle_project_resolution_request_value(&payload)
                        .map(|(response, _)| response)
                }
                operation => Err(format!(
                    "provider runtime operation is not admitted: {operation}"
                )),
            }
        });
    match result {
        Ok(payload) => match serde_json::from_slice(&payload) {
            Ok(payload) => ProviderRuntimeResponseFrame::ready(request_id, payload),
            Err(error) => ProviderRuntimeResponseFrame::error(
                request_id,
                format!("provider operation returned non-JSON payload: {error}"),
            ),
        },
        Err(error) => ProviderRuntimeResponseFrame::error(request_id, error),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/provider_server_runtime.rs"]
mod tests;

fn required_env(key: &str) -> Result<String, String> {
    std::env::var(key).map_err(|error| format!("provider runtime requires {key}: {error}"))
}
