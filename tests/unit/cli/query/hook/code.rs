use tempfile::TempDir;

use crate::cli::support::{run_cli, write_search_fixture, write_source_snapshot_envelope};

#[test]
fn cli_query_code_output_is_source_slice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    let source = std::fs::read_to_string(root.join("src/lib.rs")).expect("read fixture source");
    let envelope =
        write_source_snapshot_envelope(root, "rs-harness-test", &[("src/lib.rs", &source)]);
    let output = run_cli([
        "query".as_ref(),
        "--from-hook".as_ref(),
        "direct-source-read".as_ref(),
        "--selector".as_ref(),
        "rust://src/lib.rs#item/function/load".as_ref(),
        "--source-snapshot-envelope".as_ref(),
        envelope.as_os_str(),
        "--code".as_ref(),
        "--workspace".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "{stdout}");
    assert!(!stdout.contains("|code"), "{stdout}");
    assert!(!stdout.contains("text="), "{stdout}");
    assert!(stdout.contains("fn load"), "{stdout}");
    assert!(stdout.contains("domain::make_thing()"), "{stdout}");
}
