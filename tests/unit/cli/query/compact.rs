use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::cli::support::{normalize_temp_root, run_cli};

#[test]
fn cli_query_parser_code_source_slice_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_compact_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "branch_and_write|match_and_loop".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    insta::assert_snapshot!(
        stdout.trim_end(),
        @r###"
pub fn branch_and_write(flag: bool, block: &mut String) -> Option<String> {
    if let Some(line) = flag.then_some("ok") {
        let _ = writeln!(block, "{line}");
        return Some(line.to_string());
    }

    None
}
pub fn match_and_loop(values: &[String]) -> usize {
    let mut count = 0;
    for value in values {
        match value.as_str() {
            "skip" => continue,
            _ => count += 1,
        }
    }
    count
}
"###
    );
}

#[test]
fn cli_query_parser_code_preserves_whitespace_sensitive_literals() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_literal_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "raw_indent|spaced_literal".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("r#\"\nalpha\n    beta\n\"#"), "{stdout}");
    assert!(stdout.contains("\"alpha    beta\""), "{stdout}");
    assert!(!stdout.contains("string[lines="), "{stdout}");
}

#[test]
fn cli_query_parser_json_exposes_literal_exact_selector() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_literal_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "raw_indent".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("query packet json");
    let item = &value["items"][0];
    assert_eq!(item["name"], "raw_indent", "{value}");
    assert_eq!(
        item["fields"]["structuralSelector"], "rust://src/lib.rs#item/function/raw_indent",
        "{value}"
    );
    assert!(item.get("code").is_none(), "{value}");
    assert_eq!(value["syntaxAnchor"]["nodeType"], "function_item");
}

#[test]
fn cli_query_parser_compact_line_protocol_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_compact_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "branch_and_write|match_and_loop".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    );
    insta::assert_snapshot!(
        stdout.trim_end(),
        @r###"
[search-owner] q=src/lib.rs pkg=. own=1 item=2 itemQuery=branch_and_write|match_and_loop
|owner src/lib.rs role=source source=parser-visible-module lines=21 imports=1
|query itemQuery=branch_and_write|match_and_loop status=hit match=exact item=2 reason=parser-item-exact next=query-code
|item branch_and_write kind=fn responsibilities=guard-branch,call-dispatch,early-return public=true next=syntax:branch_and_write read=src/lib.rs:3:10 structuralSelector=rust://src/lib.rs#item/function/branch_and_write canonicalItemSelector={"schemaId":"asp.canonical-item-selector.v1","schemaVersion":"v1","languageId":"rust","kind":"function","symbol":"branch_and_write","scopes":[],"structuralSelector":"rust://src/lib.rs#item/function/branch_and_write"} syn=function_item/name tsqRef=semantic-tree-sitter-query/rust-owner-items.v1
|item match_and_loop kind=fn responsibilities=state-mutation,bounded-loop,match-dispatch,match-arm,early-return public=true next=syntax:match_and_loop read=src/lib.rs:12:21 structuralSelector=rust://src/lib.rs#item/function/match_and_loop canonicalItemSelector={"schemaId":"asp.canonical-item-selector.v1","schemaVersion":"v1","languageId":"rust","kind":"function","symbol":"match_and_loop","scopes":[],"structuralSelector":"rust://src/lib.rs#item/function/match_and_loop"} syn=function_item/name tsqRef=semantic-tree-sitter-query/rust-owner-items.v1
"###
    );
}

#[test]
fn cli_query_parser_owner_packet_exposes_canonical_item_identity() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_compact_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "branch_and_write".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("query packet json");
    let item = &value["items"][0];
    assert_eq!(item["name"], "branch_and_write", "{value}");
    assert_eq!(item["kind"], "fn");
    assert_eq!(
        item["fields"]["structuralSelector"], "rust://src/lib.rs#item/function/branch_and_write",
        "{value}"
    );
    assert_eq!(
        item["fields"]["syntaxQueryRef"],
        "semantic-tree-sitter-query/rust-owner-items.v1"
    );
    assert_eq!(value["syntaxAnchor"]["nodeType"], "function_item");
}

#[test]
fn cli_query_parser_type_shape_includes_fields_and_impl_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_data_shape_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "UserSummary".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    insta::assert_snapshot!(
        stdout.trim_end(),
        @r###"
pub struct UserSummary {
    pub user_id: u64,
    pub name: String,
    pub active: bool,
}
impl UserSummary {
    pub fn label(&self) -> String {
        if self.active {
            format!("{}#{}", self.name, self.user_id)
        } else {
            "inactive".to_string()
        }
    }
}
"###
    );
}

#[test]
fn cli_query_parser_type_shape_json_links_struct_and_impl_selectors() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_data_shape_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "UserSummary".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("query packet json");
    let items = value["items"].as_array().expect("items");
    assert_eq!(items.len(), 3, "{value}");
    assert_eq!(items[0]["kind"], "struct");
    assert_eq!(items[1]["kind"], "impl");
    assert_eq!(items[2]["kind"], "method");
    for item in items {
        assert!(
            item["fields"]["structuralSelector"]
                .as_str()
                .is_some_and(|selector| selector.starts_with("rust://src/lib.rs#item/")),
            "{value}"
        );
    }
    assert_eq!(
        items[2]["fields"]["structuralSelector"],
        "rust://src/lib.rs#item/method/label/scope/implementation-owner/type/UserSummary"
    );
}

fn write_parser_compact_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"compact-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r#"use std::fmt::Write as _;

pub fn branch_and_write(flag: bool, block: &mut String) -> Option<String> {
    if let Some(line) = flag.then_some("ok") {
        let _ = writeln!(block, "{line}");
        return Some(line.to_string());
    }

    None
}

pub fn match_and_loop(values: &[String]) -> usize {
    let mut count = 0;
    for value in values {
        match value.as_str() {
            "skip" => continue,
            _ => count += 1,
        }
    }
    count
}
"#,
    )
    .expect("write source");
}

fn write_parser_literal_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"literal-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r###"pub fn raw_indent() -> &'static str {
    r#"
alpha
    beta
"#
}

pub fn spaced_literal() -> &'static str {
    "alpha    beta"
}
"###,
    )
    .expect("write source");
}

fn write_parser_data_shape_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"data-shape-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r#"pub struct UserSummary {
    pub user_id: u64,
    pub name: String,
    pub active: bool,
}

impl UserSummary {
    pub fn label(&self) -> String {
        if self.active {
            format!("{}#{}", self.name, self.user_id)
        } else {
            "inactive".to_string()
        }
    }
}
"#,
    )
    .expect("write source");
}
