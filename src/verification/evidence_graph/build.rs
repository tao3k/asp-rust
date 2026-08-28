//! Build `semantic-evidence-graph` packets from review packet evidence.

use super::model::{
    RUST_EVIDENCE_GRAPH_PROTOCOL_ID, RUST_EVIDENCE_GRAPH_PROTOCOL_VERSION,
    RUST_EVIDENCE_GRAPH_SCHEMA_ID, RUST_EVIDENCE_GRAPH_SCHEMA_VERSION, RustEvidenceEdge,
    RustEvidenceEdgeKind, RustEvidenceGap, RustEvidenceGraph, RustEvidenceGraphProducer,
    RustEvidenceGraphProject, RustEvidenceGraphSummary, RustEvidenceLocation, RustEvidenceNode,
    RustEvidenceNodeKind, RustEvidenceNodeStatus,
};
use crate::verification::review_packet::RustReviewPacket;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Input for building a Rust evidence graph.
#[derive(Debug, Clone)]
pub struct RustEvidenceGraphInput {
    pub project_root: PathBuf,
    pub review_packets: Vec<RustReviewPacket>,
}

/// Build an evidence graph from review packets.
#[must_use]
pub fn build_rust_evidence_graph(input: RustEvidenceGraphInput) -> RustEvidenceGraph {
    let mut builder = RustEvidenceGraphBuilder::new(input.project_root);
    for packet in input.review_packets {
        builder.add_review_packet(&packet);
    }
    builder.finish()
}

struct RustEvidenceGraphBuilder {
    project_root: PathBuf,
    nodes: Vec<RustEvidenceNode>,
    edges: Vec<RustEvidenceEdge>,
    gaps: Vec<RustEvidenceGap>,
    node_ids: BTreeSet<String>,
    edge_ids: BTreeSet<String>,
    owner_paths: BTreeSet<String>,
    stale_items: usize,
    claims: usize,
}

impl RustEvidenceGraphBuilder {
    fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            nodes: Vec::new(),
            edges: Vec::new(),
            gaps: Vec::new(),
            node_ids: BTreeSet::new(),
            edge_ids: BTreeSet::new(),
            owner_paths: BTreeSet::new(),
            stale_items: 0,
            claims: 0,
        }
    }

    fn add_review_packet(&mut self, packet: &RustReviewPacket) {
        let packet_value = serde_json::to_value(packet).unwrap_or(Value::Null);
        let packet_id = string_field(&packet_value, "packetId")
            .unwrap_or_else(|| "rust.review.packet".to_owned());
        let packet_node_id = node_id("review-packet", &packet_id);
        self.insert_node(RustEvidenceNode {
            node_id: packet_node_id.clone(),
            kind: RustEvidenceNodeKind::ReviewPacket,
            label: packet_id.clone(),
            owner_path: None,
            candidate_id: None,
            receipt_id: None,
            snapshot_id: None,
            readiness_id: None,
            proof_id: None,
            packet_id: Some(packet_id),
            waiver_id: None,
            action_id: None,
            status: Some(RustEvidenceNodeStatus::Current),
            summary: Some("review packet".to_owned()),
            location: None,
            fields: BTreeMap::new(),
        });

        let invariant_nodes = self.add_changed_invariants(&packet_value, &packet_node_id);
        self.add_changed_behavior(&packet_value, &packet_node_id, &invariant_nodes);
        self.add_missing_receipts(&packet_value, &packet_node_id, &invariant_nodes);
        self.add_stale_waivers(&packet_value, &packet_node_id, &invariant_nodes);
        self.add_determinism_readiness(&packet_value, &packet_node_id);
        self.add_proof_pilots(&packet_value, &packet_node_id);
        self.add_review_actions(&packet_value, &packet_node_id, &invariant_nodes);
    }

    fn add_changed_invariants(
        &mut self,
        packet_value: &Value,
        packet_node_id: &str,
    ) -> BTreeMap<String, String> {
        let mut invariant_nodes = BTreeMap::new();
        for invariant in array_field(packet_value, "changedInvariants") {
            let Some(invariant_id) = string_field(invariant, "invariantId") else {
                continue;
            };
            let invariant_node_id = node_id("invariant", &invariant_id);
            let location = location_field(invariant);
            let owner_path = location.as_ref().and_then(|location| location.path.clone());
            if let Some(owner_path) = owner_path.as_ref() {
                self.add_owner_node(owner_path);
            }
            let title = string_field(invariant, "title").unwrap_or_else(|| invariant_id.clone());
            let summary = string_field(invariant, "hypothesis");
            self.insert_node(RustEvidenceNode {
                node_id: invariant_node_id.clone(),
                kind: RustEvidenceNodeKind::InvariantCandidate,
                label: title,
                owner_path: owner_path.clone(),
                candidate_id: Some(invariant_id.clone()),
                receipt_id: None,
                snapshot_id: None,
                readiness_id: None,
                proof_id: None,
                packet_id: None,
                waiver_id: None,
                action_id: None,
                status: Some(RustEvidenceNodeStatus::Changed),
                summary,
                location,
                fields: string_fields(invariant, &["sourceRuleId", "kind", "severity"]),
            });
            self.insert_edge(
                RustEvidenceEdgeKind::DerivedFrom,
                &invariant_node_id,
                packet_node_id,
                None,
            );
            if let Some(owner_path) = owner_path {
                self.insert_edge(
                    RustEvidenceEdgeKind::DerivedFrom,
                    &invariant_node_id,
                    &node_id("owner", &owner_path),
                    Some("owner"),
                );
            }
            invariant_nodes.insert(invariant_id, invariant_node_id);
        }
        invariant_nodes
    }

    fn add_changed_behavior(
        &mut self,
        packet_value: &Value,
        packet_node_id: &str,
        invariant_nodes: &BTreeMap<String, String>,
    ) {
        for behavior in array_field(packet_value, "changedBehavior") {
            let Some(snapshot_id) = string_field(behavior, "snapshotId") else {
                continue;
            };
            let behavior_node_id = node_id("behavior-snapshot", &snapshot_id);
            self.insert_node(RustEvidenceNode {
                node_id: behavior_node_id.clone(),
                kind: RustEvidenceNodeKind::BehaviorSnapshot,
                label: string_field(behavior, "subject").unwrap_or_else(|| snapshot_id.clone()),
                owner_path: None,
                candidate_id: None,
                receipt_id: None,
                snapshot_id: Some(snapshot_id),
                readiness_id: None,
                proof_id: None,
                packet_id: None,
                waiver_id: None,
                action_id: None,
                status: status_from_string(string_field(behavior, "status").as_deref()),
                summary: string_field(behavior, "summary"),
                location: None,
                fields: BTreeMap::new(),
            });
            self.insert_edge(
                RustEvidenceEdgeKind::DerivedFrom,
                &behavior_node_id,
                packet_node_id,
                None,
            );
            for candidate_id in string_array_field(behavior, "candidateIds") {
                if let Some(invariant_node_id) = invariant_nodes.get(&candidate_id) {
                    self.insert_edge(
                        RustEvidenceEdgeKind::ObservedBy,
                        invariant_node_id,
                        &behavior_node_id,
                        None,
                    );
                }
            }
            for receipt_id in string_array_field(behavior, "receiptIds") {
                let receipt_node_id = self.add_receipt_node(&receipt_id);
                self.insert_edge(
                    RustEvidenceEdgeKind::ObservedBy,
                    &behavior_node_id,
                    &receipt_node_id,
                    Some("receipt"),
                );
            }
        }
    }

    fn add_missing_receipts(
        &mut self,
        packet_value: &Value,
        packet_node_id: &str,
        invariant_nodes: &BTreeMap<String, String>,
    ) {
        for missing in array_field(packet_value, "missingReceipts") {
            let Some(invariant_id) = string_field(missing, "invariantId") else {
                continue;
            };
            let receipt_kind =
                string_field(missing, "receiptKind").unwrap_or_else(|| "receipt".to_owned());
            let owner_path = invariant_nodes
                .get(&invariant_id)
                .and_then(|node_id| self.owner_path_for_node(node_id));
            self.gaps.push(RustEvidenceGap {
                gap_id: node_id("gap", &format!("{invariant_id}.{receipt_kind}")),
                owner_path,
                summary: string_field(missing, "reason")
                    .unwrap_or_else(|| format!("missing {receipt_kind} receipt")),
                severity: Some("warning".to_owned()),
                fields: string_fields(missing, &["invariantId", "receiptKind"]),
            });
            if let Some(invariant_node_id) = invariant_nodes.get(&invariant_id) {
                self.insert_edge(
                    RustEvidenceEdgeKind::RequiresEvidence,
                    invariant_node_id,
                    packet_node_id,
                    Some(&receipt_kind),
                );
            }
        }
    }

    fn add_stale_waivers(
        &mut self,
        packet_value: &Value,
        packet_node_id: &str,
        invariant_nodes: &BTreeMap<String, String>,
    ) {
        for waiver in array_field(packet_value, "staleWaivers") {
            let Some(waiver_id) = string_field(waiver, "waiverId") else {
                continue;
            };
            let waiver_node_id = node_id("waiver", &waiver_id);
            let status = status_from_string(string_field(waiver, "status").as_deref())
                .unwrap_or(RustEvidenceNodeStatus::Stale);
            if matches!(
                status,
                RustEvidenceNodeStatus::Stale | RustEvidenceNodeStatus::Expired
            ) {
                self.stale_items += 1;
            }
            self.insert_node(RustEvidenceNode {
                node_id: waiver_node_id.clone(),
                kind: RustEvidenceNodeKind::Waiver,
                label: waiver_id.clone(),
                owner_path: None,
                candidate_id: None,
                receipt_id: None,
                snapshot_id: None,
                readiness_id: None,
                proof_id: None,
                packet_id: None,
                waiver_id: Some(waiver_id),
                action_id: None,
                status: Some(status),
                summary: string_field(waiver, "reason"),
                location: None,
                fields: string_fields(waiver, &["owner", "receiptKind", "expiresAt"]),
            });
            self.insert_edge(
                RustEvidenceEdgeKind::DerivedFrom,
                &waiver_node_id,
                packet_node_id,
                None,
            );
            if let Some(invariant_id) = string_field(waiver, "invariantId")
                && let Some(invariant_node_id) = invariant_nodes.get(&invariant_id)
            {
                self.insert_edge(
                    RustEvidenceEdgeKind::WaivedBy,
                    invariant_node_id,
                    &waiver_node_id,
                    None,
                );
            }
        }
    }

    fn add_determinism_readiness(&mut self, packet_value: &Value, packet_node_id: &str) {
        for readiness in array_field(packet_value, "determinismReadiness") {
            let Some(readiness_id) = string_field(readiness, "readinessId") else {
                continue;
            };
            let readiness_node_id = node_id("determinism-readiness", &readiness_id);
            self.insert_node(RustEvidenceNode {
                node_id: readiness_node_id.clone(),
                kind: RustEvidenceNodeKind::DeterminismReadiness,
                label: readiness_id.clone(),
                owner_path: None,
                candidate_id: None,
                receipt_id: None,
                snapshot_id: None,
                readiness_id: Some(readiness_id),
                proof_id: None,
                packet_id: None,
                waiver_id: None,
                action_id: None,
                status: status_from_string(string_field(readiness, "status").as_deref()),
                summary: Some(format!(
                    "observations={} suggestions={}",
                    number_field(readiness, "observations").unwrap_or(0),
                    number_field(readiness, "suggestions").unwrap_or(0)
                )),
                location: None,
                fields: BTreeMap::new(),
            });
            self.insert_edge(
                RustEvidenceEdgeKind::DerivedFrom,
                &readiness_node_id,
                packet_node_id,
                None,
            );
        }
    }

    fn add_proof_pilots(&mut self, packet_value: &Value, packet_node_id: &str) {
        for proof in array_field(packet_value, "proofPilots") {
            let Some(proof_id) = string_field(proof, "proofId") else {
                continue;
            };
            let proof_node_id = node_id("formal-proof-pilot", &proof_id);
            self.claims += number_field(proof, "claims").unwrap_or(0) as usize;
            self.insert_node(RustEvidenceNode {
                node_id: proof_node_id.clone(),
                kind: RustEvidenceNodeKind::FormalProofPilot,
                label: string_field(proof, "target").unwrap_or_else(|| proof_id.clone()),
                owner_path: None,
                candidate_id: None,
                receipt_id: None,
                snapshot_id: None,
                readiness_id: None,
                proof_id: Some(proof_id),
                packet_id: None,
                waiver_id: None,
                action_id: None,
                status: status_from_string(string_field(proof, "status").as_deref()),
                summary: Some(format!(
                    "claims={} checks={}",
                    number_field(proof, "claims").unwrap_or(0),
                    number_field(proof, "checks").unwrap_or(0)
                )),
                location: None,
                fields: BTreeMap::new(),
            });
            self.insert_edge(
                RustEvidenceEdgeKind::SupportsClaim,
                &proof_node_id,
                packet_node_id,
                None,
            );
        }
    }

    fn add_review_actions(
        &mut self,
        packet_value: &Value,
        packet_node_id: &str,
        invariant_nodes: &BTreeMap<String, String>,
    ) {
        for action in array_field(packet_value, "reviewActions") {
            let Some(action_id) = string_field(action, "actionId") else {
                continue;
            };
            let action_node_id = node_id("review-action", &action_id);
            self.insert_node(RustEvidenceNode {
                node_id: action_node_id.clone(),
                kind: RustEvidenceNodeKind::ReviewAction,
                label: string_field(action, "summary").unwrap_or_else(|| action_id.clone()),
                owner_path: None,
                candidate_id: None,
                receipt_id: None,
                snapshot_id: None,
                readiness_id: None,
                proof_id: None,
                packet_id: None,
                waiver_id: None,
                action_id: Some(action_id),
                status: Some(RustEvidenceNodeStatus::Missing),
                summary: string_field(action, "kind"),
                location: None,
                fields: string_fields(action, &["priority", "targetId"]),
            });
            self.insert_edge(
                RustEvidenceEdgeKind::SuggestsAction,
                packet_node_id,
                &action_node_id,
                None,
            );
            if let Some(target_id) = string_field(action, "targetId")
                && let Some(invariant_node_id) = invariant_nodes.get(&target_id)
            {
                self.insert_edge(
                    RustEvidenceEdgeKind::RequiresEvidence,
                    &action_node_id,
                    invariant_node_id,
                    None,
                );
            }
        }
    }

    fn add_owner_node(&mut self, owner_path: &str) {
        self.owner_paths.insert(owner_path.to_owned());
        self.insert_node(RustEvidenceNode {
            node_id: node_id("owner", owner_path),
            kind: RustEvidenceNodeKind::Owner,
            label: owner_path.to_owned(),
            owner_path: Some(owner_path.to_owned()),
            candidate_id: None,
            receipt_id: None,
            snapshot_id: None,
            readiness_id: None,
            proof_id: None,
            packet_id: None,
            waiver_id: None,
            action_id: None,
            status: Some(RustEvidenceNodeStatus::Current),
            summary: None,
            location: None,
            fields: BTreeMap::new(),
        });
    }

    fn add_receipt_node(&mut self, receipt_id: &str) -> String {
        let receipt_node_id = node_id("verification-receipt", receipt_id);
        self.insert_node(RustEvidenceNode {
            node_id: receipt_node_id.clone(),
            kind: RustEvidenceNodeKind::VerificationReceipt,
            label: receipt_id.to_owned(),
            owner_path: None,
            candidate_id: None,
            receipt_id: Some(receipt_id.to_owned()),
            snapshot_id: None,
            readiness_id: None,
            proof_id: None,
            packet_id: None,
            waiver_id: None,
            action_id: None,
            status: Some(RustEvidenceNodeStatus::Current),
            summary: None,
            location: None,
            fields: BTreeMap::new(),
        });
        receipt_node_id
    }

    fn insert_node(&mut self, node: RustEvidenceNode) {
        if self.node_ids.insert(node.node_id.clone()) {
            self.nodes.push(node);
        }
    }

    fn insert_edge(
        &mut self,
        kind: RustEvidenceEdgeKind,
        from_node_id: &str,
        to_node_id: &str,
        label: Option<&str>,
    ) {
        let edge_id = edge_id(from_node_id, to_node_id, kind, label);
        if self.edge_ids.insert(edge_id.clone()) {
            self.edges.push(RustEvidenceEdge {
                edge_id,
                kind,
                from_node_id: from_node_id.to_owned(),
                to_node_id: to_node_id.to_owned(),
                label: label.map(str::to_owned),
                fields: BTreeMap::new(),
            });
        }
    }

    fn owner_path_for_node(&self, node_id: &str) -> Option<String> {
        self.nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .and_then(|node| node.owner_path.clone())
    }

    fn finish(self) -> RustEvidenceGraph {
        let summary = RustEvidenceGraphSummary {
            nodes: self.nodes.len(),
            edges: self.edges.len(),
            owners: self.owner_paths.len(),
            claims: self.claims,
            stale_items: self.stale_items,
            gaps: self.gaps.len(),
        };
        RustEvidenceGraph {
            schema_id: RUST_EVIDENCE_GRAPH_SCHEMA_ID.to_owned(),
            schema_version: RUST_EVIDENCE_GRAPH_SCHEMA_VERSION.to_owned(),
            protocol_id: RUST_EVIDENCE_GRAPH_PROTOCOL_ID.to_owned(),
            protocol_version: RUST_EVIDENCE_GRAPH_PROTOCOL_VERSION.to_owned(),
            graph_id: "rust.evidence.graph".to_owned(),
            producer: RustEvidenceGraphProducer {
                language_id: "rust".to_owned(),
                provider_id: "asp-rust".to_owned(),
                namespace: "agent.semantic-protocols.languages.rust.asp-rust".to_owned(),
            },
            project: RustEvidenceGraphProject {
                root: self.project_root.display().to_string(),
                package: None,
                fields: BTreeMap::new(),
            },
            summary,
            nodes: self.nodes,
            edges: self.edges,
            gaps: self.gaps,
            fields: BTreeMap::new(),
        }
    }
}

fn array_field<'a>(value: &'a Value, field: &str) -> &'a [Value] {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn string_array_field(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn number_field(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(Value::as_u64)
}

fn location_field(value: &Value) -> Option<RustEvidenceLocation> {
    let location = value.get("location")?;
    Some(RustEvidenceLocation {
        path: string_field(location, "path").map(|path| normalize_project_path(&path)),
        line: number_field(location, "line"),
        column: number_field(location, "column"),
    })
}

fn string_fields(value: &Value, fields: &[&str]) -> BTreeMap<String, String> {
    fields
        .iter()
        .filter_map(|field| string_field(value, field).map(|value| ((*field).to_owned(), value)))
        .collect()
}

fn node_id(prefix: &str, raw: &str) -> String {
    format!("{prefix}:{}", sanitize_id_part(raw))
}

fn edge_id(
    from_node_id: &str,
    to_node_id: &str,
    kind: RustEvidenceEdgeKind,
    label: Option<&str>,
) -> String {
    let mut id = format!(
        "edge:{}:{}:{}",
        sanitize_id_part(kind_id(kind)),
        sanitize_id_part(from_node_id),
        sanitize_id_part(to_node_id)
    );
    if let Some(label) = label {
        id.push(':');
        id.push_str(&sanitize_id_part(label));
    }
    id
}

fn kind_id(kind: RustEvidenceEdgeKind) -> &'static str {
    match kind {
        RustEvidenceEdgeKind::DerivedFrom => "derived-from",
        RustEvidenceEdgeKind::RequiresEvidence => "requires-evidence",
        RustEvidenceEdgeKind::VerifiedBy => "verified-by",
        RustEvidenceEdgeKind::ObservedBy => "observed-by",
        RustEvidenceEdgeKind::WaivedBy => "waived-by",
        RustEvidenceEdgeKind::ReviewedBy => "reviewed-by",
        RustEvidenceEdgeKind::SuggestsAction => "suggests-action",
        RustEvidenceEdgeKind::SupportsClaim => "supports-claim",
    }
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

fn normalize_project_path(path: &str) -> String {
    let mut normalized = path;
    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest;
    }
    if normalized.is_empty() {
        ".".to_owned()
    } else {
        normalized.to_owned()
    }
}

fn status_from_string(status: Option<&str>) -> Option<RustEvidenceNodeStatus> {
    match status {
        Some("current") => Some(RustEvidenceNodeStatus::Current),
        Some("changed") => Some(RustEvidenceNodeStatus::Changed),
        Some("missing") => Some(RustEvidenceNodeStatus::Missing),
        Some("stale") => Some(RustEvidenceNodeStatus::Stale),
        Some("expired") => Some(RustEvidenceNodeStatus::Expired),
        Some("ready") => Some(RustEvidenceNodeStatus::Ready),
        Some("needs-injection") => Some(RustEvidenceNodeStatus::NeedsInjection),
        Some("blocked") => Some(RustEvidenceNodeStatus::Blocked),
        Some("unknown") => Some(RustEvidenceNodeStatus::Unknown),
        Some("proved") => Some(RustEvidenceNodeStatus::Proved),
        Some("proved-bounded") => Some(RustEvidenceNodeStatus::ProvedBounded),
        Some("failed") => Some(RustEvidenceNodeStatus::Failed),
        Some("skipped") => Some(RustEvidenceNodeStatus::Skipped),
        Some("error") => Some(RustEvidenceNodeStatus::Error),
        _ => None,
    }
}
