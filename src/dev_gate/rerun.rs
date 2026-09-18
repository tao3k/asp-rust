use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::runner::AspRustAnalysis;

pub(super) fn emit_cargo_rerun_inputs(project_root: &Path, analysis: &AspRustAnalysis) {
    for path in rerun_inputs(project_root, analysis) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

pub(super) fn emit_cargo_rerun_paths(
    project_root: &Path,
    relative_paths: impl IntoIterator<Item = impl AsRef<Path>>,
) {
    println!("cargo:rerun-if-changed={}", project_root.display());
    for relative_path in relative_paths {
        println!(
            "cargo:rerun-if-changed={}",
            project_root.join(relative_path).display()
        );
    }
}

fn rerun_inputs(project_root: &Path, analysis: &AspRustAnalysis) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::from([
        project_root.join("Cargo.toml"),
        project_root.join("Cargo.lock"),
        project_root.join("build.rs"),
    ]);
    for path in analysis.monitored_paths() {
        paths.insert(path);
    }
    paths.retain(|path| path.exists());
    paths
}
