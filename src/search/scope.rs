use std::path::Path;

use crate::RustProjectHarnessScope;
use crate::parser::{ParsedRustModule, RustReasoningOwnerBranchFacts};

use super::RustSearchOptions;
use super::context::PackageSearchContext;
use super::format::display_project_path;

pub(super) fn module_allowed(
    context: &PackageSearchContext,
    module: &ParsedRustModule,
    options: &RustSearchOptions,
) -> bool {
    options
        .owner
        .as_deref()
        .is_none_or(|owner| owner_path_matches(&context.package_root, &module.report.path, owner))
        && path_allowed_by_scope(
            &context.scope,
            &context.package_root,
            &module.report.path,
            options,
        )
}

pub(super) fn path_allowed_by_scope(
    project_resolution: &RustProjectHarnessScope,
    package_root: &Path,
    path: &Path,
    options: &RustSearchOptions,
) -> bool {
    options.scope.as_deref().is_none_or(|scope| {
        scope_terms(scope).any(|term| {
            term == "all"
                || path_is_scope(project_resolution, path, term)
                || owner_path_matches(package_root, path, term)
        })
    })
}

fn scope_terms(scope: &str) -> impl Iterator<Item = &str> {
    scope
        .split(',')
        .map(str::trim)
        .filter(|term| !term.is_empty())
}

pub(super) fn module_is_scope(
    scope: &RustProjectHarnessScope,
    module: &ParsedRustModule,
    scope_name: &str,
) -> bool {
    path_is_scope(scope, &module.report.path, scope_name)
}

fn path_is_scope(scope: &RustProjectHarnessScope, path: &Path, scope_name: &str) -> bool {
    match scope_name {
        "src" => scope
            .source_paths
            .iter()
            .any(|scope_path| path.starts_with(scope_path)),
        "tests" => scope
            .test_paths
            .iter()
            .any(|scope_path| path.starts_with(scope_path)),
        "benches" => path
            .components()
            .any(|component| component.as_os_str().to_string_lossy() == "benches"),
        "examples" => path
            .components()
            .any(|component| component.as_os_str().to_string_lossy() == "examples"),
        "build" => path.file_name().and_then(|name| name.to_str()) == Some("build.rs"),
        _ => false,
    }
}

pub(super) fn owner_branch_matches(
    package_root: &Path,
    branch: &RustReasoningOwnerBranchFacts,
    query: &str,
) -> bool {
    owner_path_matches(package_root, &branch.path, query)
        || branch.owner_namespace.join("/").contains(query)
        || branch.owner_namespace.join("::").contains(query)
}

pub(super) fn owner_path_matches(package_root: &Path, path: &Path, query: &str) -> bool {
    let display = display_project_path(package_root, path);
    display == query
        || display.ends_with(query)
        || display.contains(query)
        || query_matches_workspace_relative_path(package_root, path, query)
        || query_matches_absolute_path_from_ancestor(package_root, path, query)
}

fn query_matches_workspace_relative_path(package_root: &Path, path: &Path, query: &str) -> bool {
    let query_path = Path::new(query);
    if query_path.is_absolute() {
        return false;
    }
    package_root
        .ancestors()
        .map(|ancestor| ancestor.join(query_path))
        .any(|candidate| path.starts_with(candidate))
}

fn query_matches_absolute_path_from_ancestor(
    package_root: &Path,
    path: &Path,
    query: &str,
) -> bool {
    let query_path = Path::new(query);
    !query_path.is_absolute()
        && package_root
            .ancestors()
            .any(|ancestor| ancestor.join(query_path) == path)
}
