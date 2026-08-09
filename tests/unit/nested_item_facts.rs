use rust_lang_project_harness::nested_item_facts::{
    RustItemScopeFactV1, project_rust_nested_item_code_v1, rust_nested_item_facts_v1,
};

#[test]
fn nested_function_and_struct_have_independent_scoped_facts() {
    let source = r#"
fn outer() {
    struct Local;
    fn inner() -> usize {
        1
    }
    let _ = inner();
}
"#;
    let facts = rust_nested_item_facts_v1("src/lib.rs", source).expect("facts");
    let outer = facts
        .iter()
        .find(|fact| fact.symbol.as_str() == "outer")
        .expect("outer");
    let local = facts
        .iter()
        .find(|fact| fact.symbol.as_str() == "Local")
        .expect("local");
    let inner = facts
        .iter()
        .find(|fact| fact.symbol.as_str() == "inner")
        .expect("inner");
    assert!(outer.scopes.is_empty());
    assert_eq!(
        local.scopes,
        vec![RustItemScopeFactV1 {
            kind: "function".into(),
            symbol: "outer".into(),
        }]
    );
    assert_eq!(inner.scopes, local.scopes);
    assert_eq!(
        project_rust_nested_item_code_v1(source, inner).expect("inner projection"),
        "fn inner() -> usize {\n        1\n    }"
    );
}

#[test]
fn repeated_nested_symbols_remain_distinct_by_scope() {
    let source = r#"
fn left() {
    fn duplicate() {}
}
fn right() {
    fn duplicate() {}
}
"#;
    let facts = rust_nested_item_facts_v1("src/lib.rs", source).expect("facts");
    let duplicates = facts
        .iter()
        .filter(|fact| fact.symbol.as_str() == "duplicate")
        .collect::<Vec<_>>();
    assert_eq!(duplicates.len(), 2);
    assert_ne!(duplicates[0].scopes, duplicates[1].scopes);
    assert_ne!(
        duplicates[0].source_byte_start,
        duplicates[1].source_byte_start
    );
    assert_ne!(duplicates[0].identity_digest, duplicates[1].identity_digest);
}

#[test]
fn impl_and_trait_owners_are_parser_facts() {
    let source = r#"
trait Run {
    fn run(&self);
}
struct Worker;
impl Run for Worker {
    fn run(&self) {}
}
"#;
    let facts = rust_nested_item_facts_v1("src/lib.rs", source).expect("facts");
    let implementation = facts
        .iter()
        .find(|fact| fact.item_kind.as_str() == "impl")
        .expect("impl");
    let method = facts
        .iter()
        .find(|fact| fact.item_kind.as_str() == "method" && fact.impl_owner.is_some())
        .expect("impl method");
    assert_eq!(
        implementation
            .impl_owner
            .as_ref()
            .map(|owner| owner.as_str()),
        Some("Worker")
    );
    assert_eq!(
        implementation
            .trait_owner
            .as_ref()
            .map(|owner| owner.as_str()),
        Some("Run")
    );
    assert_eq!(
        method.impl_owner.as_ref().map(|owner| owner.as_str()),
        Some("Worker")
    );
    assert_eq!(
        method.trait_owner.as_ref().map(|owner| owner.as_str()),
        Some("Run")
    );
}
