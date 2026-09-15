//! Cargo lockfile dependency-baseline policy and evidence.

use std::path::{Path, PathBuf};

/// Cargo.lock dependency baseline shared by downstream build gates.
///
/// Use this when a downstream workspace must guarantee that a package resolves
/// to one exact version and one git source/rev across all member crates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AspRustDependencyBaseline {
    packages: Vec<AspRustDependencyBaselinePackage>,
}

impl AspRustDependencyBaseline {
    /// Create an empty dependency baseline.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Require one git package to resolve to an exact version and source
    /// fragment, such as `rev=<commit>`.
    #[must_use]
    pub fn require_git_package(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        source_contains: impl Into<String>,
    ) -> Self {
        self.packages.push(AspRustDependencyBaselinePackage {
            name: name.into(),
            version: version.into(),
            source_contains: source_contains.into(),
        });
        self
    }

    /// Required packages in insertion order.
    #[must_use]
    pub fn packages(&self) -> &[AspRustDependencyBaselinePackage] {
        &self.packages
    }
}

/// One exact Cargo.lock package requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AspRustDependencyBaselinePackage {
    name: String,
    version: String,
    source_contains: String,
}

impl AspRustDependencyBaselinePackage {
    /// Package name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Exact package version expected in Cargo.lock.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Required source fragment expected in Cargo.lock.
    #[must_use]
    pub fn source_contains(&self) -> &str {
        &self.source_contains
    }
}

/// Assert an exact Cargo.lock dependency baseline from a downstream gate.
///
/// The lockfile is searched from `project_root` upward, so member crates in a
/// Cargo workspace can share the workspace root `Cargo.lock`.
///
/// # Panics
///
/// Panics when no `Cargo.lock` is found, the lockfile cannot be parsed, or any
/// required package resolves to a missing, duplicate, wrong-version, or
/// wrong-source entry.
#[track_caller]
pub fn assert_asp_rust_dependency_baseline(
    project_root: &Path,
    dependency_baseline: &AspRustDependencyBaseline,
    gate_label: &str,
) {
    if dependency_baseline.packages().is_empty() {
        return;
    }
    let lockfile_path = find_cargo_lock(project_root).unwrap_or_else(|| {
        panic!(
            "{gate_label} dependency baseline: Cargo.lock not found from {}\n{}",
            project_root.display(),
            dependency_baseline_agent_guidance(gate_label)
        )
    });
    println!("cargo:rerun-if-changed={}", lockfile_path.display());
    let lockfile = cargo_lock::Lockfile::load(&lockfile_path).unwrap_or_else(|error| {
        panic!(
            "{gate_label} dependency baseline: failed to parse {}: {error}\n{}",
            lockfile_path.display(),
            dependency_baseline_agent_guidance(gate_label)
        )
    });

    for required_package in dependency_baseline.packages() {
        assert_dependency_baseline_package(&lockfile, required_package, gate_label, &lockfile_path);
    }
}

fn assert_dependency_baseline_package(
    lockfile: &cargo_lock::Lockfile,
    required_package: &AspRustDependencyBaselinePackage,
    gate_label: &str,
    lockfile_path: &Path,
) {
    let package_matches = lockfile
        .packages
        .iter()
        .filter(|package| package.name.to_string() == required_package.name())
        .collect::<Vec<_>>();
    if package_matches.len() != 1 {
        panic!(
            "{gate_label} dependency baseline: {} requires exactly one Cargo.lock entry in {}; found {}\nexpected: {}\nactual:\n{}\n{}",
            required_package.name(),
            lockfile_path.display(),
            package_matches.len(),
            render_required_dependency_baseline_package(required_package),
            render_dependency_baseline_package_matches(&package_matches),
            dependency_baseline_agent_guidance(gate_label)
        );
    }

    let package = package_matches[0];
    let actual_version = package.version.to_string();
    let actual_source = package
        .source
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "<none>".to_string());
    if actual_version != required_package.version()
        || !actual_source.contains(required_package.source_contains())
    {
        panic!(
            "{gate_label} dependency baseline: {} resolved to an unexpected Cargo.lock entry in {}\nexpected: {}\nactual: {}\n{}",
            required_package.name(),
            lockfile_path.display(),
            render_required_dependency_baseline_package(required_package),
            render_dependency_baseline_package(package),
            dependency_baseline_agent_guidance(gate_label)
        );
    }
}

fn render_required_dependency_baseline_package(
    package: &AspRustDependencyBaselinePackage,
) -> String {
    format!(
        "{} {} source contains {}",
        package.name(),
        package.version(),
        package.source_contains()
    )
}

fn render_dependency_baseline_package_matches(packages: &[&cargo_lock::Package]) -> String {
    if packages.is_empty() {
        return "- <none>".to_string();
    }
    packages
        .iter()
        .map(|package| format!("- {}", render_dependency_baseline_package(package)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_dependency_baseline_package(package: &cargo_lock::Package) -> String {
    let source = package
        .source
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "<none>".to_string());
    format!("{} {} source {source}", package.name, package.version)
}

fn dependency_baseline_agent_guidance(gate_label: &str) -> String {
    format!(
        "\
[asp-rust-dependency-guidance]
gate: {gate_label}
trigger: Cargo.lock dependency baseline drift.
repair:
- update the workspace dependency declaration that still pins the old version or git rev.
- if a transitive crate pins the old rev, upgrade that crate first instead of overriding the lockfile by hand.
- keep the baseline in shared workspace policy and derive member gates from AspRustWorkspacePolicy.
- rerun cargo update for the affected package, then cargo tree -i <package> --workspace.
- rerun cargo test so build.rs verifies the repaired lockfile.
"
    )
}

fn find_cargo_lock(project_root: &Path) -> Option<PathBuf> {
    let mut current = Some(project_root);
    while let Some(root) = current {
        let candidate = root.join("Cargo.lock");
        if candidate.exists() {
            return Some(candidate);
        }
        current = root.parent();
    }
    None
}
