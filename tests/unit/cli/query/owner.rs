use std::ffi::OsStr;
use std::process::{Command, Output};

fn run_cli<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .args(args)
        .output()
        .expect("run rs-harness")
}

#[test]
fn cli_help_separates_search_discovery_from_exact_query() {
    let search = run_cli(["search", "--help"]);
    assert!(search.status.success(), "{search:?}");
    let stdout = String::from_utf8(search.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("items --query SYMBOL [PROJECT_ROOT]"),
        "{stdout}"
    );

    let query = run_cli(["query", "--help"]);
    assert!(query.status.success(), "{query:?}");
    let stdout = String::from_utf8(query.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("query --selector 'rust://OWNER#item/KIND/NAME'"),
        "{stdout}"
    );
    assert!(
        stdout.contains("query --treesitter-query QUERY"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Owner and symbol discovery is owned by `search owner`"),
        "{stdout}"
    );
    assert!(!stdout.contains("query <owner-path>"), "{stdout}");
}
