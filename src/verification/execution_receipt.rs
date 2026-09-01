//! Executable verification receipt adapters.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::RustInvariantId;

/// Shared semantic verification receipt schema id.
pub const RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID: &str =
    "agent.semantic-protocols.semantic-verification-receipt";

/// Shared semantic verification receipt schema version.
pub const RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_VERSION: &str = "1";

/// Shared semantic verification receipt protocol id.
pub const RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_ID: &str =
    "agent.semantic-protocols.verification-receipt";

/// Shared semantic verification receipt protocol version.
pub const RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_VERSION: &str = "1";

/// Stable execution receipt id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionReceiptId(pub String);

impl RustVerificationExecutionReceiptId {
    /// Return the id as a borrowed string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Verification tool adapter id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionAdapterId(pub String);

/// Execution receipt schema id value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionSchemaId(pub String);

/// Execution receipt schema version value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionSchemaVersion(pub String);

/// Execution receipt protocol id value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionProtocolId(pub String);

/// Execution receipt protocol version value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionProtocolVersion(pub String);

/// Execution result exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionExitCode(pub i32);

/// Execution duration in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionDurationMs(pub u64);

/// Execution receipt summary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionSummary(pub String);

/// Optional observed-at timestamp supplied by the tool runner.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionObservedAt(pub String);

/// Verification task fingerprint linked to an execution receipt.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionTaskFingerprint(pub String);

/// Verification receipt producer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustVerificationExecutionProducer {
    /// Source language id.
    pub language_id: RustVerificationExecutionLanguageId,
    /// Provider id.
    pub provider_id: RustVerificationExecutionProviderId,
    /// Tool adapter id.
    pub adapter_id: RustVerificationExecutionAdapterId,
    /// Provider namespace.
    pub namespace: RustVerificationExecutionNamespace,
}

/// Source language id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionLanguageId(pub String);

/// Provider id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionProviderId(pub String);

/// Provider namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionNamespace(pub String);

impl Default for RustVerificationExecutionProducer {
    fn default() -> Self {
        Self {
            language_id: RustVerificationExecutionLanguageId("rust".to_owned()),
            provider_id: RustVerificationExecutionProviderId("asp-rust".to_owned()),
            adapter_id: RustVerificationExecutionAdapterId("rust.unknown".to_owned()),
            namespace: RustVerificationExecutionNamespace(
                "agent.semantic-protocols.languages.rust.asp-rust".to_owned(),
            ),
        }
    }
}

/// Project context for an execution receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustVerificationExecutionProject {
    /// Project display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<RustVerificationExecutionProjectName>,
    /// Working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<PathBuf>,
    /// Cargo package name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<RustVerificationExecutionPackageName>,
}

/// Project display name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionProjectName(pub String);

/// Package display name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionPackageName(pub String);

/// Verification execution tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustVerificationExecutionTool {
    /// `cargo check`.
    CargoCheck,
    /// `cargo test`.
    CargoTest,
    /// `cargo clippy`.
    Clippy,
    /// Rust `expect_test` snapshot test run.
    ExpectTest,
    /// `proptest` test run.
    Proptest,
    /// `cargo fuzz`.
    CargoFuzz,
    /// Kani proof run.
    Kani,
    /// Creusot proof run.
    Creusot,
    /// Verus proof run.
    Verus,
}

/// Execution receipt status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustVerificationExecutionStatus {
    /// Tool completed successfully.
    Passed,
    /// Tool completed and reported a failure.
    Failed,
    /// Tool was intentionally skipped.
    Skipped,
    /// Tool could not produce normal evidence.
    Error,
}

/// Command output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustVerificationExecutionOutputFormat {
    /// Cargo JSON messages.
    CargoJson,
    /// Plain text output.
    PlainText,
    /// libtest output.
    Libtest,
    /// `expect_test` snapshot output.
    ExpectTest,
    /// Fuzz corpus or crash artifact output.
    FuzzCorpus,
    /// Proof report output.
    ProofReport,
    /// Unknown output format.
    Unknown,
}

/// Executable command captured by a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustVerificationExecutionCommand {
    /// Executable argv.
    pub argv: Vec<String>,
    /// Command working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<PathBuf>,
    /// Environment variables that materially affect the adapter.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Adapter output format.
    pub output_format: RustVerificationExecutionOutputFormat,
}

/// Observation row attached to an execution receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustVerificationExecutionObservation {
    /// Observation kind.
    pub kind: RustVerificationExecutionObservationKind,
    /// Compact observation message.
    pub message: RustVerificationExecutionObservationMessage,
    /// Optional project-relative path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Optional one-based line number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<RustVerificationExecutionLine>,
    /// Additional adapter facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Execution observation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustVerificationExecutionObservationKind {
    /// Process exit status.
    ExitStatus,
    /// Stdout summary.
    Stdout,
    /// Stderr summary.
    Stderr,
    /// Compiler or lint diagnostic.
    Diagnostic,
    /// Test result row.
    TestResult,
    /// Snapshot diff.
    SnapshotDiff,
    /// Coverage fact.
    Coverage,
    /// Artifact fact.
    Artifact,
    /// General note.
    Note,
}

/// Execution observation message.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionObservationMessage(pub String);

/// One-based source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionLine(pub u64);

/// Artifact reference attached to a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustVerificationExecutionArtifact {
    /// Artifact kind.
    pub kind: RustVerificationExecutionArtifactKind,
    /// Artifact URI or project-local path.
    pub uri: RustVerificationExecutionArtifactUri,
    /// Optional SHA-256 digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<RustVerificationExecutionSha256>,
    /// Additional artifact facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Execution artifact kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustVerificationExecutionArtifactKind {
    /// Captured stdout.
    Stdout,
    /// Captured stderr.
    Stderr,
    /// Cargo JSON output.
    CargoJson,
    /// Expect snapshot artifact.
    ExpectSnapshot,
    /// Coverage report.
    CoverageReport,
    /// Proof log.
    ProofLog,
    /// Fuzz artifact.
    FuzzArtifact,
    /// Other artifact.
    Other,
}

/// Artifact URI.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionArtifactUri(pub String);

/// Artifact SHA-256 digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustVerificationExecutionSha256(pub String);

/// Executable verification receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustVerificationExecutionReceipt {
    /// Shared schema id.
    pub schema_id: RustVerificationExecutionSchemaId,
    /// Shared schema version.
    pub schema_version: RustVerificationExecutionSchemaVersion,
    /// Shared protocol id.
    pub protocol_id: RustVerificationExecutionProtocolId,
    /// Shared protocol version.
    pub protocol_version: RustVerificationExecutionProtocolVersion,
    /// Stable receipt id.
    pub receipt_id: RustVerificationExecutionReceiptId,
    /// Receipt producer.
    pub producer: RustVerificationExecutionProducer,
    /// Optional project context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<RustVerificationExecutionProject>,
    /// Tool adapter.
    pub tool: RustVerificationExecutionTool,
    /// Execution status.
    pub status: RustVerificationExecutionStatus,
    /// Captured command.
    pub command: RustVerificationExecutionCommand,
    /// Process exit code, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<RustVerificationExecutionExitCode>,
    /// Execution duration, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<RustVerificationExecutionDurationMs>,
    /// Tool-run timestamp, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<RustVerificationExecutionObservedAt>,
    /// Compact receipt summary.
    pub summary: RustVerificationExecutionSummary,
    /// Compact observations.
    pub observations: Vec<RustVerificationExecutionObservation>,
    /// Linked invariant candidates.
    #[serde(
        default,
        rename = "candidateIds",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub candidate_ids: Vec<RustInvariantId>,
    /// Linked lifecycle task fingerprints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_fingerprints: Vec<RustVerificationExecutionTaskFingerprint>,
    /// Persisted artifacts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<RustVerificationExecutionArtifact>,
    /// Additional receipt facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

impl RustVerificationExecutionReceipt {
    /// Build a receipt from a known tool adapter and process exit code.
    #[must_use]
    pub fn from_exit_code(
        receipt_id: RustVerificationExecutionReceiptId,
        adapter: RustVerificationToolAdapter,
        exit_code: RustVerificationExecutionExitCode,
        summary: RustVerificationExecutionSummary,
    ) -> Self {
        let status = if exit_code.0 == 0 {
            RustVerificationExecutionStatus::Passed
        } else {
            RustVerificationExecutionStatus::Failed
        };
        let producer = RustVerificationExecutionProducer {
            adapter_id: adapter.adapter_id(),
            ..Default::default()
        };
        Self {
            schema_id: RustVerificationExecutionSchemaId(
                RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID.to_owned(),
            ),
            schema_version: RustVerificationExecutionSchemaVersion(
                RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_VERSION.to_owned(),
            ),
            protocol_id: RustVerificationExecutionProtocolId(
                RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_ID.to_owned(),
            ),
            protocol_version: RustVerificationExecutionProtocolVersion(
                RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_VERSION.to_owned(),
            ),
            receipt_id,
            producer,
            project: None,
            tool: adapter.tool(),
            status,
            command: adapter.default_command(),
            exit_code: Some(exit_code),
            duration_ms: None,
            observed_at: None,
            summary,
            observations: vec![RustVerificationExecutionObservation {
                kind: RustVerificationExecutionObservationKind::ExitStatus,
                message: RustVerificationExecutionObservationMessage(format!(
                    "exit code {}",
                    exit_code.0
                )),
                path: None,
                line: None,
                fields: BTreeMap::new(),
            }],
            candidate_ids: Vec::new(),
            task_fingerprints: Vec::new(),
            artifacts: Vec::new(),
            fields: BTreeMap::new(),
        }
    }
}

/// Rust verification tool adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustVerificationToolAdapter {
    /// `cargo check --message-format=json`.
    CargoCheck,
    /// `cargo test --no-fail-fast`.
    CargoTest,
    /// `cargo clippy --message-format=json`.
    Clippy,
    /// `cargo test` with `expect_test` snapshot semantics.
    ExpectTest,
    /// Property-based tests through libtest.
    Proptest,
    /// `cargo fuzz run` libFuzzer adapter.
    CargoFuzz,
    /// Future Kani adapter.
    Kani,
    /// Future Creusot adapter.
    Creusot,
    /// Future Verus adapter.
    Verus,
}

impl std::str::FromStr for RustVerificationToolAdapter {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "cargo-check" => Ok(Self::CargoCheck),
            "cargo-test" => Ok(Self::CargoTest),
            "clippy" => Ok(Self::Clippy),
            "expect-test" => Ok(Self::ExpectTest),
            "proptest" => Ok(Self::Proptest),
            "cargo-fuzz" => Ok(Self::CargoFuzz),
            "kani" => Ok(Self::Kani),
            "creusot" => Ok(Self::Creusot),
            "verus" => Ok(Self::Verus),
            other => Err(format!("unknown verification receipt adapter: {other}")),
        }
    }
}

impl RustVerificationToolAdapter {
    /// Tool emitted by this adapter.
    #[must_use]
    pub fn tool(self) -> RustVerificationExecutionTool {
        match self {
            Self::CargoCheck => RustVerificationExecutionTool::CargoCheck,
            Self::CargoTest => RustVerificationExecutionTool::CargoTest,
            Self::Clippy => RustVerificationExecutionTool::Clippy,
            Self::ExpectTest => RustVerificationExecutionTool::ExpectTest,
            Self::Proptest => RustVerificationExecutionTool::Proptest,
            Self::CargoFuzz => RustVerificationExecutionTool::CargoFuzz,
            Self::Kani => RustVerificationExecutionTool::Kani,
            Self::Creusot => RustVerificationExecutionTool::Creusot,
            Self::Verus => RustVerificationExecutionTool::Verus,
        }
    }

    /// Stable adapter id.
    #[must_use]
    pub fn adapter_id(self) -> RustVerificationExecutionAdapterId {
        let id = match self {
            Self::CargoCheck => "rust.cargo-check",
            Self::CargoTest => "rust.cargo-test",
            Self::Clippy => "rust.clippy",
            Self::ExpectTest => "rust.expect-test",
            Self::Proptest => "rust.proptest",
            Self::CargoFuzz => "rust.cargo-fuzz",
            Self::Kani => "rust.kani",
            Self::Creusot => "rust.creusot",
            Self::Verus => "rust.verus",
        };
        RustVerificationExecutionAdapterId(id.to_owned())
    }

    /// Default command for this adapter.
    #[must_use]
    pub fn default_command(self) -> RustVerificationExecutionCommand {
        RustVerificationExecutionCommand {
            argv: self.default_argv(),
            workdir: None,
            env: BTreeMap::new(),
            output_format: self.output_format(),
        }
    }

    /// Default argv for this adapter.
    #[must_use]
    pub fn default_argv(self) -> Vec<String> {
        match self {
            Self::CargoCheck => vec!["cargo", "check", "--message-format=json"],
            Self::CargoTest => vec!["cargo", "test", "--no-fail-fast"],
            Self::Clippy => vec!["cargo", "clippy", "--message-format=json"],
            Self::ExpectTest => vec!["cargo", "test"],
            Self::Proptest => vec!["cargo", "test", "--all-targets", "--", "--nocapture"],
            Self::CargoFuzz => vec!["cargo", "fuzz", "run"],
            Self::Kani => vec!["cargo", "kani"],
            Self::Creusot => vec!["cargo", "creusot"],
            Self::Verus => vec!["verus"],
        }
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    /// Expected output format for this adapter.
    #[must_use]
    pub fn output_format(self) -> RustVerificationExecutionOutputFormat {
        match self {
            Self::CargoCheck | Self::Clippy => RustVerificationExecutionOutputFormat::CargoJson,
            Self::CargoTest | Self::Proptest => RustVerificationExecutionOutputFormat::Libtest,
            Self::ExpectTest => RustVerificationExecutionOutputFormat::ExpectTest,
            Self::CargoFuzz => RustVerificationExecutionOutputFormat::FuzzCorpus,
            Self::Kani | Self::Creusot | Self::Verus => {
                RustVerificationExecutionOutputFormat::ProofReport
            }
        }
    }
}
