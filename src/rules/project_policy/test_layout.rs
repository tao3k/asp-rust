//! Test directory layout policy.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::parser::file_location;
use crate::{AspRustFinding, AspRustRule};

use super::config::{is_allowed_test_dir, is_allowed_test_root_file};
use super::support::is_rust_file;
use super::{RUST_PROJ_R001, RUST_PROJ_R002};

pub(super) fn test_layout_findings(
    project_root: &Path,
    rules: &BTreeMap<&'static str, AspRustRule>,
) -> Vec<AspRustFinding> {
    let tests_dir = project_root.join("tests");
    let Ok(entries) = fs::read_dir(&tests_dir) else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !is_allowed_test_dir(&name) {
                let rule = &rules[RUST_PROJ_R002];
                findings.push(AspRustFinding::from_rule(
                    rule,
                    format!("tests/{name} is not a standard ASP Rust suite directory."),
                    file_location(path),
                    None,
                    "move or explicitly justify this tests directory",
                ));
            }
            continue;
        }
        if !is_rust_file(&path) || is_allowed_test_root_file(&name) {
            continue;
        }
        let rule = &rules[RUST_PROJ_R001];
        findings.push(AspRustFinding::from_rule(
            rule,
            format!("tests/{name} is a root-level test file without an explicit harness role."),
            file_location(path),
            None,
            "move this file under tests/unit or tests/integration",
        ));
    }
    findings
}
