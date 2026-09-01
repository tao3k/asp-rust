//! Stable V1 wire values consumed by the standalone Rust provider.

use serde::{Deserialize, Serialize};

pub(super) const PROVIDER_RUNTIME_REQUEST_FRAME_SCHEMA_ID: &str =
    "agent.semantic-protocols.provider-runtime-request-frame";
pub(super) const PROVIDER_RUNTIME_RESPONSE_FRAME_SCHEMA_ID: &str =
    "agent.semantic-protocols.provider-runtime-response-frame";
pub(super) const PROVIDER_RUNTIME_FRAME_SCHEMA_VERSION: &str = "1";
pub(super) const PROVIDER_SYNTAX_QUERY_REQUEST_SCHEMA_ID: &str =
    "agent.semantic-protocols.provider-syntax-query-request";
pub(super) const PROVIDER_SYNTAX_QUERY_RESPONSE_SCHEMA_ID: &str =
    "agent.semantic-protocols.provider-syntax-query-response";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProviderSchemaReference {
    pub(super) schema_id: String,
    pub(super) schema_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProviderRuntimeContractOperation {
    pub(super) operation: String,
    pub(super) request_schema: ProviderSchemaReference,
    pub(super) response_schema: ProviderSchemaReference,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProviderRuntimeRequestFrame {
    pub(super) schema_id: String,
    pub(super) schema_version: String,
    pub(super) request_id: String,
    pub(super) operation: String,
    pub(super) payload: serde_json::Value,
}

impl ProviderRuntimeRequestFrame {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.schema_id != PROVIDER_RUNTIME_REQUEST_FRAME_SCHEMA_ID
            || self.schema_version != PROVIDER_RUNTIME_FRAME_SCHEMA_VERSION
        {
            return Err("provider runtime request frame schema identity drift".to_owned());
        }
        if self.request_id.trim().is_empty() || self.operation.trim().is_empty() {
            return Err(
                "provider runtime request frame requires requestId and operation".to_owned(),
            );
        }
        if !self.payload.is_object() {
            return Err("provider runtime request payload must be a JSON object".to_owned());
        }
        Ok(())
    }

    pub(super) fn payload_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&self.payload)
            .map_err(|error| format!("encode provider runtime request payload: {error}"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ProviderRuntimeResponseOutcome {
    Ready,
    Error,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProviderRuntimeResponseFrame {
    pub(super) schema_id: String,
    pub(super) schema_version: String,
    pub(super) request_id: String,
    pub(super) outcome: ProviderRuntimeResponseOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) payload: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

impl ProviderRuntimeResponseFrame {
    pub(super) fn ready(request_id: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            schema_id: PROVIDER_RUNTIME_RESPONSE_FRAME_SCHEMA_ID.to_owned(),
            schema_version: PROVIDER_RUNTIME_FRAME_SCHEMA_VERSION.to_owned(),
            request_id: request_id.into(),
            outcome: ProviderRuntimeResponseOutcome::Ready,
            payload: Some(payload),
            error: None,
        }
    }

    pub(super) fn error(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            schema_id: PROVIDER_RUNTIME_RESPONSE_FRAME_SCHEMA_ID.to_owned(),
            schema_version: PROVIDER_RUNTIME_FRAME_SCHEMA_VERSION.to_owned(),
            request_id: request_id.into(),
            outcome: ProviderRuntimeResponseOutcome::Error,
            payload: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SyntaxQueryPlan {
    pub(super) patterns: Vec<SyntaxQueryPattern>,
    pub(super) captures: Vec<String>,
    pub(super) node_types: Vec<String>,
    pub(super) fields: Vec<String>,
    pub(super) predicates: Vec<SyntaxQueryPredicate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SyntaxQueryPattern {
    pub(super) index: usize,
    pub(super) captures: Vec<String>,
    pub(super) node_types: Vec<String>,
    pub(super) fields: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum SyntaxQueryPredicateOp {
    Eq,
    AnyEq,
    AnyOf,
    Match,
    AnyMatch,
    NotEq,
    NotMatch,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub(super) enum SyntaxQueryPredicateValue {
    String(String),
    Capture(String),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SyntaxQueryPredicate {
    pub(super) op: SyntaxQueryPredicateOp,
    pub(super) capture: String,
    pub(super) values: Vec<SyntaxQueryPredicateValue>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProviderSyntaxQueryRequest {
    pub(super) schema_id: String,
    pub(super) schema_version: String,
    pub(super) language_id: String,
    pub(super) provider_id: String,
    pub(super) owner_path: String,
    pub(super) source_content_digest: String,
    pub(super) query_digest: String,
    pub(super) source: String,
    pub(super) plan: SyntaxQueryPlan,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProviderSyntaxQueryResponse {
    pub(super) schema_id: String,
    pub(super) schema_version: String,
    pub(super) language_id: String,
    pub(super) provider_id: String,
    pub(super) owner_path: String,
    pub(super) source_content_digest: String,
    pub(super) query_digest: String,
    pub(super) parsed: bool,
    pub(super) captures: Vec<ProviderSyntaxQueryCapture>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProviderSyntaxQueryCapture {
    pub(super) pattern_index: usize,
    pub(super) capture_name: String,
    pub(super) native_fact_ref: String,
    pub(super) source_byte_start: u64,
    pub(super) source_byte_end: u64,
}
