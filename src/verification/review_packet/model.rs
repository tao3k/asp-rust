//! Typed `ReviewPacket` data model and protocol constants.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    AspRustReport, RustBehaviorSnapshot, RustDeterminismReadiness, RustFormalProofPilot,
    RustVerificationExecutionReceipt,
};

/// Shared `ReviewPacket` schema id.
pub const RUST_REVIEW_PACKET_SCHEMA_ID: &str = "agent.semantic-protocols.semantic-review-packet";
/// Shared `ReviewPacket` schema version.
pub const RUST_REVIEW_PACKET_SCHEMA_VERSION: &str = "1";
/// Shared `ReviewPacket` protocol id.
pub const RUST_REVIEW_PACKET_PROTOCOL_ID: &str = "agent.semantic-protocols.review-packet";
/// Shared `ReviewPacket` protocol version.
pub const RUST_REVIEW_PACKET_PROTOCOL_VERSION: &str = "1";

/// Review packet schema id newtype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct RustReviewPacketSchemaId(pub(crate) String);

/// Review packet schema version newtype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct RustReviewPacketSchemaVersion(pub(crate) String);

/// Review packet protocol id newtype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct RustReviewPacketProtocolId(pub(crate) String);

/// Review packet protocol version newtype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct RustReviewPacketProtocolVersion(pub(crate) String);

/// Review packet id newtype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct RustReviewPacketId(pub(crate) String);

/// Input facts for a new review packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustReviewPacketInput {
    /// Project root used for path normalization.
    pub project_root: PathBuf,
    /// Current harness report carrying invariant candidates.
    pub report: AspRustReport,
    /// Executable verification receipts.
    pub receipts: Vec<RustVerificationExecutionReceipt>,
    /// Observable behavior snapshots.
    pub behavior_snapshots: Vec<RustBehaviorSnapshot>,
    /// Determinism readiness packets.
    pub determinism_readiness: Vec<RustDeterminismReadiness>,
    /// Formal proof pilot packets.
    pub proof_pilots: Vec<RustFormalProofPilot>,
    /// New review-packet waiver evidence.
    pub waivers: Vec<RustReviewPacketWaiver>,
}

/// Reviewer-first packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustReviewPacket {
    /// Shared schema id.
    pub(super) schema_id: RustReviewPacketSchemaId,
    /// Shared schema version.
    pub(super) schema_version: RustReviewPacketSchemaVersion,
    /// Shared protocol id.
    pub(super) protocol_id: RustReviewPacketProtocolId,
    /// Shared protocol version.
    pub(super) protocol_version: RustReviewPacketProtocolVersion,
    /// Stable packet id.
    pub(super) packet_id: RustReviewPacketId,
    /// Producer metadata.
    pub(super) producer: RustReviewPacketProducer,
    /// Project metadata.
    pub(super) project: RustReviewPacketProject,
    /// Compact counters.
    pub(super) summary: RustReviewPacketSummary,
    /// Changed invariant candidates.
    pub(super) changed_invariants: Vec<RustReviewPacketChangedInvariant>,
    /// Behavior snapshots that need review.
    pub(super) changed_behavior: Vec<RustReviewPacketChangedBehavior>,
    /// Required receipts not discharged by new evidence.
    pub(super) missing_receipts: Vec<RustReviewPacketMissingReceipt>,
    /// Stale or expired waiver evidence.
    pub(super) stale_waivers: Vec<RustReviewPacketWaiver>,
    /// Determinism readiness summaries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) determinism_readiness: Vec<RustReviewPacketDeterminismSummary>,
    /// Proof pilot summaries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) proof_pilots: Vec<RustReviewPacketProofSummary>,
    /// Reviewer-first actions.
    pub(super) review_actions: Vec<RustReviewPacketAction>,
    /// Additional provider-owned fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// Review packet producer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketProducer {
    pub language_id: String,
    pub provider_id: String,
    pub namespace: String,
}

/// Review packet project metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketProject {
    pub root: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// Review packet counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketSummary {
    pub(super) changed_invariants: usize,
    pub(super) changed_behavior: usize,
    pub(super) missing_receipts: usize,
    pub(super) stale_waivers: usize,
    pub determinism_observations: usize,
    pub proof_claims: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// Changed invariant summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketChangedInvariant {
    pub invariant_id: String,
    pub source_rule_id: String,
    pub kind: RustReviewPacketInvariantKind,
    pub severity: RustReviewPacketSeverity,
    pub title: String,
    pub hypothesis: String,
    pub location: RustReviewPacketLocation,
    pub required_receipts: Vec<RustReviewPacketReceiptKind>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// Review packet source location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketLocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    pub line: usize,
    pub column: usize,
}

/// Review packet severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum RustReviewPacketSeverity {
    Info,
    Warning,
    Error,
}

/// Review packet invariant kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RustReviewPacketInvariantKind {
    PrimitiveIdentifierBoundary,
    PublicDataPrimitiveFields,
    AnonymousTupleApiSurface,
    PrimitiveTypeAliasBoundary,
    StringlyStateBoundary,
    ParserFact,
    PublicApiShape,
    ModuleReasoningTree,
    DependencyGraphAcyclicity,
    Custom,
}

/// Receipt kind used by review packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustReviewPacketReceiptKind {
    CargoCheck,
    CargoTest,
    Clippy,
    ExpectTest,
    Proptest,
    CargoFuzz,
    Kani,
    Creusot,
    Verus,
    Waiver,
}

/// Changed behavior summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketChangedBehavior {
    pub snapshot_id: String,
    pub status: RustReviewPacketBehaviorStatus,
    pub subject: String,
    pub(super) summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// Behavior status that needs review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RustReviewPacketBehaviorStatus {
    Changed,
    Missing,
    Skipped,
    Error,
}

/// Missing receipt summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketMissingReceipt {
    pub invariant_id: String,
    pub receipt_kind: RustReviewPacketReceiptKind,
    pub reason: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// New review-packet waiver evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustReviewPacketWaiver {
    pub waiver_id: String,
    pub invariant_id: String,
    pub receipt_kind: RustReviewPacketReceiptKind,
    pub status: RustReviewPacketWaiverStatus,
    pub owner: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Review-packet waiver status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustReviewPacketWaiverStatus {
    Current,
    Stale,
    Expired,
}

/// Determinism readiness summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketDeterminismSummary {
    pub readiness_id: String,
    pub status: RustReviewPacketDeterminismStatus,
    pub observations: usize,
    pub suggestions: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// Determinism readiness status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RustReviewPacketDeterminismStatus {
    Ready,
    NeedsInjection,
    Blocked,
    Unknown,
}

/// Proof pilot summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketProofSummary {
    pub proof_id: String,
    pub target: String,
    pub status: RustReviewPacketProofStatus,
    pub claims: usize,
    pub checks: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// Review packet proof status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RustReviewPacketProofStatus {
    Proved,
    ProvedBounded,
    Failed,
    Skipped,
    Unknown,
}

/// Reviewer action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RustReviewPacketAction {
    pub action_id: String,
    pub kind: RustReviewPacketActionKind,
    pub priority: RustReviewPacketActionPriority,
    pub(super) summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) fields: BTreeMap<String, String>,
}

/// Reviewer action kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum RustReviewPacketActionKind {
    VerifyInvariant,
    InspectBehavior,
    RunReceipt,
    RefreshWaiver,
    AddressDeterminism,
    InspectProof,
}

/// Reviewer action priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum RustReviewPacketActionPriority {
    P0,
    P1,
    P2,
    P3,
}
