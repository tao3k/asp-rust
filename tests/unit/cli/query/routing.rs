use tempfile::TempDir;

use crate::cli::support::{
    run_cli, write_clean_source, write_complex_dependency_fixture, write_manifest,
};

#[test]
fn cli_query_terms_require_parser_owned_owner_search() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_complex_dependency_fixture(root);
    let output = run_cli([
        "query".as_ref(),
        "--term".as_ref(),
        "RuntimeClient".as_ref(),
        "--term".as_ref(),
        "send_bytes".as_ref(),
        "--surface".as_ref(),
        "tests".as_ref(),
        "--view".as_ref(),
        "seeds".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("rust query requires an exact --selector"),
        "{stderr}"
    );
    assert!(stderr.contains("asp rust search owner"), "{stderr}");
}

#[test]
fn cli_query_broad_glob_selector_is_rejected() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "cli-query-glob");
    write_clean_source(root);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "bulk-source-dump".as_ref(),
        "--selector".as_ref(),
        "**/*.rs".as_ref(),
        root.as_os_str(),
    ]);
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("rust query requires an exact --selector"),
        "{stderr}"
    );
    assert!(stderr.contains("asp rust search owner"), "{stderr}");
}
