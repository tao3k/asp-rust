//! Fixed Rust test-layout contract without project-local policy exceptions.

use std::path::{Component, Path};

const ALLOWED_TEST_DIRS: &[&str] = &[
    "common",
    "fixtures",
    "integration",
    "performance",
    "scenarios",
    "snapshots",
    "support",
    "unit",
];

const ALLOWED_TEST_ROOT_FILES: &[&str] = &[
    "integration_test.rs",
    "lib.rs",
    "mod.rs",
    "performance_test.rs",
    "rust-project-harness-gate.rs",
    "scenarios_test.rs",
    "unit_test.rs",
    "xiuxian-testing-gate.rs",
];

pub(super) fn is_allowed_test_dir(name: &str) -> bool {
    ALLOWED_TEST_DIRS.contains(&name)
}

pub(super) fn is_allowed_test_root_file(name: &str) -> bool {
    ALLOWED_TEST_ROOT_FILES.contains(&name)
}

pub(super) fn is_allowed_test_suite_path(project_root: &Path, path: &Path) -> bool {
    if let Some(crate_local_path) = crate_local_test_suite_path(project_root, path) {
        return is_allowed_root_test_suite_path(crate_local_path);
    }
    is_allowed_root_test_suite_path(path)
}

fn crate_local_test_suite_path<'a>(project_root: &Path, path: &'a Path) -> Option<&'a Path> {
    let mut ancestors = Vec::new();
    let mut cursor = path.parent();
    while let Some(parent) = cursor {
        ancestors.push(parent);
        cursor = parent.parent();
    }
    ancestors
        .into_iter()
        .find(|parent| {
            parent != &Path::new("") && project_root.join(parent).join("Cargo.toml").is_file()
        })
        .and_then(|crate_root| path.strip_prefix(crate_root).ok())
        .filter(|relative| relative.starts_with("tests"))
}

fn is_allowed_root_test_suite_path(path: &Path) -> bool {
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return false;
    };
    if first != "tests" {
        return false;
    }
    let Some(Component::Normal(suite)) = components.next() else {
        return false;
    };
    suite.to_str().is_some_and(is_allowed_test_dir)
}
