use tempfile::TempDir;

use crate::cli::support::{run_cli, write_clean_source, write_manifest};

#[test]
fn cli_agent_guide_advertises_query_reroute() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-guide");
    write_clean_source(root);
    let output = run_cli(["guide".as_ref(), root.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("[agent-guide] lang=rust provider=asp-rust protocol=agent-guide.v1"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|surface query purpose=exact-selector-projection output=pure-source|callable-skeleton"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|flow bootstrap start="search guide .""#),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#"|refer search-guide="search guide ." use=low-frequency-tool-map"#),
        "{stdout}"
    );
    assert!(
        stdout.contains("|avoid raw-read,manual-window-scan"),
        "{stdout}"
    );
}

#[test]
fn cli_query_help_advertises_dependency_search_surface() {
    let output = run_cli(["query", "--help"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains(
            "asp-rust search dependency <crate-or-package> [items docs-use tests] [--view seeds] [--workspace WORKSPACE]"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("asp-rust search guide [--workspace WORKSPACE]"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Dependency search is manifest-first"),
        "{stdout}"
    );
}

#[test]
fn cli_query_guide_prints_query_contract() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-guide-contract");
    write_clean_source(root);
    let output = run_cli(["query".as_ref(), "guide".as_ref(), root.as_os_str()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("[query-guide] lang=rust provider=asp-rust protocol=query-guide.v1"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|contract projectionRequired=true"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            r#"|mode source command="query --selector <exact-structural-selector> --projection source --workspace <WORKSPACE>" output=pure-source"#
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("|contract jsonOutput=explicit-only"),
        "{stdout}"
    );
}
