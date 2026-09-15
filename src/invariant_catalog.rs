//! Invariant candidates derived from policy findings.

use std::collections::BTreeMap;

use crate::model::{
    AspRustFinding, RustInvariantCandidate, RustInvariantCandidateStatus, RustInvariantEvidence,
    RustInvariantEvidenceKind, RustInvariantId, RustInvariantKind, RustInvariantReceiptKind,
    RustInvariantRulePackId, RustInvariantSourceRuleId,
};

/// Derive machine-facing invariant candidates from configured findings.
pub(crate) fn invariant_candidates_from_findings(
    findings: &[AspRustFinding],
) -> Vec<RustInvariantCandidate> {
    findings
        .iter()
        .filter_map(invariant_candidate_from_finding)
        .collect()
}

fn invariant_candidate_from_finding(finding: &AspRustFinding) -> Option<RustInvariantCandidate> {
    let spec = invariant_spec(&finding.rule_id)?;
    let mut evidence_fields = BTreeMap::new();
    evidence_fields.insert("requirement".to_owned(), finding.requirement.clone());
    evidence_fields.insert("label".to_owned(), finding.label.clone());
    let mut fields = finding.labels.clone();
    fields.insert("sourceRuleId".to_owned(), finding.rule_id.clone());

    Some(RustInvariantCandidate {
        invariant_id: RustInvariantId(invariant_id(finding)),
        source_rule_id: RustInvariantSourceRuleId(finding.rule_id.clone()),
        rule_pack_id: RustInvariantRulePackId(finding.pack_id.clone()),
        kind: spec.kind,
        status: RustInvariantCandidateStatus::Candidate,
        severity: finding.severity,
        title: finding.title.clone(),
        hypothesis: spec.hypothesis.to_owned(),
        location: finding.location.clone(),
        evidence: vec![RustInvariantEvidence {
            kind: RustInvariantEvidenceKind::Finding,
            summary: finding.summary.clone(),
            location: Some(finding.location.clone()),
            fields: evidence_fields,
        }],
        required_receipts: spec.required_receipts.to_vec(),
        proof_targets: spec.proof_targets.to_vec(),
        fields,
    })
}

fn invariant_id(finding: &AspRustFinding) -> String {
    let path = finding
        .location
        .path
        .as_ref()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| "unknown".to_owned());
    sanitize_candidate_id(&format!(
        "{}:{}:{}",
        finding.rule_id, path, finding.location.line
    ))
}

fn sanitize_candidate_id(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'A'..='Z' => ch.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | '-' | '_' | '.' | ':' => ch,
            _ => '.',
        })
        .collect()
}

#[derive(Clone, Copy)]
struct InvariantSpec {
    kind: RustInvariantKind,
    hypothesis: &'static str,
    required_receipts: &'static [RustInvariantReceiptKind],
    proof_targets: &'static [RustInvariantKind],
}

fn invariant_spec(rule_id: &str) -> Option<InvariantSpec> {
    match rule_id {
        "RUST-AGENT-API-TYPE-012" => Some(InvariantSpec {
            kind: RustInvariantKind::PrimitiveIdentifierBoundary,
            hypothesis: "Public identifier parameters should cross API boundaries as owner-named types or documented raw boundaries.",
            required_receipts: BASE_RECEIPTS,
            proof_targets: &[RustInvariantKind::PublicApiShape],
        }),
        "RUST-AGENT-DATA-FIELD-020" => Some(InvariantSpec {
            kind: RustInvariantKind::PublicDataPrimitiveFields,
            hypothesis: "Public data structs with several semantic primitive fields should expose named domain fields or documented raw DTO boundaries.",
            required_receipts: BASE_RECEIPTS,
            proof_targets: &[RustInvariantKind::PublicApiShape],
        }),
        "RUST-AGENT-API-SHAPE-023" => Some(InvariantSpec {
            kind: RustInvariantKind::AnonymousTupleApiSurface,
            hypothesis: "Public APIs should not expose anonymous tuple payloads when named structs, enums, or newtypes can carry field intent.",
            required_receipts: BEHAVIOR_RECEIPTS,
            proof_targets: &[RustInvariantKind::PublicApiShape],
        }),
        "RUST-AGENT-API-TYPE-ALIAS-027" => Some(InvariantSpec {
            kind: RustInvariantKind::PrimitiveTypeAliasBoundary,
            hypothesis: "Public semantic aliases over primitive carriers should become named newtype or struct boundaries.",
            required_receipts: BASE_RECEIPTS,
            proof_targets: &[RustInvariantKind::PublicApiShape],
        }),
        "RUST-AGENT-DATA-STATE-028" => Some(InvariantSpec {
            kind: RustInvariantKind::StringlyStateBoundary,
            hypothesis: "Public stringly state fields should become enums, newtypes, or typed catalog boundaries.",
            required_receipts: BEHAVIOR_RECEIPTS,
            proof_targets: &[RustInvariantKind::PublicApiShape],
        }),
        _ => None,
    }
}

const BASE_RECEIPTS: &[RustInvariantReceiptKind] = &[
    RustInvariantReceiptKind::CargoCheck,
    RustInvariantReceiptKind::CargoTest,
    RustInvariantReceiptKind::Clippy,
];

const BEHAVIOR_RECEIPTS: &[RustInvariantReceiptKind] = &[
    RustInvariantReceiptKind::CargoCheck,
    RustInvariantReceiptKind::CargoTest,
    RustInvariantReceiptKind::Clippy,
    RustInvariantReceiptKind::ExpectTest,
];
