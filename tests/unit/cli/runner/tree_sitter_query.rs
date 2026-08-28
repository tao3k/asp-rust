use super::{
    exact_owner_path, handle_syntax_query_operation_value, inline_eq_predicate,
    predicate_function_name,
};

#[test]
fn resident_syntax_query_executes_scm_plan_on_native_rust_facts() {
    use agent_semantic_provider_transport::{
        ProviderSyntaxQueryRequest, ProviderSyntaxQueryResponse, SyntaxQueryPattern,
        SyntaxQueryPlan,
    };

    let source = "pub fn run() {}\npub struct Record;\n";
    let request = ProviderSyntaxQueryRequest {
        schema_id: "agent.semantic-protocols.provider-syntax-query-request".to_owned(),
        schema_version: "1".to_owned(),
        language_id: "rust".to_owned(),
        provider_id: "asp-rust".to_owned(),
        owner_path: "src/lib.rs".to_owned(),
        source_content_digest: blake3::hash(source.as_bytes()).to_hex().to_string(),
        query_digest: "1".repeat(64),
        source: source.to_owned(),
        plan: SyntaxQueryPlan {
            patterns: vec![
                SyntaxQueryPattern {
                    index: 0,
                    captures: vec!["declaration.name".to_owned()],
                    node_types: vec!["function_item".to_owned()],
                    fields: vec!["name".to_owned()],
                },
                SyntaxQueryPattern {
                    index: 1,
                    captures: vec!["declaration.name".to_owned()],
                    node_types: vec!["struct_item".to_owned()],
                    fields: vec!["name".to_owned()],
                },
            ],
            captures: vec!["declaration.name".to_owned()],
            node_types: vec!["function_item".to_owned(), "struct_item".to_owned()],
            fields: vec!["name".to_owned()],
            predicates: vec![],
        },
    };
    let payload = serde_json::to_value(&request).expect("syntax-query request");
    let response = handle_syntax_query_operation_value(&payload).expect("native query response");
    let response: ProviderSyntaxQueryResponse =
        serde_json::from_slice(&response).expect("typed response");

    assert!(response.parsed);
    assert_eq!(response.captures.len(), 2);
    assert!(response.captures.iter().all(|capture| {
        capture.capture_name == "declaration.name"
            && capture.source_byte_start < capture.source_byte_end
            && capture.native_fact_ref.starts_with("rust:item:src/lib.rs:")
    }));
}

#[test]
fn predicate_plan_extracts_exact_function_name() {
    let predicate = r#"[{"capture":"function.name","op":"eq","values":[{"kind":"string","value":"parse_query"}]}]"#;
    assert_eq!(
        predicate_function_name(predicate).expect("parse predicate"),
        Some("parse_query".to_string())
    );
    assert_eq!(
        inline_eq_predicate(r#"(#eq? @function.name "parse_query")"#),
        Some("parse_query".to_string())
    );
}

#[test]
fn selector_must_remain_workspace_relative() {
    assert_eq!(
        exact_owner_path("rust://src/cli/query.rs#item/function/parse_query")
            .expect("canonical selector"),
        "src/cli/query.rs"
    );
    assert!(exact_owner_path("../outside.rs").is_err());
    assert!(exact_owner_path("/absolute.rs").is_err());
}
