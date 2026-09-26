use asp_rust::{
    RustFormalProofPilotInput, RustFormalProofPilotStatus,
    build_rust_dependency_graph_acyclicity_proof_pilot,
};

#[test]
fn p4_dependency_graph_proof_pilot_checks_exhaustive_small_models() {
    let proof = build_rust_dependency_graph_acyclicity_proof_pilot(RustFormalProofPilotInput {
        max_nodes: 4,
    })
    .expect("proof pilot");

    assert_eq!(proof.status, RustFormalProofPilotStatus::ProvedBounded);
    assert_eq!(
        proof.target.rule_ids,
        vec!["RUST-AGENT-OWNER-GRAPH-009".to_string()]
    );
    assert_eq!(proof.checks[0].models_checked, Some(4166));
    assert_eq!(proof.checks[0].max_nodes, Some(4));
    assert!(proof.checks[0].counterexample.is_none());
}
