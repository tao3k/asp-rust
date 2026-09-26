//! Scenario benchmark fixture discovery.

use std::fs;
use std::path::Path;

use super::types::{
    RustScenarioBenchmarkError, RustScenarioBenchmarkManifestKind, RustScenarioBenchmarkRequirement,
};

/// Discover every ASP Rust scenario root that must carry `benchmark.toml`.
pub fn discover_required_rust_scenario_benchmarks(
    crate_root: impl AsRef<Path>,
) -> Result<Vec<RustScenarioBenchmarkRequirement>, RustScenarioBenchmarkError> {
    let crate_root = crate_root.as_ref();
    let mut requirements = Vec::new();
    collect_scenario_toml_requirements(
        &crate_root.join("tests").join("unit").join("scenarios"),
        &mut requirements,
    )?;
    collect_ast_patch_scenario_requirements(
        &crate_root
            .join("tests")
            .join("fixtures")
            .join("ast_patch_scenarios"),
        &mut requirements,
    )?;
    requirements.sort_by(|left, right| left.root.cmp(&right.root));
    Ok(requirements)
}

fn collect_scenario_toml_requirements(
    root: &Path,
    requirements: &mut Vec<RustScenarioBenchmarkRequirement>,
) -> Result<(), RustScenarioBenchmarkError> {
    if !root.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(root)
        .map_err(|error| RustScenarioBenchmarkError::new(root, error.to_string()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| RustScenarioBenchmarkError::new(root, error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    if root.join("scenario.toml").exists() {
        requirements.push(RustScenarioBenchmarkRequirement {
            root: root.to_path_buf(),
            manifest_kind: RustScenarioBenchmarkManifestKind::ScenarioToml,
        });
        return Ok(());
    }
    for entry in entries {
        if entry.is_dir() {
            collect_scenario_toml_requirements(&entry, requirements)?;
        }
    }
    Ok(())
}

fn collect_ast_patch_scenario_requirements(
    root: &Path,
    requirements: &mut Vec<RustScenarioBenchmarkRequirement>,
) -> Result<(), RustScenarioBenchmarkError> {
    if !root.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(root)
        .map_err(|error| RustScenarioBenchmarkError::new(root, error.to_string()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| RustScenarioBenchmarkError::new(root, error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for entry in entries {
        if entry.is_dir() && entry.join("scenario.json").exists() {
            requirements.push(RustScenarioBenchmarkRequirement {
                root: entry,
                manifest_kind: RustScenarioBenchmarkManifestKind::AstPatchScenarioJson,
            });
        }
    }
    Ok(())
}
