use std::collections::BTreeSet;
use std::path::Path;

const CORPUS: &str = r#"
pub fn run() {}
pub struct Data;
pub enum Choice { A }
pub trait Work {}
impl Work for Data {}
mod inner {}
use std::fmt;
const LIMIT: u8 = 1;
static FLAG: bool = true;
type Alias = u8;
macro_rules! sample { () => {} }
"#;

const RUNTIME_ADMITTED_ROWS: &[(&str, &str)] = &[
    ("const_item", "LIMIT"),
    ("enum_item", "Choice"),
    ("function_item", "run"),
    ("impl_item", ""),
    ("mod_item", "inner"),
    ("static_item", "FLAG"),
    ("struct_item", "Data"),
    ("trait_item", "Work"),
    ("type_item", "Alias"),
    ("use_declaration", ""),
];

#[test]
fn native_parser_matches_the_runtime_admitted_top_level_receipt() {
    let native = crate::parser::parse_rust_source(Path::new("src/lib.rs"), CORPUS.to_owned());
    assert!(native.report.is_valid);
    let native_rows = native
        .syntax_facts
        .top_level_items
        .iter()
        .filter_map(|item| {
            native_kind_to_tree_sitter(item.kind)
                .map(|kind| (kind.to_owned(), item.name.clone().unwrap_or_default()))
        })
        .collect::<BTreeSet<_>>();

    // ASP owns the Tree-sitter runtime and independently regenerates this
    // admitted-row receipt in the workspace contract gate.  The provider owns
    // only its native parser half of the equivalence proof.
    let runtime_admitted_rows = RUNTIME_ADMITTED_ROWS
        .iter()
        .map(|(kind, name)| ((*kind).to_owned(), (*name).to_owned()))
        .collect::<BTreeSet<_>>();

    assert_eq!(native_rows, runtime_admitted_rows);

    let parser_abi_files = [
        include_bytes!("../../src/parser/parsed_module.rs").as_slice(),
        include_bytes!("../../src/parser/native_syntax/collect.rs").as_slice(),
        include_bytes!("../../src/parser/native_syntax/item_facts.rs").as_slice(),
        include_bytes!("../../src/parser/native_syntax/facts.rs").as_slice(),
    ];
    let parser_abi_digest = framed_text_digest("asp-rust-syn-native-items-v1", &parser_abi_files);
    let query_grammar_digest = framed_text_digest(
        "asp-rust-tree-sitter-query-grammar-v1",
        &[include_bytes!("../../tree-sitter/tree-sitter-rust/grammar-profile.json").as_slice()],
    );
    let corpus_digest = framed_digest("asp-rust-enhanced-query-corpus-v1", &[CORPUS.as_bytes()]);
    let mut receipt_hasher = blake3::Hasher::new();
    digest_field(
        &mut receipt_hasher,
        b"asp-rust-enhanced-query-equivalence-receipt-v1",
    );
    digest_field(&mut receipt_hasher, CORPUS.as_bytes());
    for (kind, name) in &native_rows {
        digest_field(&mut receipt_hasher, kind.as_bytes());
        digest_field(&mut receipt_hasher, name.as_bytes());
    }
    let receipt_digest = format!("blake3-256:{}", receipt_hasher.finalize().to_hex());
    assert_eq!(
        parser_abi_digest,
        "blake3-256:122dae5e98853c9a8a168a7ee9b558d64a200bf4c97516f51f7171293883a4ca"
    );
    assert_eq!(
        query_grammar_digest,
        "blake3-256:a5f7d55c6c3dbc57f442790e1d55868f6454f7d92393359fa61807b99b60ac98"
    );
    assert_eq!(
        corpus_digest,
        "blake3-256:d44f6ab7b8426640d3e400910c9745d5fdfc3c20f5bb3219560b5a5bc5de1973"
    );
    assert_eq!(
        receipt_digest,
        "blake3-256:2cd7fd4a2451386e2d6c2522cddded95ab475ab836ec15992d3e13f60a4e0bac"
    );
}

#[test]
fn capability_table_is_bound_to_the_executable_differential_receipt() {
    let mut table: serde_json::Value = serde_json::from_str(include_str!(
        "../../tree-sitter/tree-sitter-rust/enhanced-query-capabilities.v1.json"
    ))
    .expect("enhanced-query capability table JSON");
    assert_eq!(table["schemaVersion"], "1");
    assert_eq!(table["languageId"], "rust");
    assert_eq!(table["providerId"], "asp-rust");
    assert_eq!(
        table["operatorTableDigest"],
        "blake3-256:e9f88db6f6cab915cad26739fd9c9da58e1dbd8e73b7e314adb08ff6fca45d2a"
    );
    assert_eq!(
        table["tableDigest"],
        "blake3-256:49b5721c82bc2d9b15688e28442e78b19479b5d968cc56c0c810917ee1220e9f"
    );

    let rows = table["rows"].as_array().expect("capability rows");
    assert!(rows.iter().any(|row| {
        row["rowId"] == "rust.node.macro-definition"
            && row["publicationState"] == "provider-local"
            && row.get("equivalenceEvidence").is_none()
    }));
    for row in rows
        .iter()
        .filter(|row| row["publicationState"] == "runtime")
    {
        let evidence = &row["equivalenceEvidence"];
        assert_eq!(
            evidence["corpusDigest"],
            "blake3-256:d44f6ab7b8426640d3e400910c9745d5fdfc3c20f5bb3219560b5a5bc5de1973"
        );
        assert_eq!(
            evidence["receiptDigest"],
            "blake3-256:2cd7fd4a2451386e2d6c2522cddded95ab475ab836ec15992d3e13f60a4e0bac"
        );
    }

    let expected_table_digest = table["tableDigest"]
        .as_str()
        .expect("tableDigest")
        .to_owned();
    table
        .as_object_mut()
        .expect("capability table object")
        .remove("tableDigest");
    let canonical = serde_json::to_vec(&table).expect("canonical capability table JSON");
    let actual_table_digest = format!("blake3-256:{}", blake3::hash(&canonical).to_hex());
    assert_eq!(expected_table_digest, actual_table_digest);
}

fn framed_digest(domain: &str, fields: &[&[u8]]) -> String {
    let mut hasher = blake3::Hasher::new();
    digest_field(&mut hasher, domain.as_bytes());
    for field in fields {
        digest_field(&mut hasher, field);
    }
    format!("blake3-256:{}", hasher.finalize().to_hex())
}

fn framed_text_digest(domain: &str, fields: &[&[u8]]) -> String {
    let canonical = fields
        .iter()
        .map(|field| canonical_text(field))
        .collect::<Vec<_>>();
    let canonical = canonical.iter().map(Vec::as_slice).collect::<Vec<_>>();
    framed_digest(domain, &canonical)
}

fn canonical_text(value: &[u8]) -> Vec<u8> {
    let mut canonical = Vec::with_capacity(value.len());
    let mut cursor = 0;
    while cursor < value.len() {
        if value[cursor] == b'\r' {
            canonical.push(b'\n');
            cursor += usize::from(value.get(cursor + 1) == Some(&b'\n')) + 1;
        } else {
            canonical.push(value[cursor]);
            cursor += 1;
        }
    }
    canonical
}

fn digest_field(hasher: &mut blake3::Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
}

fn native_kind_to_tree_sitter(kind: &str) -> Option<&'static str> {
    match kind {
        "fn" | "function" => Some("function_item"),
        "struct" => Some("struct_item"),
        "enum" => Some("enum_item"),
        "trait" => Some("trait_item"),
        "impl" => Some("impl_item"),
        "mod" => Some("mod_item"),
        "use" => Some("use_declaration"),
        "const" => Some("const_item"),
        "static" => Some("static_item"),
        "type" => Some("type_item"),
        _ => None,
    }
}
