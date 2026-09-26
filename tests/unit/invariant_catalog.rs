use std::fs;

use asp_rust::{
    RustInvariantCandidateStatus, RustInvariantKind, RustInvariantReceiptKind,
    run_asp_rust_for_scope,
};

#[test]
fn p0_agent_policy_findings_emit_invariant_candidates() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "invariant-catalog-fixture"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write manifest");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::write(
        temp.path().join("src/lib.rs"),
        r#"
pub type UserId = String;

pub struct Account {
    pub user_id: String,
    pub tenant_id: u64,
    pub profile_url: String,
}

pub struct Session {
    pub status: String,
}

pub fn load_user(user_id: String) -> (String, u64) {
    (user_id, 1)
}
"#,
    )
    .expect("write source");

    let report = run_asp_rust_for_scope(temp.path(), asp_rust::AspRustRunScope::Package)
        .expect("run harness");
    let candidates = report.invariant_candidates;
    let kinds = candidates
        .iter()
        .map(|candidate| candidate.kind)
        .collect::<Vec<_>>();

    assert!(kinds.contains(&RustInvariantKind::PrimitiveIdentifierBoundary));
    assert!(kinds.contains(&RustInvariantKind::PublicDataPrimitiveFields));
    assert!(kinds.contains(&RustInvariantKind::AnonymousTupleApiSurface));
    assert!(kinds.contains(&RustInvariantKind::PrimitiveTypeAliasBoundary));
    assert!(kinds.contains(&RustInvariantKind::StringlyStateBoundary));

    let alias_candidate = candidates
        .iter()
        .find(|candidate| candidate.kind == RustInvariantKind::PrimitiveTypeAliasBoundary)
        .expect("alias candidate");
    assert_eq!(
        alias_candidate.source_rule_id.as_str(),
        "RUST-AGENT-API-TYPE-ALIAS-027"
    );
    assert_eq!(alias_candidate.rule_pack_id.as_str(), "rust.agent_policy");
    assert_eq!(
        alias_candidate.status,
        RustInvariantCandidateStatus::Candidate
    );
    assert!(
        alias_candidate
            .invariant_id
            .as_str()
            .starts_with("rust-agent-api-type-alias-027:")
    );
    assert!(
        alias_candidate
            .required_receipts
            .contains(&RustInvariantReceiptKind::CargoCheck)
    );
    assert!(
        alias_candidate
            .proof_targets
            .contains(&RustInvariantKind::PublicApiShape)
    );
}
