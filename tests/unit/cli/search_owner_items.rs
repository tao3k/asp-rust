use super::support::{run_search, write_search_fixture};

#[test]
fn owner_items_inventory_omits_flow_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    write_search_fixture(root);

    let rendered = run_search(root, &["owner", "src/lib.rs", "items"]);

    assert!(
        rendered.starts_with("[search-owner] q=src/lib.rs"),
        "{rendered}"
    );
    assert!(rendered.contains(" item="), "{rendered}");
    assert!(rendered.contains("|item "), "{rendered}");
    assert!(!rendered.contains("|code "), "{rendered}");
    assert!(!rendered.contains("|test "), "{rendered}");
    assert!(!rendered.contains("|edge "), "{rendered}");
    assert!(!rendered.contains("|synthesis "), "{rendered}");
    assert!(!rendered.contains("|next "), "{rendered}");
}

#[test]
fn owner_item_query_seeds_render_code_frontier() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    write_search_fixture(root);

    let rendered = run_search(
        root,
        &[
            "owner",
            "src/lib.rs",
            "items",
            "--query",
            "load",
            "--view",
            "seeds",
        ],
    );

    assert!(rendered.starts_with("[search-owner]"), "{rendered}");
    assert!(
        rendered.contains("I=item:symbol(load)@rust://src/lib.rs#item/function/load!syntax"),
        "{rendered}"
    );
    assert!(
        rendered.contains("syntax I selector=rust://src/lib.rs#item/function/load"),
        "{rendered}"
    );
    assert!(rendered.contains("frontier=I.syntax"), "{rendered}");
    assert!(
        !rendered.contains("syntax I selector=src/lib.rs:"),
        "{rendered}"
    );
    assert!(!rendered.contains("fn load"), "{rendered}");
}

#[test]
fn owner_item_query_preserves_impl_and_trait_owner_scopes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    write_search_fixture(root);
    std::fs::write(
        root.join("src/lib.rs"),
        "trait Parse { fn parse(); }\n\
         struct A;\n\
         impl A { fn parse() {} }\n\
         struct B;\n\
         impl Parse for B { fn parse() {} }\n",
    )
    .expect("write scoped owner fixture");

    let rendered = run_search(
        root,
        &[
            "owner",
            "src/lib.rs",
            "items",
            "--query",
            "parse",
            "--view",
            "seeds",
        ],
    );

    for selector in [
        "rust://src/lib.rs#item/method/parse/scope/implementation-owner/type/A",
        "rust://src/lib.rs#item/method/parse/scope/implementation-owner/type/B/scope/trait-owner/trait/Parse",
        "rust://src/lib.rs#item/trait-function/parse/scope/trait-owner/trait/Parse",
    ] {
        assert!(
            rendered.contains(selector),
            "missing {selector}: {rendered}"
        );
    }
    assert!(
        !rendered.contains("rust://src/lib.rs#item/method/parse!syntax"),
        "{rendered}"
    );
}
