//! Rust module tree facts derived from native module declarations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::ParsedRustModule;
use super::path_resolution::resolve_rust_include_literal;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustModuleTreeFacts {
    pub(crate) shadowed_module_sources: Vec<RustModuleSourceShadow>,
    pub(crate) unreachable_source_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustModuleSourceShadow {
    pub(crate) file_form: PathBuf,
    pub(crate) mod_form: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustModuleChildEdge {
    pub(crate) child_path: PathBuf,
    pub(crate) kind: RustModuleChildEdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RustModuleChildEdgeKind {
    Mod,
    PathAttrMod,
    IncludeLiteral,
}

impl RustModuleChildEdgeKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::PathAttrMod => "path-mod",
            Self::IncludeLiteral => "include",
        }
    }
}

pub(crate) fn rust_module_tree_facts(
    source_paths: &[PathBuf],
    modules: &[ParsedRustModule],
) -> RustModuleTreeFacts {
    let source_files = modules
        .iter()
        .filter(|module| is_under_any_dir(&module.report.path, source_paths))
        .map(|module| module.report.path.clone())
        .collect::<BTreeSet<_>>();
    if source_files.is_empty() {
        return RustModuleTreeFacts::default();
    }
    let reachable_source_files = reachable_source_files(source_paths, modules, &source_files);
    RustModuleTreeFacts {
        shadowed_module_sources: shadowed_module_sources(&source_files),
        unreachable_source_files: source_files
            .difference(&reachable_source_files)
            .filter(|path| !is_module_tree_root(source_paths, path))
            .cloned()
            .collect(),
    }
}

fn is_special_rust_entrypoint_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "lib.rs" | "main.rs" | "mod.rs"))
}

fn shadowed_module_sources(source_files: &BTreeSet<PathBuf>) -> Vec<RustModuleSourceShadow> {
    let mut shadows = Vec::new();
    for mod_form in source_files
        .iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some("mod.rs"))
    {
        let Some(parent) = mod_form.parent() else {
            continue;
        };
        let Some(module_name) = parent.file_name() else {
            continue;
        };
        let Some(grandparent) = parent.parent() else {
            continue;
        };
        let file_form = grandparent.join(format!("{}.rs", module_name.to_string_lossy()));
        if source_files.contains(&file_form) {
            shadows.push(RustModuleSourceShadow {
                file_form,
                mod_form: mod_form.clone(),
            });
        }
    }
    shadows
}

fn reachable_source_files(
    source_paths: &[PathBuf],
    modules: &[ParsedRustModule],
    source_files: &BTreeSet<PathBuf>,
) -> BTreeSet<PathBuf> {
    let modules_by_path = modules
        .iter()
        .map(|module| (module.report.path.clone(), module))
        .collect::<BTreeMap<_, _>>();
    let mut reachable = BTreeSet::new();
    let mut stack = source_files
        .iter()
        .filter(|path| is_module_tree_root(source_paths, path))
        .cloned()
        .collect::<Vec<_>>();
    while let Some(path) = stack.pop() {
        if !reachable.insert(path.clone()) {
            continue;
        }
        let Some(module) = modules_by_path.get(&path) else {
            continue;
        };
        for edge in external_child_module_edges(module, source_files) {
            let child_path = edge.child_path;
            if !reachable.contains(&child_path) {
                stack.push(child_path);
            }
        }
    }
    reachable
}

pub(in crate::parser) fn external_child_module_edges(
    module: &ParsedRustModule,
    source_files: &BTreeSet<PathBuf>,
) -> Vec<RustModuleChildEdge> {
    external_child_module_edges_with_test_cfg(module, source_files, false)
}

pub(in crate::parser) fn external_child_module_edges_with_test_cfg(
    module: &ParsedRustModule,
    source_files: &BTreeSet<PathBuf>,
    include_cfg_test: bool,
) -> Vec<RustModuleChildEdge> {
    let module_path = &module.report.path;
    let mut edges = Vec::new();
    for item in &module.syntax_facts.top_level_items {
        if let Some(include_target) = &item.include_target {
            let include_path = resolve_rust_include_literal(module_path, include_target);
            if source_files.contains(&include_path) {
                edges.push(RustModuleChildEdge {
                    child_path: include_path,
                    kind: RustModuleChildEdgeKind::IncludeLiteral,
                });
            }
        }
        let Some(item_mod) = &item.module else {
            continue;
        };
        if item_mod.is_inline || (item_mod.is_cfg_test && !include_cfg_test) {
            continue;
        }
        if let Some(resolved) = &item_mod.resolved_path_attr {
            if source_files.contains(resolved) {
                edges.push(RustModuleChildEdge {
                    child_path: resolved.clone(),
                    kind: RustModuleChildEdgeKind::PathAttrMod,
                });
            }
            continue;
        }
        let base = child_module_base_dir(module_path);
        let name = &item_mod.ident;
        let file_form = base.join(format!("{name}.rs"));
        if source_files.contains(&file_form) {
            edges.push(RustModuleChildEdge {
                child_path: file_form,
                kind: RustModuleChildEdgeKind::Mod,
            });
        }
        let mod_form = base.join(name).join("mod.rs");
        if source_files.contains(&mod_form) {
            edges.push(RustModuleChildEdge {
                child_path: mod_form,
                kind: RustModuleChildEdgeKind::Mod,
            });
        }
    }
    edges
}

fn child_module_base_dir(module_path: &Path) -> PathBuf {
    let parent = module_path.parent().unwrap_or_else(|| Path::new(""));
    if is_special_rust_entrypoint_path(module_path)
        || module_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "mod.rs")
    {
        return parent.to_path_buf();
    }
    let Some(stem) = module_path.file_stem() else {
        return parent.to_path_buf();
    };
    parent.join(stem)
}

pub(in crate::parser) fn is_module_tree_root(source_paths: &[PathBuf], path: &Path) -> bool {
    source_paths.iter().any(|source_root| {
        if path == source_root.join("lib.rs") || path == source_root.join("main.rs") {
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

fn is_under_any_dir(path: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| path.starts_with(dir))
}
