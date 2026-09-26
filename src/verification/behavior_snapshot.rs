//! Typed `semantic-behavior-snapshot` model for observable Rust behavior.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::RustInvariantId;

/// Shared behavior snapshot schema id.
pub const RUST_BEHAVIOR_SNAPSHOT_SCHEMA_ID: &str =
    "agent.semantic-protocols.semantic-behavior-snapshot";

/// Shared behavior snapshot schema version.
pub const RUST_BEHAVIOR_SNAPSHOT_SCHEMA_VERSION: &str = "1";

/// Shared behavior snapshot protocol id.
pub const RUST_BEHAVIOR_SNAPSHOT_PROTOCOL_ID: &str = "agent.semantic-protocols.behavior-snapshot";

/// Shared behavior snapshot protocol version.
pub const RUST_BEHAVIOR_SNAPSHOT_PROTOCOL_VERSION: &str = "1";

/// Stable behavior snapshot id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotId(pub String);

/// Behavior snapshot schema id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotSchemaId(pub String);

/// Behavior snapshot schema version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotSchemaVersion(pub String);

/// Behavior snapshot protocol id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotProtocolId(pub String);

/// Behavior snapshot protocol version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotProtocolVersion(pub String);

/// Behavior snapshot timestamp.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotObservedAt(pub String);

/// Behavior snapshot producer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustBehaviorSnapshotProducer {
    /// Source language id.
    pub language_id: RustBehaviorSnapshotLanguageId,
    /// Provider id.
    pub provider_id: RustBehaviorSnapshotProviderId,
    /// Provider namespace.
    pub namespace: RustBehaviorSnapshotNamespace,
}

impl Default for RustBehaviorSnapshotProducer {
    fn default() -> Self {
        Self {
            language_id: RustBehaviorSnapshotLanguageId("rust".to_string()),
            provider_id: RustBehaviorSnapshotProviderId("asp-rust".to_string()),
            namespace: RustBehaviorSnapshotNamespace(
                "agent.semantic-protocols.languages.rust.asp-rust".to_string(),
            ),
        }
    }
}

/// Behavior snapshot language id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotLanguageId(pub String);

/// Behavior snapshot provider id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotProviderId(pub String);

/// Behavior snapshot namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotNamespace(pub String);

/// Observable behavior subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustBehaviorSnapshotSubject {
    /// Subject kind.
    pub kind: RustBehaviorSnapshotSubjectKind,
    /// Project-relative subject path.
    pub path: PathBuf,
    /// Optional symbol or API name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<RustBehaviorSnapshotSymbol>,
    /// Command that produced the observable behavior.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    /// Additional subject facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Behavior subject kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustBehaviorSnapshotSubjectKind {
    /// Public API behavior.
    PublicApi,
    /// Function behavior.
    Function,
    /// Method behavior.
    Method,
    /// Module behavior.
    Module,
    /// CLI behavior.
    Cli,
    /// Test behavior.
    Test,
    /// Custom subject.
    Custom,
}

impl FromStr for RustBehaviorSnapshotSubjectKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "public-api" => Ok(Self::PublicApi),
            "function" => Ok(Self::Function),
            "method" => Ok(Self::Method),
            "module" => Ok(Self::Module),
            "cli" => Ok(Self::Cli),
            "test" => Ok(Self::Test),
            "custom" => Ok(Self::Custom),
            other => Err(format!("unknown behavior subject kind: {other}")),
        }
    }
}

/// Behavior snapshot symbol.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotSymbol(pub String);

/// Behavior snapshot status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustBehaviorSnapshotStatus {
    /// Behavior matched the expected snapshot.
    Matched,
    /// Behavior changed relative to the expected snapshot.
    Changed,
    /// Expected snapshot or subject was missing.
    Missing,
    /// Snapshot was intentionally skipped.
    Skipped,
    /// Snapshot could not be produced.
    Error,
}

impl FromStr for RustBehaviorSnapshotStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "matched" => Ok(Self::Matched),
            "changed" => Ok(Self::Changed),
            "missing" => Ok(Self::Missing),
            "skipped" => Ok(Self::Skipped),
            "error" => Ok(Self::Error),
            other => Err(format!("unknown behavior snapshot status: {other}")),
        }
    }
}

/// Behavior observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustBehaviorSnapshotObservation {
    /// Observation kind.
    pub kind: RustBehaviorSnapshotObservationKind,
    /// Compact observation message.
    pub message: RustBehaviorSnapshotObservationMessage,
    /// Optional project-relative path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Optional one-based line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<RustBehaviorSnapshotLine>,
    /// Additional observation facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Behavior observation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustBehaviorSnapshotObservationKind {
    /// Stdout behavior.
    Stdout,
    /// Stderr behavior.
    Stderr,
    /// Return-value behavior.
    ReturnValue,
    /// Snapshot behavior.
    Snapshot,
    /// Diff behavior.
    Diff,
    /// Diagnostic behavior.
    Diagnostic,
    /// General note.
    Note,
}

/// Behavior observation message.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotObservationMessage(pub String);

/// Behavior observation line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotLine(pub u64);

/// Snapshot value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustBehaviorSnapshotValue {
    /// Value format.
    pub format: RustBehaviorSnapshotValueFormat,
    /// Compact value.
    pub value: String,
    /// Optional sha256 digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<RustBehaviorSnapshotSha256>,
    /// Optional artifact URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_uri: Option<RustBehaviorSnapshotArtifactUri>,
}

impl RustBehaviorSnapshotValue {
    /// Construct a text snapshot value.
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            format: RustBehaviorSnapshotValueFormat::Text,
            value: value.into(),
            sha256: None,
            artifact_uri: None,
        }
    }
}

/// Snapshot value format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustBehaviorSnapshotValueFormat {
    /// Plain text snapshot.
    Text,
    /// JSON snapshot.
    Json,
    /// Rust debug snapshot.
    Debug,
    /// Bytes snapshot.
    Bytes,
    /// Unknown format.
    Unknown,
}

/// Snapshot value sha256.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotSha256(pub String);

/// Snapshot value artifact URI.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RustBehaviorSnapshotArtifactUri(pub String);

/// Observable behavior snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustBehaviorSnapshot {
    /// Shared schema id.
    pub schema_id: RustBehaviorSnapshotSchemaId,
    /// Shared schema version.
    pub schema_version: RustBehaviorSnapshotSchemaVersion,
    /// Shared protocol id.
    pub protocol_id: RustBehaviorSnapshotProtocolId,
    /// Shared protocol version.
    pub protocol_version: RustBehaviorSnapshotProtocolVersion,
    /// Stable snapshot id.
    pub snapshot_id: RustBehaviorSnapshotId,
    /// Snapshot producer.
    pub producer: RustBehaviorSnapshotProducer,
    /// Observable subject.
    pub subject: RustBehaviorSnapshotSubject,
    /// Snapshot status.
    pub status: RustBehaviorSnapshotStatus,
    /// Optional observation timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<RustBehaviorSnapshotObservedAt>,
    /// Compact observations.
    pub observations: Vec<RustBehaviorSnapshotObservation>,
    /// Expected behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<RustBehaviorSnapshotValue>,
    /// Actual behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<RustBehaviorSnapshotValue>,
    /// Behavior diff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<RustBehaviorSnapshotValue>,
    /// Linked execution receipt ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_ids: Vec<String>,
    /// Linked invariant candidate ids.
    #[serde(
        default,
        rename = "candidateIds",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub candidate_ids: Vec<RustInvariantId>,
    /// Additional snapshot facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Input for constructing an expect-test behavior snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustBehaviorSnapshotExpectTestInput {
    /// Stable snapshot id.
    pub snapshot_id: RustBehaviorSnapshotId,
    /// Project-relative subject path.
    pub subject_path: PathBuf,
    /// Optional subject symbol.
    pub symbol: Option<RustBehaviorSnapshotSymbol>,
    /// Expected behavior.
    pub expected: RustBehaviorSnapshotValue,
    /// Actual behavior.
    pub actual: RustBehaviorSnapshotValue,
}

/// Input for constructing a behavior snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustBehaviorSnapshotInput {
    /// Stable snapshot id.
    pub snapshot_id: RustBehaviorSnapshotId,
    /// Observable subject.
    pub subject: RustBehaviorSnapshotSubject,
    /// Snapshot status.
    pub status: RustBehaviorSnapshotStatus,
    /// Compact observations.
    pub observations: Vec<RustBehaviorSnapshotObservation>,
    /// Expected behavior.
    pub expected: Option<RustBehaviorSnapshotValue>,
    /// Actual behavior.
    pub actual: Option<RustBehaviorSnapshotValue>,
    /// Behavior diff.
    pub diff: Option<RustBehaviorSnapshotValue>,
}

impl RustBehaviorSnapshot {
    /// Construct a matched expect-test style snapshot.
    pub fn matched_expect_test(input: RustBehaviorSnapshotExpectTestInput) -> Self {
        Self::new(RustBehaviorSnapshotInput {
            snapshot_id: input.snapshot_id,
            subject: RustBehaviorSnapshotSubject {
                kind: RustBehaviorSnapshotSubjectKind::PublicApi,
                path: input.subject_path,
                symbol: input.symbol,
                command: Vec::new(),
                fields: BTreeMap::new(),
            },
            status: RustBehaviorSnapshotStatus::Matched,
            observations: vec![RustBehaviorSnapshotObservation {
                kind: RustBehaviorSnapshotObservationKind::Snapshot,
                message: RustBehaviorSnapshotObservationMessage(
                    "expect-test snapshot matched".to_string(),
                ),
                path: None,
                line: None,
                fields: BTreeMap::new(),
            }],
            expected: Some(input.expected),
            actual: Some(input.actual),
            diff: None,
        })
    }

    /// Construct a behavior snapshot.
    pub fn new(input: RustBehaviorSnapshotInput) -> Self {
        Self {
            schema_id: RustBehaviorSnapshotSchemaId(RUST_BEHAVIOR_SNAPSHOT_SCHEMA_ID.to_string()),
            schema_version: RustBehaviorSnapshotSchemaVersion(
                RUST_BEHAVIOR_SNAPSHOT_SCHEMA_VERSION.to_string(),
            ),
            protocol_id: RustBehaviorSnapshotProtocolId(
                RUST_BEHAVIOR_SNAPSHOT_PROTOCOL_ID.to_string(),
            ),
            protocol_version: RustBehaviorSnapshotProtocolVersion(
                RUST_BEHAVIOR_SNAPSHOT_PROTOCOL_VERSION.to_string(),
            ),
            snapshot_id: input.snapshot_id,
            producer: RustBehaviorSnapshotProducer::default(),
            subject: input.subject,
            status: input.status,
            observed_at: None,
            observations: input.observations,
            expected: input.expected,
            actual: input.actual,
            diff: input.diff,
            receipt_ids: Vec::new(),
            candidate_ids: Vec::new(),
            fields: BTreeMap::new(),
        }
    }
}
