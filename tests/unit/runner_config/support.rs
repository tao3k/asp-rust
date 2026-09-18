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

pub(super) fn create_member_crate(root: &Path, name: &str) {
    fs::create_dir_all(root.join("src")).expect("create member src");
    write_manifest(root, name);
}

pub(super) fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}

pub(super) fn has_rule(report: &AspRustReport, rule_id: &str) -> bool {
    report
        .findings
        .iter()
        .any(|finding| finding.rule_id == rule_id)
}

pub(super) fn has_rule_for_path_suffix(
    report: &AspRustReport,
    rule_id: &str,
    suffix: &str,
) -> bool {
    report.findings.iter().any(|finding| {
        finding.rule_id == rule_id
            && finding
                .location
                .path
                .as_ref()
                .is_some_and(|path| path.ends_with(suffix))
    })
}

pub(super) fn has_module_path(report: &AspRustReport, suffix: &str) -> bool {
    report
        .modules
        .iter()
        .any(|module| module.path.ends_with(suffix))
}

pub(super) fn has_package_path(report: &AspRustReport, suffix: &str) -> bool {
    report.project_resolution.as_ref().is_some_and(|scope| {
        scope
            .package_paths
            .iter()
            .any(|path| path.ends_with(suffix))
    })
}
