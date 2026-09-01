use asp_rust::{
    RustDiagnosticSeverity, rust_agent_policy_rules, rust_modularity_rules,
    rust_project_policy_rules, rust_rule_pack_descriptors, rust_syntax_rules,
};

#[test]
fn rule_pack_descriptors_expose_default_execution_order() {
    let packs = rust_rule_pack_descriptors();
    let pack_ids = packs.iter().map(|pack| pack.id).collect::<Vec<_>>();

    assert_eq!(
        pack_ids,
        vec![
            "rust.syntax",
            "rust.project_policy",
            "rust.modularity",
            "rust.agent_policy",
        ]
    );
    assert_eq!(packs[0].default_mode, "blocking");
    assert_eq!(packs[1].default_mode, "blocking");
    assert_eq!(packs[2].default_mode, "blocking");
    assert_eq!(packs[3].default_mode, "advisory");
    assert!(packs.iter().all(|pack| pack.version == "1"));
    assert!(
        packs
            .iter()
            .all(|pack| pack.domains.contains(&"rust") && !pack.domains.is_empty())
    );
}

#[test]
fn rule_catalogs_expose_stable_rule_ids() {
    let syntax_ids = rust_syntax_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<Vec<_>>();
    assert_eq!(syntax_ids, vec!["RUST-SYN-R001"]);

    let mut agent_ids = rust_agent_policy_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<Vec<_>>();
    agent_ids.sort_unstable();
    let mut expected_agent_ids = vec![
        "RUST-AGENT-DOCS-MODULE-001",
        "RUST-AGENT-DOCS-PUBLIC-002",
        "RUST-AGENT-SOURCE-NAMESPACE-003",
        "RUST-AGENT-API-NAME-004",
        "RUST-AGENT-API-FACADE-005",
        "RUST-AGENT-SOURCE-MODULE-006",
        "RUST-AGENT-SOURCE-PATH-007",
        "RUST-AGENT-DOCS-BRANCH-008",
        "RUST-AGENT-OWNER-GRAPH-009",
        "RUST-AGENT-OWNER-BOUNDARY-010",
        "RUST-AGENT-DOCS-OWNER-FANOUT-011",
        "RUST-AGENT-API-TYPE-012",
        "RUST-AGENT-API-ERROR-013",
        "RUST-AGENT-TEST-SUPPORT-014",
        "RUST-AGENT-CFG-PUBLIC-015",
        "RUST-AGENT-CFG-PUBLIC-016",
        "RUST-AGENT-ITER-PUBLIC-017",
        "RUST-AGENT-API-FLAGS-018",
        "RUST-AGENT-API-PARAMETERS-019",
        "RUST-AGENT-DATA-FIELD-020",
        "RUST-AGENT-DATA-ENUM-PAYLOAD-021",
        "RUST-AGENT-DATA-BOUNDS-022",
        "RUST-AGENT-DATA-ENUM-TUPLE-024",
        "RUST-AGENT-CFG-IMPL-025",
        "RUST-AGENT-ITER-IMPL-026",
        "RUST-AGENT-API-TYPE-ALIAS-027",
        "RUST-AGENT-DATA-STATE-028",
        "RUST-AGENT-DATA-MEMBERSHIP-029",
        "RUST-AGENT-ASYNC-BLOCKING-030",
        "RUST-AGENT-ASYNC-SYNC-LOCK-031",
        "RUST-AGENT-ASYNC-BACKPRESSURE-032",
        "RUST-AGENT-ASYNC-CANCEL-SAFETY-033",
        "RUST-AGENT-ASYNC-CANCEL-SAFETY-034",
        "RUST-AGENT-API-SHAPE-023",
        "RUST-AGENT-API-SHAPE-036",
        "RUST-AGENT-ASYNC-TASK-LIFECYCLE-001",
        "RUST-AGENT-NATIVE-ABI-001",
        "RUST-AGENT-PROC-001",
        "RUST-AGENT-TOKIO-RUNTIME-002",
    ];
    expected_agent_ids.sort_unstable();
    assert_eq!(agent_ids, expected_agent_ids);

    let project_ids = rust_project_policy_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<Vec<_>>();
    assert_eq!(
        project_ids,
        vec![
            "RUST-AGENT-PROJECT-001",
            "RUST-AGENT-PROJECT-002",
            "RUST-AGENT-PROJECT-003",
            "RUST-AGENT-PROJECT-004",
            "RUST-AGENT-PROJECT-005",
            "RUST-AGENT-PROJECT-006",
            "RUST-AGENT-PROJECT-007",
            "RUST-AGENT-PROJECT-008",
            "RUST-AGENT-PROJECT-009",
            "RUST-AGENT-PROJECT-010",
            "RUST-AGENT-PROJECT-011",
            "RUST-AGENT-PROJECT-012",
            "RUST-AGENT-PROJECT-013",
            "RUST-AGENT-PROJECT-014",
            "RUST-AGENT-PROJECT-015",
            "RUST-AGENT-PROJECT-016",
            "RUST-AGENT-PROJECT-017",
            "RUST-AGENT-PROJECT-018",
            "RUST-AGENT-PROJECT-019",
            "RUST-AGENT-PROJECT-020",
            "RUST-AGENT-PROJECT-021",
            "RUST-AGENT-PROJECT-022",
            "RUST-AGENT-PROJECT-024",
            "RUST-AGENT-PROJECT-MANIFEST-023",
        ]
    );

    let modularity_ids = rust_modularity_rules()
        .into_iter()
        .map(|rule| rule.rule_id)
        .collect::<Vec<_>>();
    assert_eq!(
        modularity_ids,
        vec![
            "RUST-MOD-R001",
            "RUST-MOD-R002",
            "RUST-MOD-R003",
            "RUST-MOD-R004",
            "RUST-MOD-R005",
            "RUST-MOD-R006",
            "RUST-MOD-R007",
            "RUST-MOD-R008",
            "RUST-MOD-R009",
            "RUST-MOD-R010",
            "RUST-MOD-R011",
        ]
    );
}

#[test]
fn rule_catalogs_keep_default_severities_aligned() {
    assert!(
        rust_syntax_rules()
            .into_iter()
            .all(|rule| rule.severity == RustDiagnosticSeverity::Error)
    );
    assert!(
        rust_project_policy_rules()
            .into_iter()
            .all(|rule| rule.severity == RustDiagnosticSeverity::Warning)
    );
    assert!(
        rust_modularity_rules()
            .into_iter()
            .all(|rule| rule.severity == RustDiagnosticSeverity::Warning)
    );
    assert!(
        rust_agent_policy_rules()
            .into_iter()
            .all(|rule| rule.severity == RustDiagnosticSeverity::Info)
    );
}
