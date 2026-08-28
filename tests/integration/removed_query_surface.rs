use std::process::Command;

fn removed_source_switch() -> String {
    ["--", "code"].concat()
}

fn removed_names_switch() -> String {
    ["--", "names", "-only"].concat()
}

#[test]
fn removed_query_projection_flags_are_unknown_arguments_via_cli() {
    let binary = env!("CARGO_BIN_EXE_asp-rust");
    for (removed, unrelated) in [
        (removed_source_switch(), removed_names_switch()),
        (removed_names_switch(), removed_source_switch()),
    ] {
        let output = Command::new(binary)
            .args([
                "query",
                "--selector",
                "rust://src/lib.rs#item/function/definitely_missing",
                "--projection",
                "source",
                removed.as_str(),
            ])
            .output()
            .expect("run rejected query switch");
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 query error");
        assert!(stderr.contains("unexpected argument"));
        assert!(stderr.contains(&removed));
        assert!(!stderr.contains(&unrelated));
        assert!(!stderr.contains("legacy"));
    }
}

#[test]
fn query_help_exposes_only_projection_rendering() {
    let output = Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .args(["query", "--help"])
        .output()
        .expect("run query help");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 query help");
    assert!(stdout.contains("--projection source|callable-skeleton"));
    assert!(!stdout.contains(&removed_source_switch()));
    assert!(!stdout.contains(&removed_names_switch()));
}
#[test]
fn query_projection_is_accepted_by_the_real_cli_parser() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .args([
            "query",
            "--selector",
            "rust://src/lib.rs#item/function/definitely_missing",
            "--workspace",
            ".",
            "--projection",
            "callable-skeleton",
        ])
        .output()
        .expect("run rs-harness query");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument '--projection'"),
        "projection must be parsed by the real CLI, stderr={stderr}"
    );
}
