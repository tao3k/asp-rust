use std::fs;

use tempfile::TempDir;

use super::support::{run_cli, write_manifest};

#[test]
fn cli_check_changed_renders_failure_frontier() {
    let temp = TempDir::new().expect("temp dir");
    write_frontier_fixture(temp.path());

    let output = run_cli([
        "check".as_ref(),
        "--changed".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(!output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("[fail] rust blockingFindings=3 advisoryFindings=2"),
        "{stdout}"
    );
    assert!(
        stdout.contains("|failureFrontier status=ready source=rust-check hotBlocks=1"),
        "{stdout}"
    );
    assert!(stdout.contains("directSourceReadCode<=1"), "{stdout}");
    assert!(
        stdout.contains(
            "|hotBlock selector=src/lib.rs:1-14 source=finding rule=RUST-AGENT-PROJECT-003 line=2"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "|next asp rust query --from-hook direct-source-read --code --workspace . --selector 'src/lib.rs:1-14'"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("--code ."), "{stdout}");
    assert!(!stdout.contains("[RUST-AGENT-PROJECT-003]"), "{stdout}");
}

#[test]
fn cli_check_changed_json_keeps_structured_output_only() {
    let temp = TempDir::new().expect("temp dir");
    write_frontier_fixture(temp.path());

    let output = run_cli([
        "check".as_ref(),
        "--changed".as_ref(),
        "--json".as_ref(),
        temp.path().as_os_str(),
    ]);

    assert!(!output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value = serde_json::from_str::<serde_json::Value>(&stdout).expect("json output");
    assert!(!value["findings"].as_array().expect("findings").is_empty());
    assert!(!stdout.contains("|failureFrontier"), "{stdout}");
}

fn write_frontier_fixture(root: &std::path::Path) {
    write_manifest(root, "cli-check-frontier");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn value() -> usize { 1 }\n#[cfg(test)] mod tests { #[test] fn it_works() {} }\n",
    )
    .expect("write lib");
}
