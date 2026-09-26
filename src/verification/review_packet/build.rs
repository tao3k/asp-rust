//! Builder for `ReviewPacket` reviewer summaries.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::{
    RustBehaviorSnapshot, RustBehaviorSnapshotStatus, RustDeterminismReadiness,
    RustDeterminismReadinessStatus, RustDiagnosticSeverity, RustFormalProofPilot,
    RustFormalProofPilotStatus, RustInvariantCandidate, RustInvariantCandidateStatus,
    RustInvariantKind, RustInvariantReceiptKind, RustVerificationExecutionReceipt,
    RustVerificationExecutionStatus, RustVerificationExecutionTool, SourceLocation,
};

use super::model::{
    RUST_REVIEW_PACKET_PROTOCOL_ID, RUST_REVIEW_PACKET_PROTOCOL_VERSION,
    RUST_REVIEW_PACKET_SCHEMA_ID, RUST_REVIEW_PACKET_SCHEMA_VERSION, RustReviewPacket,
    RustReviewPacketAction, RustReviewPacketActionKind, RustReviewPacketActionPriority,
    RustReviewPacketBehaviorStatus, RustReviewPacketChangedBehavior,
    RustReviewPacketChangedInvariant, RustReviewPacketDeterminismStatus,
    RustReviewPacketDeterminismSummary, RustReviewPacketId, RustReviewPacketInput,
    RustReviewPacketInvariantKind, RustReviewPacketLocation, RustReviewPacketMissingReceipt,
    RustReviewPacketProducer, RustReviewPacketProject, RustReviewPacketProofStatus,
    RustReviewPacketProofSummary, RustReviewPacketProtocolId, RustReviewPacketProtocolVersion,
    RustReviewPacketReceiptKind, RustReviewPacketSchemaId, RustReviewPacketSchemaVersion,
    RustReviewPacketSeverity, RustReviewPacketSummary, RustReviewPacketWaiver,
    RustReviewPacketWaiverStatus,
};

/// Build a reviewer-first packet from new evidence APIs.
#[must_use]
pub fn build_rust_review_packet(input: RustReviewPacketInput) -> RustReviewPacket {
    let changed_invariants = input
        .report
        .invariant_candidates
        .iter()
        .map(|candidate| changed_invariant(&input.project_root, candidate))
        .collect::<Vec<_>>();

    let receipt_evidence = passed_receipt_evidence(&input.receipts);
    let current_waivers = current_waiver_evidence(&input.waivers);
    let missing_receipts = input
        .report
        .invariant_candidates
        .iter()
        .flat_map(|candidate| {
            missing_receipts_for_candidate(candidate, &receipt_evidence, &current_waivers)
        })
        .collect::<Vec<_>>();

    let changed_behavior = input
        .behavior_snapshots
        .iter()
        .filter_map(changed_behavior)
        .collect::<Vec<_>>();
    let stale_waivers = input
        .waivers
        .into_iter()
        .filter(|waiver| waiver.status != RustReviewPacketWaiverStatus::Current)
        .collect::<Vec<_>>();
    let determinism_readiness = input
        .determinism_readiness
        .iter()
        .map(determinism_summary)
        .collect::<Vec<_>>();
    let proof_pilots = input
        .proof_pilots
        .iter()
        .map(proof_summary)
        .collect::<Vec<_>>();

    let summary = RustReviewPacketSummary {
        changed_invariants: changed_invariants.len(),
        changed_behavior: changed_behavior.len(),
        missing_receipts: missing_receipts.len(),
        stale_waivers: stale_waivers.len(),
        determinism_observations: determinism_readiness
            .iter()
            .map(|readiness| readiness.observations)
            .sum(),
        proof_claims: proof_pilots.iter().map(|proof| proof.claims).sum(),
        fields: BTreeMap::new(),
    };
    let review_actions = review_actions(
        &changed_invariants,
        &changed_behavior,
        &missing_receipts,
        &stale_waivers,
        &determinism_readiness,
        &proof_pilots,
    );

    RustReviewPacket {
        schema_id: RustReviewPacketSchemaId(RUST_REVIEW_PACKET_SCHEMA_ID.to_owned()),
        schema_version: RustReviewPacketSchemaVersion(RUST_REVIEW_PACKET_SCHEMA_VERSION.to_owned()),
        protocol_id: RustReviewPacketProtocolId(RUST_REVIEW_PACKET_PROTOCOL_ID.to_owned()),
        protocol_version: RustReviewPacketProtocolVersion(
            RUST_REVIEW_PACKET_PROTOCOL_VERSION.to_owned(),
        ),
        packet_id: RustReviewPacketId("rust.review.packet".to_owned()),
        producer: RustReviewPacketProducer {
            language_id: "rust".to_owned(),
            provider_id: "asp-rust".to_owned(),
            namespace: "agent.semantic-protocols.languages.rust.asp-rust".to_owned(),
        },
        project: RustReviewPacketProject {
            root: PathBuf::from("."),
            package: None,
            fields: BTreeMap::new(),
        },
        summary,
        changed_invariants,
        changed_behavior,
        missing_receipts,
        stale_waivers,
        determinism_readiness,
        proof_pilots,
        review_actions,
        fields: BTreeMap::new(),
    }
}

fn changed_invariant(
    project_root: &Path,
    candidate: &RustInvariantCandidate,
) -> RustReviewPacketChangedInvariant {
    let mut fields = BTreeMap::new();
    fields.insert("rulePackId".to_owned(), candidate.rule_pack_id.0.clone());
    fields.insert(
        "status".to_owned(),
        invariant_candidate_status(candidate.status).to_owned(),
    );

    RustReviewPacketChangedInvariant {
        invariant_id: candidate.invariant_id.0.clone(),
        source_rule_id: candidate.source_rule_id.0.clone(),
        kind: invariant_kind(candidate.kind),
        severity: severity(candidate.severity),
        title: candidate.title.clone(),
        hypothesis: candidate.hypothesis.clone(),
        location: review_location(project_root, &candidate.location),
        required_receipts: candidate
            .required_receipts
            .iter()
            .copied()
            .map(receipt_kind)
            .collect(),
        fields,
    }
}

fn review_location(project_root: &Path, location: &SourceLocation) -> RustReviewPacketLocation {
    RustReviewPacketLocation {
        path: location
            .path
            .as_ref()
            .and_then(|path| project_relative_path(project_root, path)),
        line: location.line.max(1),
        column: location.column,
    }
}

fn project_relative_path(project_root: &Path, path: &Path) -> Option<PathBuf> {
    if path == Path::new(".") {
        return Some(PathBuf::from("."));
    }
    if path.is_absolute() {
        return path
            .strip_prefix(project_root)
            .ok()
            .filter(|relative| !relative.as_os_str().is_empty())
            .map(Path::to_path_buf);
    }
    Some(path.to_path_buf())
}

fn passed_receipt_evidence(
    receipts: &[RustVerificationExecutionReceipt],
) -> BTreeSet<(String, RustReviewPacketReceiptKind)> {
    receipts
        .iter()
        .filter(|receipt| receipt.status == RustVerificationExecutionStatus::Passed)
        .flat_map(|receipt| {
            let kind = receipt_tool_kind(receipt.tool);
            receipt
                .candidate_ids
                .iter()
                .map(move |candidate_id| (candidate_id.0.clone(), kind))
        })
        .collect()
}

fn current_waiver_evidence(
    waivers: &[RustReviewPacketWaiver],
) -> BTreeSet<(String, RustReviewPacketReceiptKind)> {
    waivers
        .iter()
        .filter(|waiver| waiver.status == RustReviewPacketWaiverStatus::Current)
        .map(|waiver| (waiver.invariant_id.clone(), waiver.receipt_kind))
        .collect()
}

fn missing_receipts_for_candidate(
    candidate: &RustInvariantCandidate,
    receipt_evidence: &BTreeSet<(String, RustReviewPacketReceiptKind)>,
    waiver_evidence: &BTreeSet<(String, RustReviewPacketReceiptKind)>,
) -> Vec<RustReviewPacketMissingReceipt> {
    let invariant_id = candidate.invariant_id.0.clone();
    candidate
        .required_receipts
        .iter()
        .copied()
        .map(receipt_kind)
        .filter(|kind| {
            let evidence_key = (invariant_id.clone(), *kind);
            !receipt_evidence.contains(&evidence_key) && !waiver_evidence.contains(&evidence_key)
        })
        .map(|kind| RustReviewPacketMissingReceipt {
            invariant_id: invariant_id.clone(),
            receipt_kind: kind,
            reason: format!(
                "no passed {} receipt linked to candidate",
                receipt_kind_id(kind)
            ),
            fields: BTreeMap::new(),
        })
        .collect()
}

fn changed_behavior(snapshot: &RustBehaviorSnapshot) -> Option<RustReviewPacketChangedBehavior> {
    let status = match snapshot.status {
        RustBehaviorSnapshotStatus::Matched => return None,
        RustBehaviorSnapshotStatus::Changed => RustReviewPacketBehaviorStatus::Changed,
        RustBehaviorSnapshotStatus::Missing => RustReviewPacketBehaviorStatus::Missing,
        RustBehaviorSnapshotStatus::Skipped => RustReviewPacketBehaviorStatus::Skipped,
        RustBehaviorSnapshotStatus::Error => RustReviewPacketBehaviorStatus::Error,
    };
    let summary = snapshot.observations.first().map_or_else(
        || "behavior snapshot needs review".to_owned(),
        |observation| observation.message.0.clone(),
    );
    Some(RustReviewPacketChangedBehavior {
        snapshot_id: snapshot.snapshot_id.0.clone(),
        status,
        subject: behavior_subject(snapshot),
        summary,
        receipt_ids: snapshot.receipt_ids.clone(),
        candidate_ids: snapshot
            .candidate_ids
            .iter()
            .map(|candidate_id| candidate_id.0.clone())
            .collect(),
        fields: BTreeMap::new(),
    })
}

fn behavior_subject(snapshot: &RustBehaviorSnapshot) -> String {
    match &snapshot.subject.symbol {
        Some(symbol) => format!("{}::{}", snapshot.subject.path.display(), symbol.0),
        None => snapshot.subject.path.display().to_string(),
    }
}

fn determinism_summary(readiness: &RustDeterminismReadiness) -> RustReviewPacketDeterminismSummary {
    RustReviewPacketDeterminismSummary {
        readiness_id: readiness.readiness_id.0.clone(),
        status: determinism_status(readiness.status),
        observations: readiness.observations.len(),
        suggestions: readiness.suggestions.len(),
        fields: BTreeMap::new(),
    }
}

fn proof_summary(proof: &RustFormalProofPilot) -> RustReviewPacketProofSummary {
    RustReviewPacketProofSummary {
        proof_id: proof.proof_id.0.clone(),
        target: proof.target.name.0.clone(),
        status: proof_status(proof.status),
        claims: proof.claims.len(),
        checks: proof.checks.len(),
        fields: BTreeMap::new(),
    }
}

fn review_actions(
    changed_invariants: &[RustReviewPacketChangedInvariant],
    changed_behavior: &[RustReviewPacketChangedBehavior],
    missing_receipts: &[RustReviewPacketMissingReceipt],
    stale_waivers: &[RustReviewPacketWaiver],
    determinism_readiness: &[RustReviewPacketDeterminismSummary],
    proof_pilots: &[RustReviewPacketProofSummary],
) -> Vec<RustReviewPacketAction> {
    let mut actions = Vec::new();
    for invariant in changed_invariants {
        actions.push(RustReviewPacketAction {
            action_id: format!(
                "verify-invariant.{}",
                sanitize_id_part(&invariant.invariant_id)
            ),
            kind: RustReviewPacketActionKind::VerifyInvariant,
            priority: priority_for_severity(invariant.severity),
            summary: format!("Review invariant {}", invariant.invariant_id),
            target_id: Some(invariant.invariant_id.clone()),
            fields: BTreeMap::new(),
        });
    }
    for missing in missing_receipts {
        actions.push(RustReviewPacketAction {
            action_id: format!(
                "run-receipt.{}.{}",
                sanitize_id_part(&missing.invariant_id),
                receipt_kind_id(missing.receipt_kind)
            ),
            kind: RustReviewPacketActionKind::RunReceipt,
            priority: RustReviewPacketActionPriority::P0,
            summary: format!(
                "Run {} for {}",
                receipt_kind_id(missing.receipt_kind),
                missing.invariant_id
            ),
            target_id: Some(missing.invariant_id.clone()),
            fields: BTreeMap::new(),
        });
    }
    for behavior in changed_behavior {
        actions.push(RustReviewPacketAction {
            action_id: format!(
                "inspect-behavior.{}",
                sanitize_id_part(&behavior.snapshot_id)
            ),
            kind: RustReviewPacketActionKind::InspectBehavior,
            priority: RustReviewPacketActionPriority::P1,
            summary: format!("Inspect behavior snapshot {}", behavior.snapshot_id),
            target_id: Some(behavior.snapshot_id.clone()),
            fields: BTreeMap::new(),
        });
    }
    for waiver in stale_waivers {
        actions.push(RustReviewPacketAction {
            action_id: format!("refresh-waiver.{}", sanitize_id_part(&waiver.waiver_id)),
            kind: RustReviewPacketActionKind::RefreshWaiver,
            priority: RustReviewPacketActionPriority::P1,
            summary: format!("Refresh waiver {}", waiver.waiver_id),
            target_id: Some(waiver.waiver_id.clone()),
            fields: BTreeMap::new(),
        });
    }
    for readiness in determinism_readiness {
        if readiness.status == RustReviewPacketDeterminismStatus::NeedsInjection {
            actions.push(RustReviewPacketAction {
                action_id: format!(
                    "address-determinism.{}",
                    sanitize_id_part(&readiness.readiness_id)
                ),
                kind: RustReviewPacketActionKind::AddressDeterminism,
                priority: RustReviewPacketActionPriority::P1,
                summary: format!(
                    "Address {} determinism observations",
                    readiness.observations
                ),
                target_id: Some(readiness.readiness_id.clone()),
                fields: BTreeMap::new(),
            });
        }
    }
    for proof in proof_pilots {
        if proof.status == RustReviewPacketProofStatus::Failed {
            actions.push(RustReviewPacketAction {
                action_id: format!("inspect-proof.{}", sanitize_id_part(&proof.proof_id)),
                kind: RustReviewPacketActionKind::InspectProof,
                priority: RustReviewPacketActionPriority::P0,
                summary: format!("Inspect failed proof pilot {}", proof.proof_id),
                target_id: Some(proof.proof_id.clone()),
                fields: BTreeMap::new(),
            });
        }
    }
    actions
}

fn sanitize_id_part(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    sanitized.trim_matches('-').to_owned()
}

fn priority_for_severity(severity: RustReviewPacketSeverity) -> RustReviewPacketActionPriority {
    match severity {
        RustReviewPacketSeverity::Error | RustReviewPacketSeverity::Warning => {
            RustReviewPacketActionPriority::P1
        }
        RustReviewPacketSeverity::Info => RustReviewPacketActionPriority::P2,
    }
}

fn severity(severity: RustDiagnosticSeverity) -> RustReviewPacketSeverity {
    match severity {
        RustDiagnosticSeverity::Info => RustReviewPacketSeverity::Info,
        RustDiagnosticSeverity::Warning => RustReviewPacketSeverity::Warning,
        RustDiagnosticSeverity::Error => RustReviewPacketSeverity::Error,
    }
}

fn invariant_kind(kind: RustInvariantKind) -> RustReviewPacketInvariantKind {
    match kind {
        RustInvariantKind::PrimitiveIdentifierBoundary => {
            RustReviewPacketInvariantKind::PrimitiveIdentifierBoundary
        }
        RustInvariantKind::PublicDataPrimitiveFields => {
            RustReviewPacketInvariantKind::PublicDataPrimitiveFields
        }
        RustInvariantKind::AnonymousTupleApiSurface => {
            RustReviewPacketInvariantKind::AnonymousTupleApiSurface
        }
        RustInvariantKind::PrimitiveTypeAliasBoundary => {
            RustReviewPacketInvariantKind::PrimitiveTypeAliasBoundary
        }
        RustInvariantKind::StringlyStateBoundary => {
            RustReviewPacketInvariantKind::StringlyStateBoundary
        }
        RustInvariantKind::ParserFact => RustReviewPacketInvariantKind::ParserFact,
        RustInvariantKind::PublicApiShape => RustReviewPacketInvariantKind::PublicApiShape,
        RustInvariantKind::ModuleReasoningTree => {
            RustReviewPacketInvariantKind::ModuleReasoningTree
        }
        RustInvariantKind::DependencyGraphAcyclicity => {
            RustReviewPacketInvariantKind::DependencyGraphAcyclicity
        }
        RustInvariantKind::Custom => RustReviewPacketInvariantKind::Custom,
    }
}

fn receipt_kind(kind: RustInvariantReceiptKind) -> RustReviewPacketReceiptKind {
    match kind {
        RustInvariantReceiptKind::CargoCheck => RustReviewPacketReceiptKind::CargoCheck,
        RustInvariantReceiptKind::CargoTest => RustReviewPacketReceiptKind::CargoTest,
        RustInvariantReceiptKind::Clippy => RustReviewPacketReceiptKind::Clippy,
        RustInvariantReceiptKind::ExpectTest => RustReviewPacketReceiptKind::ExpectTest,
        RustInvariantReceiptKind::Proptest => RustReviewPacketReceiptKind::Proptest,
        RustInvariantReceiptKind::CargoFuzz => RustReviewPacketReceiptKind::CargoFuzz,
        RustInvariantReceiptKind::Kani => RustReviewPacketReceiptKind::Kani,
        RustInvariantReceiptKind::Creusot => RustReviewPacketReceiptKind::Creusot,
        RustInvariantReceiptKind::Verus => RustReviewPacketReceiptKind::Verus,
        RustInvariantReceiptKind::Waiver => RustReviewPacketReceiptKind::Waiver,
    }
}

fn receipt_tool_kind(tool: RustVerificationExecutionTool) -> RustReviewPacketReceiptKind {
    match tool {
        RustVerificationExecutionTool::CargoCheck => RustReviewPacketReceiptKind::CargoCheck,
        RustVerificationExecutionTool::CargoTest => RustReviewPacketReceiptKind::CargoTest,
        RustVerificationExecutionTool::Clippy => RustReviewPacketReceiptKind::Clippy,
        RustVerificationExecutionTool::ExpectTest => RustReviewPacketReceiptKind::ExpectTest,
        RustVerificationExecutionTool::Proptest => RustReviewPacketReceiptKind::Proptest,
        RustVerificationExecutionTool::CargoFuzz => RustReviewPacketReceiptKind::CargoFuzz,
        RustVerificationExecutionTool::Kani => RustReviewPacketReceiptKind::Kani,
        RustVerificationExecutionTool::Creusot => RustReviewPacketReceiptKind::Creusot,
        RustVerificationExecutionTool::Verus => RustReviewPacketReceiptKind::Verus,
    }
}

fn receipt_kind_id(kind: RustReviewPacketReceiptKind) -> &'static str {
    match kind {
        RustReviewPacketReceiptKind::CargoCheck => "cargo-check",
        RustReviewPacketReceiptKind::CargoTest => "cargo-test",
        RustReviewPacketReceiptKind::Clippy => "clippy",
        RustReviewPacketReceiptKind::ExpectTest => "expect-test",
        RustReviewPacketReceiptKind::Proptest => "proptest",
        RustReviewPacketReceiptKind::CargoFuzz => "cargo-fuzz",
        RustReviewPacketReceiptKind::Kani => "kani",
        RustReviewPacketReceiptKind::Creusot => "creusot",
        RustReviewPacketReceiptKind::Verus => "verus",
        RustReviewPacketReceiptKind::Waiver => "waiver",
    }
}

fn invariant_candidate_status(status: RustInvariantCandidateStatus) -> &'static str {
    match status {
        RustInvariantCandidateStatus::Candidate => "candidate",
        RustInvariantCandidateStatus::Accepted => "accepted",
        RustInvariantCandidateStatus::Verified => "verified",
        RustInvariantCandidateStatus::Waived => "waived",
        RustInvariantCandidateStatus::Stale => "stale",
    }
}

fn determinism_status(status: RustDeterminismReadinessStatus) -> RustReviewPacketDeterminismStatus {
    match status {
        RustDeterminismReadinessStatus::Ready => RustReviewPacketDeterminismStatus::Ready,
        RustDeterminismReadinessStatus::NeedsInjection => {
            RustReviewPacketDeterminismStatus::NeedsInjection
        }
        RustDeterminismReadinessStatus::Blocked => RustReviewPacketDeterminismStatus::Blocked,
        RustDeterminismReadinessStatus::Unknown => RustReviewPacketDeterminismStatus::Unknown,
    }
}

fn proof_status(status: RustFormalProofPilotStatus) -> RustReviewPacketProofStatus {
    match status {
        RustFormalProofPilotStatus::Proved => RustReviewPacketProofStatus::Proved,
        RustFormalProofPilotStatus::ProvedBounded => RustReviewPacketProofStatus::ProvedBounded,
        RustFormalProofPilotStatus::Failed => RustReviewPacketProofStatus::Failed,
        RustFormalProofPilotStatus::Skipped => RustReviewPacketProofStatus::Skipped,
        RustFormalProofPilotStatus::Unknown => RustReviewPacketProofStatus::Unknown,
    }
}
