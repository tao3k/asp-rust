//! Graph-turbo request packets for evidence-graph analysis.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::model::{
    RustEvidenceEdge, RustEvidenceEdgeKind, RustEvidenceGap, RustEvidenceGraph,
    RustEvidenceGraphProject, RustEvidenceGraphSummary, RustEvidenceNode, RustEvidenceNodeKind,
    RustEvidenceNodeStatus,
};

/// Shared graph-turbo request schema id consumed by `asp graph render`.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_REQUEST_SCHEMA_ID: &str =
    "agent.semantic-protocols.semantic-graph-turbo-request";
/// Shared graph-turbo request schema version.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_REQUEST_SCHEMA_VERSION: &str = "1";
/// Shared graph-turbo request protocol id.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_PROTOCOL_ID: &str =
    "agent.semantic-protocols.semantic-language";
/// Shared graph-turbo request protocol version.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_PROTOCOL_VERSION: &str = "1";
/// Graph-turbo packet kind discriminator.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_PACKET_KIND: &str = "graph-turbo-request";
/// Graph-turbo algorithm profile served by asp-python-graphs.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_ALGORITHM: &str = "typed-ppr-diverse";
/// Request surface for Rust evidence graph analysis.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_SURFACE: &str = "evidence-analyze";
/// Rust evidence analysis profile name.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_PROFILE: &str = "rust-evidence-quality";
/// Default frontier budget for evidence analysis packets.
pub const RUST_EVIDENCE_GRAPH_ANALYSIS_BUDGET: usize = 8;

/// Input for building a graph-turbo evidence analysis request.
#[derive(Debug, Clone)]
pub struct RustEvidenceGraphAnalysisInput {
    pub project_root: PathBuf,
    pub evidence_graphs: Vec<RustEvidenceGraph>,
}

/// raw dto boundary: serialized request packet is consumed by graph-turbo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphAnalysisRequest {
    pub schema_id: String,
    pub schema_version: String,
    pub protocol_id: String,
    pub protocol_version: String,
    pub packet_kind: RustEvidenceGraphAnalysisPacketKind,
    pub request_id: String,
    pub surface: String,
    pub query_terms: Vec<String>,
    pub profile: String,
    pub algorithm: String,
    pub seed_ids: Vec<String>,
    pub budget: usize,
    pub producer: RustEvidenceGraphAnalysisProducer,
    pub project: RustEvidenceGraphProject,
    pub summary: RustEvidenceGraphAnalysisSummary,
    pub graphs: Vec<RustEvidenceGraphAnalysisGraph>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Graph-turbo request packet kind for Rust evidence analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustEvidenceGraphAnalysisPacketKind {
    /// Packet consumed by graph-turbo request renderers.
    GraphTurboRequest,
}

/// Producer metadata for Rust evidence analysis graph-turbo requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphAnalysisProducer {
    pub language_id: String,
    pub provider_id: String,
    pub namespace: String,
}

/// Aggregate counts included in a Rust evidence analysis request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphAnalysisSummary {
    pub graphs: usize,
    pub nodes: usize,
    pub edges: usize,
    pub owners: usize,
    pub claims: usize,
    pub stale_items: usize,
    pub gaps: usize,
}

/// Evidence graph payload projected for graph-turbo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphAnalysisGraph {
    pub graph_id: String,
    pub summary: RustEvidenceGraphSummary,
    pub nodes: Vec<RustEvidenceGraphAnalysisNode>,
    pub edges: Vec<RustEvidenceGraphAnalysisEdge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<RustEvidenceGap>,
}

/// raw dto boundary: serialized graph-turbo node projected from Rust evidence.
/// Graph-turbo node projected from a Rust evidence graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphAnalysisNode {
    pub id: String,
    pub kind: RustEvidenceGraphAnalysisNodeKind,
    pub role: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Graph-turbo node kind catalog for Rust evidence graph analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustEvidenceGraphAnalysisNodeKind {
    Owner,
    InvariantCandidate,
    VerificationReceipt,
    BehaviorSnapshot,
    DeterminismReadiness,
    FormalProofPilot,
    ReviewPacket,
    Waiver,
    ReviewAction,
}

/// raw dto boundary: serialized graph-turbo edge projected from Rust evidence.
/// Graph-turbo edge projected from a Rust evidence graph edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEvidenceGraphAnalysisEdge {
    pub source: String,
    pub target: String,
    pub relation: RustEvidenceGraphAnalysisEdgeRelation,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Graph-turbo edge relation catalog for Rust evidence graph analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustEvidenceGraphAnalysisEdgeRelation {
    DerivedFrom,
    RequiresEvidence,
    VerifiedBy,
    ObservedBy,
    WaivedBy,
    ReviewedBy,
    SuggestsAction,
    SupportsClaim,
}

/// Build a graph-turbo request from Rust evidence graphs.
#[must_use]
pub fn build_rust_evidence_graph_analysis_request(
    input: RustEvidenceGraphAnalysisInput,
) -> RustEvidenceGraphAnalysisRequest {
    let summary = evidence_analysis_summary(&input.evidence_graphs);
    let graphs: Vec<RustEvidenceGraphAnalysisGraph> = input
        .evidence_graphs
        .into_iter()
        .map(evidence_analysis_graph)
        .collect();
    let seed_ids = evidence_analysis_seed_ids(&graphs);
    RustEvidenceGraphAnalysisRequest {
        schema_id: RUST_EVIDENCE_GRAPH_ANALYSIS_REQUEST_SCHEMA_ID.to_owned(),
        schema_version: RUST_EVIDENCE_GRAPH_ANALYSIS_REQUEST_SCHEMA_VERSION.to_owned(),
        protocol_id: RUST_EVIDENCE_GRAPH_ANALYSIS_PROTOCOL_ID.to_owned(),
        protocol_version: RUST_EVIDENCE_GRAPH_ANALYSIS_PROTOCOL_VERSION.to_owned(),
        packet_kind: RustEvidenceGraphAnalysisPacketKind::GraphTurboRequest,
        request_id: format!(
            "rust.evidence.analysis.graphs-{}.nodes-{}.gaps-{}",
            summary.graphs, summary.nodes, summary.gaps
        ),
        surface: RUST_EVIDENCE_GRAPH_ANALYSIS_SURFACE.to_owned(),
        query_terms: vec!["rust evidence quality".to_owned()],
        profile: RUST_EVIDENCE_GRAPH_ANALYSIS_PROFILE.to_owned(),
        algorithm: RUST_EVIDENCE_GRAPH_ANALYSIS_ALGORITHM.to_owned(),
        seed_ids,
        budget: RUST_EVIDENCE_GRAPH_ANALYSIS_BUDGET,
        producer: RustEvidenceGraphAnalysisProducer {
            language_id: "rust".to_owned(),
            provider_id: "asp-rust".to_owned(),
            namespace: "agent.semantic-protocols.languages.rust.asp-rust".to_owned(),
        },
        project: RustEvidenceGraphProject {
            root: input.project_root.display().to_string(),
            package: None,
            fields: BTreeMap::new(),
        },
        summary,
        graphs,
        fields: BTreeMap::from([(
            "next".to_owned(),
            "pipe JSON to `asp graph render --packet - --view seeds`".to_owned(),
        )]),
    }
}

/// Render a compact agent-facing evidence analysis request summary.
#[must_use]
pub fn render_rust_evidence_graph_analysis_request(
    request: &RustEvidenceGraphAnalysisRequest,
) -> String {
    format!(
        "evidence-analysis profile={} graphs={} nodes={} edges={} owners={} claims={} stale-items={} gaps={} next=\"asp graph render --packet - --view seeds\"",
        request.profile,
        request.summary.graphs,
        request.summary.nodes,
        request.summary.edges,
        request.summary.owners,
        request.summary.claims,
        request.summary.stale_items,
        request.summary.gaps
    )
}

/// Render graph-turbo request JSON.
pub fn render_rust_evidence_graph_analysis_request_json(
    request: &RustEvidenceGraphAnalysisRequest,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(request)
}

fn evidence_analysis_summary(graphs: &[RustEvidenceGraph]) -> RustEvidenceGraphAnalysisSummary {
    graphs.iter().fold(
        RustEvidenceGraphAnalysisSummary {
            graphs: graphs.len(),
            nodes: 0,
            edges: 0,
            owners: 0,
            claims: 0,
            stale_items: 0,
            gaps: 0,
        },
        |mut summary, graph| {
            summary.nodes += graph.summary.nodes;
            summary.edges += graph.summary.edges;
            summary.owners += graph.summary.owners;
            summary.claims += graph.summary.claims;
            summary.stale_items += graph.summary.stale_items;
            summary.gaps += graph.summary.gaps;
            summary
        },
    )
}

fn evidence_analysis_graph(graph: RustEvidenceGraph) -> RustEvidenceGraphAnalysisGraph {
    RustEvidenceGraphAnalysisGraph {
        graph_id: graph.graph_id,
        summary: graph.summary,
        nodes: graph
            .nodes
            .into_iter()
            .map(evidence_analysis_node)
            .collect(),
        edges: graph
            .edges
            .into_iter()
            .map(evidence_analysis_edge)
            .collect(),
        gaps: graph.gaps,
    }
}

fn evidence_analysis_node(node: RustEvidenceNode) -> RustEvidenceGraphAnalysisNode {
    let kind = evidence_node_kind(node.kind);
    let role = evidence_node_role(node.kind).to_owned();
    let location_path = node
        .location
        .as_ref()
        .and_then(|location| location.path.clone());
    let start_line = node.location.as_ref().and_then(|location| location.line);
    let path = node.owner_path.clone().or(location_path);
    let locator = path
        .as_ref()
        .zip(start_line)
        .map(|(path, line)| format!("{path}:{line}:{line}"));
    let mut fields = node.fields;
    insert_optional_field(&mut fields, "candidateId", node.candidate_id);
    insert_optional_field(&mut fields, "receiptId", node.receipt_id);
    insert_optional_field(&mut fields, "snapshotId", node.snapshot_id);
    insert_optional_field(&mut fields, "readinessId", node.readiness_id);
    insert_optional_field(&mut fields, "proofId", node.proof_id);
    insert_optional_field(&mut fields, "packetId", node.packet_id);
    insert_optional_field(&mut fields, "waiverId", node.waiver_id);
    insert_optional_field(&mut fields, "actionId", node.action_id);
    insert_optional_field(&mut fields, "summary", node.summary);
    if let Some(status) = node.status {
        fields.insert("status".to_owned(), evidence_node_status(status).to_owned());
    }
    RustEvidenceGraphAnalysisNode {
        id: node.node_id,
        kind,
        role,
        value: node.label,
        path: path.clone(),
        owner_path: node.owner_path.or(path),
        locator,
        start_line,
        end_line: start_line,
        fields,
    }
}

fn evidence_analysis_edge(edge: RustEvidenceEdge) -> RustEvidenceGraphAnalysisEdge {
    let mut fields = edge.fields;
    fields.insert("edgeId".to_owned(), edge.edge_id);
    insert_optional_field(&mut fields, "label", edge.label);
    RustEvidenceGraphAnalysisEdge {
        source: edge.from_node_id,
        target: edge.to_node_id,
        relation: evidence_edge_relation(edge.kind).to_owned(),
        fields,
    }
}

fn evidence_analysis_seed_ids(graphs: &[RustEvidenceGraphAnalysisGraph]) -> Vec<String> {
    let mut seed_ids = Vec::new();
    for graph in graphs {
        for node in &graph.nodes {
            if node.kind == RustEvidenceGraphAnalysisNodeKind::Owner && !seed_ids.contains(&node.id)
            {
                seed_ids.push(node.id.clone());
            }
        }
    }
    if seed_ids.is_empty()
        && let Some(node) = graphs.iter().flat_map(|graph| &graph.nodes).next()
    {
        seed_ids.push(node.id.clone());
    }
    seed_ids
}

fn evidence_node_kind(kind: RustEvidenceNodeKind) -> RustEvidenceGraphAnalysisNodeKind {
    match kind {
        RustEvidenceNodeKind::Owner => RustEvidenceGraphAnalysisNodeKind::Owner,
        RustEvidenceNodeKind::InvariantCandidate => {
            RustEvidenceGraphAnalysisNodeKind::InvariantCandidate
        }
        RustEvidenceNodeKind::VerificationReceipt => {
            RustEvidenceGraphAnalysisNodeKind::VerificationReceipt
        }
        RustEvidenceNodeKind::BehaviorSnapshot => {
            RustEvidenceGraphAnalysisNodeKind::BehaviorSnapshot
        }
        RustEvidenceNodeKind::DeterminismReadiness => {
            RustEvidenceGraphAnalysisNodeKind::DeterminismReadiness
        }
        RustEvidenceNodeKind::FormalProofPilot => {
            RustEvidenceGraphAnalysisNodeKind::FormalProofPilot
        }
        RustEvidenceNodeKind::ReviewPacket => RustEvidenceGraphAnalysisNodeKind::ReviewPacket,
        RustEvidenceNodeKind::Waiver => RustEvidenceGraphAnalysisNodeKind::Waiver,
        RustEvidenceNodeKind::ReviewAction => RustEvidenceGraphAnalysisNodeKind::ReviewAction,
    }
}

fn evidence_node_role(kind: RustEvidenceNodeKind) -> &'static str {
    match kind {
        RustEvidenceNodeKind::Owner => "path",
        RustEvidenceNodeKind::InvariantCandidate => "claim",
        RustEvidenceNodeKind::VerificationReceipt => "receipt",
        RustEvidenceNodeKind::BehaviorSnapshot => "snapshot",
        RustEvidenceNodeKind::DeterminismReadiness => "readiness",
        RustEvidenceNodeKind::FormalProofPilot => "proof",
        RustEvidenceNodeKind::ReviewPacket => "packet",
        RustEvidenceNodeKind::Waiver => "waiver",
        RustEvidenceNodeKind::ReviewAction => "action",
    }
}

fn evidence_node_status(status: RustEvidenceNodeStatus) -> &'static str {
    match status {
        RustEvidenceNodeStatus::Current => "current",
        RustEvidenceNodeStatus::Changed => "changed",
        RustEvidenceNodeStatus::Missing => "missing",
        RustEvidenceNodeStatus::Stale => "stale",
        RustEvidenceNodeStatus::Expired => "expired",
        RustEvidenceNodeStatus::Ready => "ready",
        RustEvidenceNodeStatus::NeedsInjection => "needs-injection",
        RustEvidenceNodeStatus::Blocked => "blocked",
        RustEvidenceNodeStatus::Unknown => "unknown",
        RustEvidenceNodeStatus::Proved => "proved",
        RustEvidenceNodeStatus::ProvedBounded => "proved-bounded",
        RustEvidenceNodeStatus::Failed => "failed",
        RustEvidenceNodeStatus::Skipped => "skipped",
        RustEvidenceNodeStatus::Error => "error",
    }
}

fn evidence_edge_relation(kind: RustEvidenceEdgeKind) -> RustEvidenceGraphAnalysisEdgeRelation {
    match kind {
        RustEvidenceEdgeKind::DerivedFrom => RustEvidenceGraphAnalysisEdgeRelation::DerivedFrom,
        RustEvidenceEdgeKind::RequiresEvidence => {
            RustEvidenceGraphAnalysisEdgeRelation::RequiresEvidence
        }
        RustEvidenceEdgeKind::VerifiedBy => RustEvidenceGraphAnalysisEdgeRelation::VerifiedBy,
        RustEvidenceEdgeKind::ObservedBy => RustEvidenceGraphAnalysisEdgeRelation::ObservedBy,
        RustEvidenceEdgeKind::WaivedBy => RustEvidenceGraphAnalysisEdgeRelation::WaivedBy,
        RustEvidenceEdgeKind::ReviewedBy => RustEvidenceGraphAnalysisEdgeRelation::ReviewedBy,
        RustEvidenceEdgeKind::SuggestsAction => {
            RustEvidenceGraphAnalysisEdgeRelation::SuggestsAction
        }
        RustEvidenceEdgeKind::SupportsClaim => RustEvidenceGraphAnalysisEdgeRelation::SupportsClaim,
    }
}

fn insert_optional_field(
    fields: &mut BTreeMap<String, String>,
    key: &'static str,
    value: Option<String>,
) {
    if let Some(value) = value {
        fields.insert(key.to_owned(), value);
    }
}
