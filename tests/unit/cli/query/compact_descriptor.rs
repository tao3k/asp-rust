use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::cli::support::run_cli;

#[test]
fn cli_query_parser_code_preserves_descriptor_collections() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_parser_descriptor_fixture(root);

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "rust_view_descriptors".as_ref(),
        "--code".as_ref(),
        root.as_os_str(),
    ]);

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("vec!["), "{stdout}");
    assert!(stdout.contains("ViewDescriptor {"), "{stdout}");
    assert!(stdout.contains("requires_query: false"), "{stdout}");
    assert!(
        !stdout.contains("vec[4] items=ViewDescriptor name=workspace"),
        "{stdout}"
    );

    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        "rust_view_descriptors".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("query packet json");
    assert_eq!(value["items"][0]["name"], "rust_view_descriptors");
    assert_eq!(
        value["items"][0]["fields"]["structuralSelector"],
        "rust://src/lib.rs#item/function/rust_view_descriptors"
    );
}

fn write_parser_descriptor_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"descriptor-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        r#"pub struct ViewDescriptor {
    pub name: &'static str,
    pub capabilities: &'static [&'static str],
    pub requires_query: bool,
    pub accepted_pipes: &'static [&'static str],
}

pub fn rust_view_descriptors() -> Vec<ViewDescriptor> {
    vec![
        ViewDescriptor {
            name: "workspace",
            capabilities: &["workspace-router"],
            requires_query: false,
            accepted_pipes: &[],
        },
        ViewDescriptor {
            name: "dependencies",
            capabilities: &["dependency-api"],
            requires_query: true,
            accepted_pipes: &[],
        },
        ViewDescriptor {
            name: "tests",
            capabilities: &["coverage"],
            requires_query: false,
            accepted_pipes: &[],
        },
        ViewDescriptor {
            name: "policy",
            capabilities: &["policy"],
            requires_query: true,
            accepted_pipes: &["rg"],
        },
    ]
}
"#,
    )
    .expect("write source");
}
