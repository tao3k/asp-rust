use std::fs;
use std::path::Path;

use asp_rust::{
    asp_rust_rules_markdown, render_asp_rust_rules_markdown, rust_agent_policy_rules,
    rust_modularity_rules, rust_project_policy_rules, write_asp_rust_rules_to_unit_tests,
};

fn asp_rust_rules_rule_ids() -> Vec<&'static str> {
    asp_rust_rules_markdown()
        .lines()
        .map(|line| {
            line.strip_prefix("- ")
                .and_then(|item| item.split_once(": "))
                .map(|(rule_id, _)| rule_id)
                .expect("ASP Rust rules lines must use '<rule-id>: <sentence>'")
        })
        .collect()
}

fn catalog_rule_ids() -> Vec<&'static str> {
    let mut rule_ids = Vec::new();
    rule_ids.extend(
        rust_agent_policy_rules()
            .into_iter()
            .map(|rule| rule.rule_id),
    );
    rule_ids.extend(rust_modularity_rules().into_iter().map(|rule| rule.rule_id));
    rule_ids.extend(
        rust_project_policy_rules()
            .into_iter()
            .map(|rule| rule.rule_id),
    );
    rule_ids
}

#[test]
fn asp_rust_rules_markdown_is_plain_rule_id_list() {
    let raw = asp_rust_rules_markdown();
    let mut count = 0;

    for (index, line) in raw.lines().enumerate() {
        let Some(item) = line.strip_prefix("- ") else {
            panic!("line {} must be a markdown list item", index + 1);
        };
        let Some((rule_id, sentence)) = item.split_once(": ") else {
            panic!("line {} must use '<rule-id>: <sentence>'", index + 1);
        };

        assert!(
            rule_id.starts_with("RUST-AGENT-") || rule_id.starts_with("RUST-MOD-R"),
            "unexpected policy id prefix: {rule_id}"
        );
        assert!(
            sentence.ends_with('.'),
            "ASP Rust rule sentence must end with a period: {rule_id}"
        );
        assert_eq!(
            sentence.matches(['.', '!', '?']).count(),
            1,
            "ASP Rust rule must use one sentence: {rule_id}"
        );
        count += 1;
    }

    assert_eq!(count, 74);
}

#[test]
fn asp_rust_rules_ids_match_rule_catalog() {
    let mut asp_rust_rule_ids = asp_rust_rules_rule_ids();
    asp_rust_rule_ids.sort_unstable();

    let mut catalog_rule_ids = catalog_rule_ids();
    catalog_rule_ids.sort_unstable();

    assert_eq!(asp_rust_rule_ids, catalog_rule_ids);
}

#[test]
fn generated_asp_rust_rules_matches_unit_fixture() {
    let unit_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/unit");
    let fixture = unit_dir.join("asp-rust-rules.generated.md");
    if std::env::var_os("UPDATE_ASP_RUST_RULES").is_some() {
        write_asp_rust_rules_to_unit_tests(&unit_dir).unwrap();
    }

    assert_eq!(
        fs::read_to_string(fixture).unwrap(),
        render_asp_rust_rules_markdown()
    );
}

#[test]
fn build_dependency_helper_writes_to_requested_unit_dir() {
    let unit_dir = std::env::temp_dir().join(format!("asp-rust-rules-{}", std::process::id()));
    let _ = fs::remove_dir_all(&unit_dir);

    let output = write_asp_rust_rules_to_unit_tests(&unit_dir).unwrap();

    assert_eq!(output, unit_dir.join("asp-rust-rules.generated.md"));
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        render_asp_rust_rules_markdown()
    );

    fs::remove_dir_all(unit_dir).unwrap();
}
