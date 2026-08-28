//! Build `semantic-assurance-case` packets from evidence graph nodes and edges.

use super::model::{
    RUST_ASSURANCE_CASE_PROTOCOL_ID, RUST_ASSURANCE_CASE_PROTOCOL_VERSION,
    RUST_ASSURANCE_CASE_SCHEMA_ID, RUST_ASSURANCE_CASE_SCHEMA_VERSION, RustAssuranceActionRef,
    RustAssuranceCase, RustAssuranceCaseSet, RustAssuranceCaseSetProducer,
    RustAssuranceCaseSetProject, RustAssuranceCaseStatus, RustAssuranceCaseSummary,
    RustAssuranceClaim, RustAssuranceClaimKind, RustAssuranceGap, RustAssuranceNodeKind,
    RustAssuranceNodeRef, RustAssuranceNodeStatus,
};
use crate::verification::{
    RustEvidenceEdge, RustEvidenceEdgeKind, RustEvidenceGap, RustEvidenceGraph, RustEvidenceNode,
    RustEvidenceNodeKind, RustEvidenceNodeStatus,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Input for building a Rust assurance case set.
#[derive(Debug, Clone)]
pub struct RustAssuranceCaseInput {
    pub project_root: PathBuf,
    pub evidence_graphs: Vec<RustEvidenceGraph>,
}

/// Build reviewer-first assurance cases from evidence graphs.
#[must_use]
pub fn build_rust_assurance_case_set(input: RustAssuranceCaseInput) -> RustAssuranceCaseSet {
    let mut builder = RustAssuranceCaseBuilder::new(input.project_root);
    for graph in &input.evidence_graphs {
        builder.add_graph(graph);
    }
    builder.finish()
}

struct RustAssuranceCaseBuilder {
    project_root: PathBuf,
    cases: Vec<RustAssuranceCase>,
}

impl RustAssuranceCaseBuilder {
    fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            cases: Vec::new(),
        }
    }

    fn add_graph(&mut self, graph: &RustEvidenceGraph) {
        let index = GraphIndex::new(graph);
        for node in &graph.nodes {
            match node.kind {
                RustEvidenceNodeKind::InvariantCandidate => {
                    self.cases.push(build_invariant_case(graph, &index, node));
                }
                RustEvidenceNodeKind::FormalProofPilot => {
                    self.cases.push(build_proof_case(&index, node));
                }
                _ => {}
            }
        }
    }

    fn finish(self) -> RustAssuranceCaseSet {
        let supported_claims = self
            .cases
            .iter()
            .filter(|case| case.status == RustAssuranceCaseStatus::Supported)
            .count();
        let open_gaps = self.cases.iter().map(|case| case.gaps.len()).sum();
        let stale_items = self
            .cases
            .iter()
            .map(|case| {
                case.waived_by
                    .iter()
                    .filter(|node| {
                        matches!(
                            node.status,
                            Some(RustAssuranceNodeStatus::Stale)
                                | Some(RustAssuranceNodeStatus::Expired)
                        )
                    })
                    .count()
            })
            .sum();
        let summary = RustAssuranceCaseSummary {
            cases: self.cases.len(),
            claims: self.cases.len(),
            supported_claims,
            open_gaps,
            stale_items,
        };
        RustAssuranceCaseSet {
            schema_id: RUST_ASSURANCE_CASE_SCHEMA_ID.to_owned(),
            schema_version: RUST_ASSURANCE_CASE_SCHEMA_VERSION.to_owned(),
            protocol_id: RUST_ASSURANCE_CASE_PROTOCOL_ID.to_owned(),
            protocol_version: RUST_ASSURANCE_CASE_PROTOCOL_VERSION.to_owned(),
            case_set_id: "rust.assurance.case".to_owned(),
            producer: RustAssuranceCaseSetProducer {
                language_id: "rust".to_owned(),
                provider_id: "asp-rust".to_owned(),
                namespace: "agent.semantic-protocols.languages.rust.asp-rust".to_owned(),
            },
            project: RustAssuranceCaseSetProject {
                root: self.project_root.display().to_string(),
                package: None,
                fields: BTreeMap::new(),
            },
            summary,
            cases: self.cases,
            fields: BTreeMap::new(),
        }
    }
}

struct GraphIndex<'a> {
    nodes: BTreeMap<&'a str, &'a RustEvidenceNode>,
    outgoing: BTreeMap<&'a str, Vec<usize>>,
    incoming: BTreeMap<&'a str, Vec<usize>>,
    graph: &'a RustEvidenceGraph,
}

impl<'a> GraphIndex<'a> {
    fn new(graph: &'a RustEvidenceGraph) -> Self {
        let nodes = graph
            .nodes
            .iter()
            .map(|node| (node.node_id.as_str(), node))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        let mut incoming: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (index, edge) in graph.edges.iter().enumerate() {
            outgoing
                .entry(edge.from_node_id.as_str())
                .or_default()
                .push(index);
            incoming
                .entry(edge.to_node_id.as_str())
                .or_default()
                .push(index);
        }
        Self {
            nodes,
            outgoing,
            incoming,
            graph,
        }
    }

    fn node(&self, node_id: &str) -> Option<&'a RustEvidenceNode> {
        self.nodes.get(node_id).copied()
    }
}

fn build_invariant_case(
    graph: &RustEvidenceGraph,
    index: &GraphIndex<'_>,
    node: &RustEvidenceNode,
) -> RustAssuranceCase {
    let candidate_id = node.candidate_id.as_deref().unwrap_or(&node.node_id);
    let mut supported_by = Vec::new();
    let mut observed_by = Vec::new();
    let mut reviewed_by = Vec::new();
    let mut waived_by = Vec::new();
    let mut actions = Vec::new();
    let mut seen_refs = BTreeSet::new();
    let mut seen_actions = BTreeSet::new();

    {
        let mut evidence_refs = InvariantEvidenceRefs {
            supported_by: &mut supported_by,
            observed_by: &mut observed_by,
            reviewed_by: &mut reviewed_by,
            waived_by: &mut waived_by,
            seen_refs: &mut seen_refs,
        };
        for (edge, target) in outgoing_targets(index, node) {
            collect_outgoing_invariant_evidence(index, edge, target, &mut evidence_refs);
        }
    }

    for (_, source) in incoming_sources(index, node)
        .filter(|(edge, _)| edge.kind == RustEvidenceEdgeKind::RequiresEvidence)
    {
        push_action_ref(&mut actions, &mut seen_actions, source);
    }
    for action in graph.nodes.iter().filter(|node| {
        node.kind == RustEvidenceNodeKind::ReviewAction
            && node
                .fields
                .get("targetId")
                .is_some_and(|target_id| target_id == candidate_id)
    }) {
        push_action_ref(&mut actions, &mut seen_actions, action);
    }

    let gaps = graph
        .gaps
        .iter()
        .filter(|gap| gap_matches_invariant(gap, node, candidate_id))
        .map(assurance_gap)
        .collect::<Vec<_>>();

    let status = case_status(&supported_by, &observed_by, &waived_by, &actions, &gaps);
    RustAssuranceCase {
        case_id: format!("case:{}", sanitize_id_part(&node.node_id)),
        claim: RustAssuranceClaim {
            claim_id: format!("claim:{candidate_id}"),
            kind: RustAssuranceClaimKind::Invariant,
            statement: node
                .summary
                .clone()
                .unwrap_or_else(|| format!("Invariant {candidate_id} is adequately supported")),
            target_node_id: Some(node.node_id.clone()),
            severity: node.fields.get("severity").cloned(),
            fields: string_fields(&[
                ("candidateId", node.candidate_id.as_deref()),
                (
                    "sourceRuleId",
                    node.fields.get("sourceRuleId").map(String::as_str),
                ),
            ]),
        },
        status,
        subject_node_id: Some(node.node_id.clone()),
        owner_path: node.owner_path.clone(),
        supported_by,
        observed_by,
        reviewed_by,
        waived_by,
        actions,
        gaps,
        fields: BTreeMap::new(),
    }
}

fn build_proof_case(index: &GraphIndex<'_>, node: &RustEvidenceNode) -> RustAssuranceCase {
    let mut supported_by = Vec::new();
    let mut reviewed_by = Vec::new();
    let mut seen_refs = BTreeSet::new();
    push_node_ref(&mut supported_by, &mut seen_refs, node);
    for edge_index in index
        .outgoing
        .get(node.node_id.as_str())
        .into_iter()
        .flatten()
    {
        let edge = &index.graph.edges[*edge_index];
        if edge.kind == RustEvidenceEdgeKind::SupportsClaim
            && let Some(target) = index.node(&edge.to_node_id)
            && target.kind == RustEvidenceNodeKind::ReviewPacket
        {
            push_node_ref(&mut reviewed_by, &mut seen_refs, target);
        }
    }

    let status = match node.status {
        Some(RustEvidenceNodeStatus::Proved | RustEvidenceNodeStatus::ProvedBounded) => {
            RustAssuranceCaseStatus::Supported
        }
        Some(
            RustEvidenceNodeStatus::Failed
            | RustEvidenceNodeStatus::Blocked
            | RustEvidenceNodeStatus::Error,
        ) => RustAssuranceCaseStatus::Blocked,
        _ => RustAssuranceCaseStatus::Unknown,
    };

    RustAssuranceCase {
        case_id: format!("case:{}", sanitize_id_part(&node.node_id)),
        claim: RustAssuranceClaim {
            claim_id: format!(
                "claim:{}",
                sanitize_id_part(node.proof_id.as_deref().unwrap_or(&node.node_id))
            ),
            kind: RustAssuranceClaimKind::Proof,
            statement: node
                .summary
                .clone()
                .unwrap_or_else(|| format!("Proof {} supports its target claim", node.label)),
            target_node_id: Some(node.node_id.clone()),
            severity: None,
            fields: string_fields(&[("proofId", node.proof_id.as_deref())]),
        },
        status,
        subject_node_id: Some(node.node_id.clone()),
        owner_path: node.owner_path.clone(),
        supported_by,
        observed_by: Vec::new(),
        reviewed_by,
        waived_by: Vec::new(),
        actions: Vec::new(),
        gaps: Vec::new(),
        fields: BTreeMap::new(),
    }
}

fn outgoing_targets<'a>(
    index: &'a GraphIndex<'a>,
    node: &RustEvidenceNode,
) -> impl Iterator<Item = (&'a RustEvidenceEdge, &'a RustEvidenceNode)> + 'a {
    index
        .outgoing
        .get(node.node_id.as_str())
        .into_iter()
        .flatten()
        .filter_map(|edge_index| {
            let edge = &index.graph.edges[*edge_index];
            let target = index.node(&edge.to_node_id)?;
            Some((edge, target))
        })
}

fn incoming_sources<'a>(
    index: &'a GraphIndex<'a>,
    node: &RustEvidenceNode,
) -> impl Iterator<Item = (&'a RustEvidenceEdge, &'a RustEvidenceNode)> + 'a {
    index
        .incoming
        .get(node.node_id.as_str())
        .into_iter()
        .flatten()
        .filter_map(|edge_index| {
            let edge = &index.graph.edges[*edge_index];
            let source = index.node(&edge.from_node_id)?;
            Some((edge, source))
        })
}

struct InvariantEvidenceRefs<'a> {
    supported_by: &'a mut Vec<RustAssuranceNodeRef>,
    observed_by: &'a mut Vec<RustAssuranceNodeRef>,
    reviewed_by: &'a mut Vec<RustAssuranceNodeRef>,
    waived_by: &'a mut Vec<RustAssuranceNodeRef>,
    seen_refs: &'a mut BTreeSet<String>,
}

fn collect_outgoing_invariant_evidence(
    index: &GraphIndex<'_>,
    edge: &RustEvidenceEdge,
    target: &RustEvidenceNode,
    refs: &mut InvariantEvidenceRefs<'_>,
) {
    match edge.kind {
        RustEvidenceEdgeKind::ObservedBy => {
            push_node_ref(refs.observed_by, refs.seen_refs, target);
            for receipt in receipt_refs_from(index, target) {
                push_existing_ref(refs.supported_by, refs.seen_refs, receipt);
            }
        }
        RustEvidenceEdgeKind::VerifiedBy | RustEvidenceEdgeKind::SupportsClaim => {
            push_node_ref(refs.supported_by, refs.seen_refs, target);
        }
        RustEvidenceEdgeKind::WaivedBy => {
            push_node_ref(refs.waived_by, refs.seen_refs, target);
        }
        RustEvidenceEdgeKind::DerivedFrom if target.kind == RustEvidenceNodeKind::ReviewPacket => {
            push_node_ref(refs.reviewed_by, refs.seen_refs, target);
        }
        _ => {}
    }
}

fn receipt_refs_from(index: &GraphIndex<'_>, node: &RustEvidenceNode) -> Vec<RustAssuranceNodeRef> {
    index
        .outgoing
        .get(node.node_id.as_str())
        .into_iter()
        .flatten()
        .filter_map(|edge_index| {
            let edge = &index.graph.edges[*edge_index];
            if edge.kind != RustEvidenceEdgeKind::ObservedBy {
                return None;
            }
            let target = index.node(&edge.to_node_id)?;
            if target.kind == RustEvidenceNodeKind::VerificationReceipt {
                Some(node_ref(target))
            } else {
                None
            }
        })
        .collect()
}

fn gap_matches_invariant(
    gap: &RustEvidenceGap,
    node: &RustEvidenceNode,
    candidate_id: &str,
) -> bool {
    gap.fields
        .get("invariantId")
        .is_some_and(|invariant_id| invariant_id == candidate_id)
        || gap
            .owner_path
            .as_ref()
            .zip(node.owner_path.as_ref())
            .is_some_and(|(gap_owner, node_owner)| gap_owner == node_owner)
}

fn case_status(
    supported_by: &[RustAssuranceNodeRef],
    observed_by: &[RustAssuranceNodeRef],
    waived_by: &[RustAssuranceNodeRef],
    actions: &[RustAssuranceActionRef],
    gaps: &[RustAssuranceGap],
) -> RustAssuranceCaseStatus {
    let has_blocking_evidence = supported_by
        .iter()
        .chain(observed_by.iter())
        .chain(waived_by.iter())
        .any(|node| {
            matches!(
                node.status,
                Some(
                    RustAssuranceNodeStatus::Failed
                        | RustAssuranceNodeStatus::Blocked
                        | RustAssuranceNodeStatus::Error
                )
            )
        });
    if has_blocking_evidence {
        return RustAssuranceCaseStatus::Blocked;
    }
    let has_stale_waiver = waived_by.iter().any(|node| {
        matches!(
            node.status,
            Some(RustAssuranceNodeStatus::Stale | RustAssuranceNodeStatus::Expired)
        )
    });
    if !gaps.is_empty() || !actions.is_empty() || has_stale_waiver {
        return RustAssuranceCaseStatus::NeedsReview;
    }
    if !supported_by.is_empty() || !observed_by.is_empty() {
        RustAssuranceCaseStatus::Supported
    } else {
        RustAssuranceCaseStatus::Unknown
    }
}

fn push_node_ref(
    refs: &mut Vec<RustAssuranceNodeRef>,
    seen: &mut BTreeSet<String>,
    node: &RustEvidenceNode,
) {
    push_existing_ref(refs, seen, node_ref(node));
}

fn push_existing_ref(
    refs: &mut Vec<RustAssuranceNodeRef>,
    seen: &mut BTreeSet<String>,
    node_ref: RustAssuranceNodeRef,
) {
    if seen.insert(node_ref.node_id.clone()) {
        refs.push(node_ref);
    }
}

fn push_action_ref(
    actions: &mut Vec<RustAssuranceActionRef>,
    seen: &mut BTreeSet<String>,
    node: &RustEvidenceNode,
) {
    if node.kind != RustEvidenceNodeKind::ReviewAction || !seen.insert(node.node_id.clone()) {
        return;
    }
    actions.push(RustAssuranceActionRef {
        node_id: node.node_id.clone(),
        action_id: node.action_id.clone(),
        summary: node.label.clone(),
        priority: node.fields.get("priority").cloned(),
        fields: string_fields(&[("targetId", node.fields.get("targetId").map(String::as_str))]),
    });
}

fn node_ref(node: &RustEvidenceNode) -> RustAssuranceNodeRef {
    RustAssuranceNodeRef {
        node_id: node.node_id.clone(),
        kind: assurance_node_kind(node.kind),
        label: node.label.clone(),
        status: node.status.map(assurance_node_status),
        summary: node.summary.clone(),
        fields: BTreeMap::new(),
    }
}

fn assurance_gap(gap: &RustEvidenceGap) -> RustAssuranceGap {
    RustAssuranceGap {
        gap_id: format!("assurance:{}", sanitize_id_part(&gap.gap_id)),
        source_gap_id: Some(gap.gap_id.clone()),
        owner_path: gap.owner_path.clone(),
        summary: gap.summary.clone(),
        severity: gap.severity.clone(),
        fields: gap.fields.clone(),
    }
}

fn assurance_node_kind(kind: RustEvidenceNodeKind) -> RustAssuranceNodeKind {
    match kind {
        RustEvidenceNodeKind::Owner => RustAssuranceNodeKind::Owner,
        RustEvidenceNodeKind::InvariantCandidate => RustAssuranceNodeKind::InvariantCandidate,
        RustEvidenceNodeKind::VerificationReceipt => RustAssuranceNodeKind::VerificationReceipt,
        RustEvidenceNodeKind::BehaviorSnapshot => RustAssuranceNodeKind::BehaviorSnapshot,
        RustEvidenceNodeKind::DeterminismReadiness => RustAssuranceNodeKind::DeterminismReadiness,
        RustEvidenceNodeKind::FormalProofPilot => RustAssuranceNodeKind::FormalProofPilot,
        RustEvidenceNodeKind::ReviewPacket => RustAssuranceNodeKind::ReviewPacket,
        RustEvidenceNodeKind::Waiver => RustAssuranceNodeKind::Waiver,
        RustEvidenceNodeKind::ReviewAction => RustAssuranceNodeKind::ReviewAction,
    }
}

fn assurance_node_status(status: RustEvidenceNodeStatus) -> RustAssuranceNodeStatus {
    match status {
        RustEvidenceNodeStatus::Current => RustAssuranceNodeStatus::Current,
        RustEvidenceNodeStatus::Changed => RustAssuranceNodeStatus::Changed,
        RustEvidenceNodeStatus::Missing => RustAssuranceNodeStatus::Missing,
        RustEvidenceNodeStatus::Stale => RustAssuranceNodeStatus::Stale,
        RustEvidenceNodeStatus::Expired => RustAssuranceNodeStatus::Expired,
        RustEvidenceNodeStatus::Ready => RustAssuranceNodeStatus::Ready,
        RustEvidenceNodeStatus::NeedsInjection => RustAssuranceNodeStatus::NeedsInjection,
        RustEvidenceNodeStatus::Blocked => RustAssuranceNodeStatus::Blocked,
        RustEvidenceNodeStatus::Unknown => RustAssuranceNodeStatus::Unknown,
        RustEvidenceNodeStatus::Proved => RustAssuranceNodeStatus::Proved,
        RustEvidenceNodeStatus::ProvedBounded => RustAssuranceNodeStatus::ProvedBounded,
        RustEvidenceNodeStatus::Failed => RustAssuranceNodeStatus::Failed,
        RustEvidenceNodeStatus::Skipped => RustAssuranceNodeStatus::Skipped,
        RustEvidenceNodeStatus::Error => RustAssuranceNodeStatus::Error,
    }
}

fn string_fields(fields: &[(&str, Option<&str>)]) -> BTreeMap<String, String> {
    fields
        .iter()
        .filter_map(|(key, value)| value.map(|value| ((*key).to_owned(), value.to_owned())))
        .collect()
}

fn sanitize_id_part(raw: &str) -> String {
    let mut output = String::new();
    for character in raw.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | ':' | '-') {
            output.push(character.to_ascii_lowercase());
        } else {
            output.push('.');
        }
    }
    while output.contains("..") {
        output = output.replace("..", ".");
    }
    output.trim_matches('.').to_owned()
}
