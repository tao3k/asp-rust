use tempfile::TempDir;

use crate::cli::support::{run_search, write_search_fixture};

#[test]
fn cli_search_owner_items_graph_prioritizes_symbol_code_frontier() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let output = run_search(
        root,
        &[
            "owner",
            "src/domain/mod.rs",
            "items",
            "--query",
            "Thing",
            "--view",
            "seeds",
        ],
    );

    assert!(
        output.contains("O=owner:path(src/domain/mod.rs)!owner"),
        "{output}"
    );
    assert!(
        output.contains("aliases: graph:{G=search,O=owner,Q=query,I=item}"),
        "{output}"
    );
    assert!(
        output.contains("I=item:symbol(Thing)@rust://src/domain/mod.rs#item/struct/Thing!syntax"),
        "{output}"
    );
    assert!(
        output.contains("syntax I selector=rust://src/domain/mod.rs#item/struct/Thing"),
        "{output}"
    );
    assert!(output.contains("rank=I,O frontier=I.syntax"), "{output}");
    assert!(
        !output.contains("syntax I selector=src/domain/mod.rs:"),
        "{output}"
    );
    assert!(!output.contains("S=symbol"), "{output}");
}
