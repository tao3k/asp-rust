//! Rust source path facts shared by path-oriented policies.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustSourcePathFacts {
    pub(crate) namespace_components: Vec<String>,
    pub(crate) repeated_namespace_segments: BTreeSet<String>,
    pub(crate) repeated_namespace_branch: Option<PathBuf>,
    pub(crate) is_special_entrypoint: bool,
    pub(crate) is_test_source: bool,
    pub(crate) is_test_support_module: bool,
    pub(crate) is_crate_facade: bool,
    pub(crate) is_interface_mod: bool,
    pub(crate) is_binary_entrypoint: bool,
    pub(crate) is_package_entrypoint: bool,
    pub(crate) is_build_script_entrypoint: bool,
}

pub(crate) fn rust_source_path_facts(
    project_root: &Path,
    source_paths: &[PathBuf],
    test_paths: &[PathBuf],
    package_paths: &[PathBuf],
    path: &Path,
) -> RustSourcePathFacts {
    let namespace_components = namespace_components(project_root, path).unwrap_or_else(|| {
        let fallback = match path.file_stem().and_then(|stem| stem.to_str()) {
            Some(stem) => stem.to_owned(),
            None => "unknown-source".to_owned(),
        };
        vec![fallback]
    });
    let repeated_namespace_segments = repeated_segments(&namespace_components);
    let repeated_namespace_branch = (!repeated_namespace_segments.is_empty())
        .then(|| offending_branch(&namespace_components, &repeated_namespace_segments));
    let is_package_entrypoint = package_paths.iter().any(|entrypoint| entrypoint == path);
    let is_test_source = is_under_any_dir(path, test_paths);
    RustSourcePathFacts {
        namespace_components,
        repeated_namespace_segments,
        repeated_namespace_branch,
        is_special_entrypoint: file_name_matches(path, &["lib.rs", "main.rs", "mod.rs"]),
        is_test_source,
        is_test_support_module: is_test_source && file_stem_is(path, "support"),
        is_crate_facade: file_name_is(path, "lib.rs"),
        is_interface_mod: file_name_is(path, "mod.rs"),
        is_binary_entrypoint: is_binary_entrypoint(source_paths, path),
        is_package_entrypoint,
        is_build_script_entrypoint: is_package_entrypoint && file_name_is(path, "build.rs"),
    }
}

fn is_under_any_dir(path: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| path.starts_with(dir))
}

fn namespace_components(project_root: &Path, path: &Path) -> Option<Vec<String>> {
    let relative = path.strip_prefix(project_root).ok()?;
    let parent = relative.parent()?;
    let mut components = parent
        .iter()
        .map(|component| component.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let file_stem = relative.file_stem()?.to_string_lossy();
    if !matches!(file_stem.as_ref(), "lib" | "main" | "mod") {
        components.push(file_stem.to_string());
    }
    (!components.is_empty()).then_some(components)
}

fn repeated_segments(components: &[String]) -> BTreeSet<String> {
    let mut counts = BTreeMap::new();
    for component in components {
        *counts.entry(component.clone()).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(component, count)| (count > 1).then_some(component))
        .collect()
}

fn offending_branch(components: &[String], repeated: &BTreeSet<String>) -> PathBuf {
    let deepest_index = components
        .iter()
        .enumerate()
        .filter_map(|(index, component)| repeated.contains(component).then_some(index))
        .max()
        .unwrap_or(components.len().saturating_sub(1));
    components
        .iter()
        .take(deepest_index + 1)
        .collect::<PathBuf>()
}

fn is_binary_entrypoint(source_paths: &[PathBuf], path: &Path) -> bool {
    source_paths.iter().any(|source_root| {
        if path == source_root.join("main.rs") {
            return true;
        }
        let Ok(relative) = path.strip_prefix(source_root) else {
            return false;
        };
        let components = relative
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>();
        matches!(
            components.as_slice(),
            [first, _] if first.as_ref() == "bin"
        ) || matches!(
            components.as_slice(),
            [first, _, file] if first.as_ref() == "bin" && file.as_ref() == "main.rs"
        )
    })
}

fn file_name_is(path: &Path, name: &str) -> bool {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| file_name == name)
}

fn file_stem_is(path: &Path, stem: &str) -> bool {
    path.file_stem()
        .and_then(|file_stem| file_stem.to_str())
        .is_some_and(|file_stem| file_stem == stem)
}

fn file_name_matches(path: &Path, names: &[&str]) -> bool {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| names.contains(&file_name))
}
