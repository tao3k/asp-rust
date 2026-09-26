use std::fs;
use std::path::Path;

use asp_rust::AspRustReport;

pub(super) fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

pub(super) fn private_implementation_pile() -> String {
    let mut source = String::from("//! Private implementation pile.\n");
    for index in 0..45 {
        source.push_str(&format!(
            "fn helper_{index}() -> usize {{\n  let mut total = {index};\n  total += 1;\n  total += 2;\n  total += 3;\n  total += 4;\n  total += 5;\n  total += 6;\n  total += 7;\n  total += 8;\n  total += 9;\n  total += 10;\n  total += 11;\n  total += 12;\n  total += 13;\n  total\n}}\n"
        ));
    }
    source
}

pub(super) fn findings_for_rule<'a>(
    report: &'a AspRustReport,
    rule_id: &str,
) -> Vec<&'a asp_rust::AspRustFinding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == rule_id)
        .collect()
}

pub(super) fn has_rule(report: &AspRustReport, rule_id: &str) -> bool {
    report
        .findings
        .iter()
        .any(|finding| finding.rule_id == rule_id)
}
