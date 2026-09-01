//! Proof pilot artifacts for harness rule judgments.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::rules::agent_policy::dependency_graph::{
    OwnerDependencyProofEdge, owner_dependency_cycle_indices,
};

/// Shared formal proof pilot schema id.
pub const RUST_FORMAL_PROOF_PILOT_SCHEMA_ID: &str =
    "agent.semantic-protocols.semantic-formal-proof-pilot";

/// Shared formal proof pilot schema version.
pub const RUST_FORMAL_PROOF_PILOT_SCHEMA_VERSION: &str = "1";

/// Shared formal proof pilot protocol id.
pub const RUST_FORMAL_PROOF_PILOT_PROTOCOL_ID: &str = "agent.semantic-protocols.formal-proof-pilot";

/// Shared formal proof pilot protocol version.
pub const RUST_FORMAL_PROOF_PILOT_PROTOCOL_VERSION: &str = "1";

/// Input for building a formal proof pilot packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustFormalProofPilotInput {
    /// Maximum directed graph node count to check exhaustively.
    pub max_nodes: usize,
}

/// Formal proof pilot packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFormalProofPilot {
    /// Shared schema id.
    pub schema_id: RustFormalProofPilotSchemaId,
    /// Shared schema version.
    pub schema_version: RustFormalProofPilotSchemaVersion,
    /// Shared protocol id.
    pub protocol_id: RustFormalProofPilotProtocolId,
    /// Shared protocol version.
    pub protocol_version: RustFormalProofPilotProtocolVersion,
    /// Stable proof id.
    pub proof_id: RustFormalProofPilotId,
    /// Producer metadata.
    pub producer: RustFormalProofPilotProducer,
    /// Proof target.
    pub target: RustFormalProofPilotTarget,
    /// Proof method.
    pub method: RustFormalProofPilotMethod,
    /// Overall proof status.
    pub status: RustFormalProofPilotStatus,
    /// Claims covered by this pilot.
    pub claims: Vec<RustFormalProofPilotClaim>,
    /// Concrete checks executed.
    pub checks: Vec<RustFormalProofPilotCheck>,
    /// Linked verification receipt ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_ids: Vec<String>,
    /// Additional provider-owned fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Stable proof id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotId(pub String);

/// Schema id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotSchemaId(pub String);

/// Schema version newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotSchemaVersion(pub String);

/// Protocol id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotProtocolId(pub String);

/// Protocol version newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotProtocolVersion(pub String);

/// Proof pilot producer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFormalProofPilotProducer {
    /// Source language id.
    pub language_id: RustFormalProofPilotLanguageId,
    /// Provider id.
    pub provider_id: RustFormalProofPilotProviderId,
    /// Provider namespace.
    pub namespace: RustFormalProofPilotNamespace,
}

/// Language id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotLanguageId(pub String);

/// Provider id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotProviderId(pub String);

/// Provider namespace newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotNamespace(pub String);

/// Proof target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFormalProofPilotTarget {
    /// Target kind.
    pub kind: RustFormalProofPilotTargetKind,
    /// Human-readable target name.
    pub name: RustFormalProofPilotTargetName,
    /// Rule ids covered by this target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_ids: Vec<String>,
    /// Owner path of the rule or proof target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_path: Option<String>,
    /// Rule core symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Additional target facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Proof target kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustFormalProofPilotTargetKind {
    /// Parser fact target.
    ParserFact,
    /// Public API shape target.
    PublicApiShape,
    /// Module reasoning tree target.
    ModuleReasoningTree,
    /// Dependency graph acyclicity target.
    DependencyGraphAcyclicity,
    /// Custom target.
    Custom,
}

/// Target name newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotTargetName(pub String);

/// Proof method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFormalProofPilotMethod {
    /// Method kind.
    pub kind: RustFormalProofPilotMethodKind,
    /// Tool used by the proof pilot.
    pub tool: RustFormalProofPilotTool,
    /// Command that produced the proof, when applicable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    /// Additional method facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Proof method kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustFormalProofPilotMethodKind {
    /// Exhaustive bounded model checking implemented by the harness.
    ExhaustiveSmallModel,
    /// Kani proof adapter.
    Kani,
    /// Creusot proof adapter.
    Creusot,
    /// Verus proof adapter.
    Verus,
    /// Manual proof outline.
    ManualProofOutline,
    /// Custom method.
    Custom,
}

/// Proof tool newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotTool(pub String);

/// Proof status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustFormalProofPilotStatus {
    /// Fully proved by a formal backend.
    Proved,
    /// Proved over an explicit bounded model space.
    ProvedBounded,
    /// A counterexample was found.
    Failed,
    /// Proof was skipped.
    Skipped,
    /// Proof status is unknown.
    Unknown,
}

/// Proof claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFormalProofPilotClaim {
    /// Stable claim id.
    pub claim_id: RustFormalProofPilotClaimId,
    /// Claim statement.
    pub statement: RustFormalProofPilotStatement,
    /// Claim status.
    pub status: RustFormalProofPilotStatus,
    /// Additional claim facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Claim id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotClaimId(pub String);

/// Claim statement newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotStatement(pub String);

/// Concrete proof check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFormalProofPilotCheck {
    /// Stable check id.
    pub check_id: RustFormalProofPilotCheckId,
    /// Check status.
    pub status: RustFormalProofPilotStatus,
    /// Check summary.
    pub summary: RustFormalProofPilotSummary,
    /// Number of models checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub models_checked: Option<usize>,
    /// Maximum node count checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_nodes: Option<usize>,
    /// Counterexample, when found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counterexample: Option<RustFormalProofPilotCounterexample>,
    /// Additional check facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Check id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotCheckId(pub String);

/// Check summary newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustFormalProofPilotSummary(pub String);

/// Proof counterexample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFormalProofPilotCounterexample {
    /// Counterexample summary.
    pub summary: RustFormalProofPilotSummary,
    /// Additional counterexample facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Build the dependency graph acyclicity proof pilot.
///
/// # Errors
///
/// Returns an error when the requested model size is too large for the built-in
/// exhaustive checker.
pub fn build_rust_dependency_graph_acyclicity_proof_pilot(
    input: RustFormalProofPilotInput,
) -> Result<RustFormalProofPilot, String> {
    if input.max_nodes > 5 {
        return Err("dependency graph proof pilot supports --max-nodes up to 5".to_owned());
    }
    let result = exhaustive_dependency_graph_check(input.max_nodes);
    let status = if result.counterexample.is_some() {
        RustFormalProofPilotStatus::Failed
    } else {
        RustFormalProofPilotStatus::ProvedBounded
    };
    Ok(RustFormalProofPilot {
        schema_id: RustFormalProofPilotSchemaId(RUST_FORMAL_PROOF_PILOT_SCHEMA_ID.to_owned()),
        schema_version: RustFormalProofPilotSchemaVersion(
            RUST_FORMAL_PROOF_PILOT_SCHEMA_VERSION.to_owned(),
        ),
        protocol_id: RustFormalProofPilotProtocolId(RUST_FORMAL_PROOF_PILOT_PROTOCOL_ID.to_owned()),
        protocol_version: RustFormalProofPilotProtocolVersion(
            RUST_FORMAL_PROOF_PILOT_PROTOCOL_VERSION.to_owned(),
        ),
        proof_id: RustFormalProofPilotId("rust.proof.dependency-graph-acyclicity".to_owned()),
        producer: default_producer(),
        target: RustFormalProofPilotTarget {
            kind: RustFormalProofPilotTargetKind::DependencyGraphAcyclicity,
            name: RustFormalProofPilotTargetName(
                "owner dependency graph cycle detection".to_owned(),
            ),
            rule_ids: vec!["RUST-AGENT-OWNER-GRAPH-009".to_owned()],
            owner_path: Some("src/rules/agent_policy/dependency_graph.rs".to_owned()),
            symbol: Some("owner_dependency_cycle_indices".to_owned()),
            fields: BTreeMap::new(),
        },
        method: RustFormalProofPilotMethod {
            kind: RustFormalProofPilotMethodKind::ExhaustiveSmallModel,
            tool: RustFormalProofPilotTool("asp-rust".to_owned()),
            command: vec![
                "asp-rust".to_owned(),
                "proof".to_owned(),
                "pilot".to_owned(),
                "dependency-graph-acyclicity".to_owned(),
                "--max-nodes".to_owned(),
                input.max_nodes.to_string(),
                "--json".to_owned(),
            ],
            fields: BTreeMap::new(),
        },
        status,
        claims: vec![RustFormalProofPilotClaim {
            claim_id: RustFormalProofPilotClaimId("cycle-detection-iff-directed-cycle".to_owned()),
            statement: RustFormalProofPilotStatement(format!(
                "For all directed graphs up to {} nodes, the RUST-AGENT-OWNER-GRAPH-009 rule core reports a cycle iff the graph contains a directed cycle.",
                input.max_nodes
            )),
            status,
            fields: BTreeMap::new(),
        }],
        checks: vec![RustFormalProofPilotCheck {
            check_id: RustFormalProofPilotCheckId(format!(
                "exhaustive-directed-graphs-up-to-{}",
                input.max_nodes
            )),
            status,
            summary: RustFormalProofPilotSummary(format!(
                "Checked all directed graphs with up to {} nodes.",
                input.max_nodes
            )),
            models_checked: Some(result.models_checked),
            max_nodes: Some(input.max_nodes),
            counterexample: result.counterexample.map(|summary| {
                RustFormalProofPilotCounterexample {
                    summary: RustFormalProofPilotSummary(summary),
                    fields: BTreeMap::new(),
                }
            }),
            fields: BTreeMap::new(),
        }],
        receipt_ids: Vec::new(),
        fields: BTreeMap::new(),
    })
}

/// Render formal proof pilot as compact text.
#[must_use]
pub fn render_rust_formal_proof_pilot(proof: &RustFormalProofPilot) -> String {
    let mut output = format!(
        "[formal-proof-pilot] target={:?} status={:?} checks={}\n",
        proof.target.kind,
        proof.status,
        proof.checks.len()
    );
    for check in &proof.checks {
        output.push_str(&format!(
            "|check id={} status={:?} models={} summary={}\n",
            check.check_id.0,
            check.status,
            check.models_checked.unwrap_or_default(),
            check.summary.0
        ));
    }
    output
}

/// Render formal proof pilot as JSON.
///
/// # Errors
///
/// Returns an error when JSON serialization fails.
pub fn render_rust_formal_proof_pilot_json(
    proof: &RustFormalProofPilot,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(proof)
}

struct ExhaustiveGraphCheckResult {
    models_checked: usize,
    counterexample: Option<String>,
}

fn exhaustive_dependency_graph_check(max_nodes: usize) -> ExhaustiveGraphCheckResult {
    let mut models_checked = 0;
    for node_count in 0..=max_nodes {
        let possible_edges = possible_directed_edges(node_count);
        let model_count = 1usize << possible_edges.len();
        for mask in 0..model_count {
            let edges = possible_edges
                .iter()
                .enumerate()
                .filter_map(|(bit, edge)| {
                    if (mask & (1usize << bit)) == 0 {
                        None
                    } else {
                        Some(*edge)
                    }
                })
                .collect::<Vec<_>>();
            models_checked += 1;
            let expected = graph_has_directed_cycle(node_count, &edges);
            let proof_edges = edges
                .iter()
                .map(|(source, target)| OwnerDependencyProofEdge {
                    source_namespace: vec![format!("n{source}")],
                    target_namespace: vec![format!("n{target}")],
                })
                .collect::<Vec<_>>();
            let actual = !owner_dependency_cycle_indices(&proof_edges).is_empty();
            if actual != expected {
                return ExhaustiveGraphCheckResult {
                    models_checked,
                    counterexample: Some(format!(
                        "node_count={node_count} edges={edges:?} expected_cycle={expected} actual_cycle={actual}"
                    )),
                };
            }
        }
    }
    ExhaustiveGraphCheckResult {
        models_checked,
        counterexample: None,
    }
}

fn possible_directed_edges(node_count: usize) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for source in 0..node_count {
        for target in 0..node_count {
            if source != target {
                edges.push((source, target));
            }
        }
    }
    edges
}

fn graph_has_directed_cycle(node_count: usize, edges: &[(usize, usize)]) -> bool {
    let mut adjacency = vec![Vec::<usize>::new(); node_count];
    for (source, target) in edges {
        adjacency[*source].push(*target);
    }
    let mut state = vec![VisitState::Unvisited; node_count];
    (0..node_count).any(|node| graph_dfs_has_cycle(node, &adjacency, &mut state))
}

fn graph_dfs_has_cycle(node: usize, adjacency: &[Vec<usize>], state: &mut [VisitState]) -> bool {
    match state[node] {
        VisitState::Visiting => return true,
        VisitState::Visited => return false,
        VisitState::Unvisited => {}
    }
    state[node] = VisitState::Visiting;
    let has_cycle = adjacency[node]
        .iter()
        .any(|next| graph_dfs_has_cycle(*next, adjacency, state));
    if !has_cycle {
        state[node] = VisitState::Visited;
    }
    has_cycle
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn default_producer() -> RustFormalProofPilotProducer {
    RustFormalProofPilotProducer {
        language_id: RustFormalProofPilotLanguageId("rust".to_owned()),
        provider_id: RustFormalProofPilotProviderId("asp-rust".to_owned()),
        namespace: RustFormalProofPilotNamespace(
            "agent.semantic-protocols.languages.rust.asp-rust".to_owned(),
        ),
    }
}
