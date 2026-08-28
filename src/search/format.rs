use std::path::{Path, PathBuf};

use crate::RustHarnessConfig;
use crate::discovery::discover_cargo_package_roots;
use crate::parser::{
    CargoDependencyFacts, CargoDependencyKind, ParsedRustModule, RustReasoningOwnerBranchFacts,
    RustReasoningOwnerBranchRole, RustTopLevelItemSyntax,
    syntax_abi::{RUST_OWNER_ITEMS_QUERY_REF, syntax_atom_for_kind},
};
use crate::path::normalize_lexical_path;

use super::limits::SOURCE_LARGE_EFFECTIVE_LINES;

pub(super) fn render_owner_line(
    package_root: &Path,
    branch: &RustReasoningOwnerBranchFacts,
    parsed_module: Option<&ParsedRustModule>,
) -> String {
    let path = display_project_path(package_root, &branch.path);
    let mut parts = vec![
        format!("|owner {path}"),
        format!("role={}", owner_branch_role_labels(branch).join(",")),
    ];
    if !branch.owner_namespace.is_empty() {
        parts.push(format!("owner={}", branch.owner_namespace.join("/")));
    }
    let imports = compact_import_summary(&branch.import_summary);
    if !imports.is_empty() {
        parts.push(format!("imports={imports}"));
    }
    if parsed_module.is_some_and(|module| {
        module.source_metrics.effective_code_lines > SOURCE_LARGE_EFFECTIVE_LINES
    }) {
        parts.push("source_large=true".to_string());
        parts.push("next=items".to_string());
    } else {
        parts.push(format!("next=owner:{path}"));
    }
    parts.join(" ")
}

fn owner_branch_role_labels(branch: &RustReasoningOwnerBranchFacts) -> Vec<String> {
    branch
        .roles
        .iter()
        .map(|role| match role {
            RustReasoningOwnerBranchRole::Root => "root".to_string(),
            RustReasoningOwnerBranchRole::Facade => "facade".to_string(),
            RustReasoningOwnerBranchRole::Interface => "interface".to_string(),
            RustReasoningOwnerBranchRole::Binary => "binary".to_string(),
            RustReasoningOwnerBranchRole::PackageEntrypoint => "package-entrypoint".to_string(),
            RustReasoningOwnerBranchRole::RepeatedNamespace(segments) => {
                format!("repeated:{}", segments.join(","))
            }
            RustReasoningOwnerBranchRole::Branch => "branch".to_string(),
        })
        .collect()
}

fn compact_import_summary(imports: &crate::parser::RustReasoningImportFacts) -> String {
    let mut parts = Vec::new();
    push_count(&mut parts, "crate", imports.crate_imports);
    push_count(&mut parts, "self", imports.self_imports);
    push_count(&mut parts, "parent", imports.parent_imports);
    push_count(&mut parts, "external", imports.external_imports);
    push_count(&mut parts, "absolute", imports.absolute_imports);
    push_count(&mut parts, "glob", imports.glob_imports);
    push_count(&mut parts, "deep", imports.deep_relative_imports);
    parts.join(",")
}

fn push_count(parts: &mut Vec<String>, label: &str, count: usize) {
    if count > 0 {
        parts.push(format!("{label}:{count}"));
    }
}

pub(super) fn render_cargo_dependency_line(dependency: &CargoDependencyFacts) -> String {
    let mut parts = vec![
        format!("|dep {}", dependency.dependency_key),
        format!("import={}", dependency.import_name),
        format!("pkg={}", dependency.package_name),
        format!(
            "version={}",
            dependency.version_req.as_deref().unwrap_or("-")
        ),
        format!("kind={}", dependency_kind_label(dependency.kind)),
        format!("opt={}", dependency.optional),
        "source=manifest".to_string(),
        "manager=cargo".to_string(),
    ];
    if let Some(target) = &dependency.target {
        parts.push(format!("target={target}"));
    }
    parts.push(format!("feat={}", empty_dash(&dependency.features)));
    parts.join(" ")
}

fn dependency_kind_label(kind: CargoDependencyKind) -> &'static str {
    match kind {
        CargoDependencyKind::Normal => "normal",
        CargoDependencyKind::Dev => "dev",
        CargoDependencyKind::Build => "build",
    }
}

pub(super) fn render_item_line(item: &RustTopLevelItemSyntax) -> String {
    format!(
        "{} syn={}",
        render_item_core_line(item),
        syntax_atom_for_kind(item.kind)
    )
}

fn render_item_core_line(item: &RustTopLevelItemSyntax) -> String {
    fn push_responsibility(responsibilities: &mut Vec<&str>, kind: &'static str) {
        if !responsibilities.contains(&kind) {
            responsibilities.push(kind);
        }
    }

    let name = item_display_name(item);
    let mut fields = vec![format!("|item {name}"), format!("kind={}", item.kind)];
    let mut responsibilities = item.projection_responsibilities.clone();
    for node in &item.projection_nodes {
        match node.role {
            "mutation" => push_responsibility(&mut responsibilities, "state-mutation"),
            "terminal" => push_responsibility(&mut responsibilities, "early-return"),
            "call" => push_responsibility(&mut responsibilities, "call-dispatch"),
            "effect" => push_responsibility(&mut responsibilities, "effect-boundary"),
            "field" => push_responsibility(&mut responsibilities, "data-shape"),
            "control-flow" => match node.kind {
                "if" => push_responsibility(&mut responsibilities, "guard-branch"),
                "match" => push_responsibility(&mut responsibilities, "match-dispatch"),
                "match-arm" => push_responsibility(&mut responsibilities, "match-arm"),
                "for" => push_responsibility(&mut responsibilities, "bounded-loop"),
                _ => push_responsibility(&mut responsibilities, "loop-control"),
            },
            _ => {}
        }
    }
    if !responsibilities.is_empty() {
        fields.push(format!("responsibilities={}", responsibilities.join(",")));
    }
    if item.is_public {
        fields.push("public=true".to_string());
    }
    if item.has_doc {
        fields.push("doc=true".to_string());
    }
    fields.push(format!("next=syntax:{name}"));
    fields.join(" ")
}

fn item_display_name(item: &RustTopLevelItemSyntax) -> &str {
    item.name
        .as_deref()
        .or(item.impl_target_name.as_deref())
        .unwrap_or("-")
}

pub(super) fn render_item_locator_line_with_read(
    package_root: &Path,
    path: &Path,
    item: &RustTopLevelItemSyntax,
) -> String {
    let read_path = display_project_path(package_root, path);
    let symbol = item_display_name(item).replace(char::is_whitespace, "-");
    let kind = canonical_rust_item_kind(item.kind);
    let mut identity =
        agent_semantic_content_identity::CanonicalItemIdentity::new("rust", kind, symbol.as_str());
    if let Some(implementation_owner) = item.impl_target_name.as_deref() {
        identity = identity.with_scope("implementation-owner", "type", implementation_owner);
    }
    if let Some(trait_owner) = item.trait_owner_name.as_deref() {
        identity = identity.with_scope("trait-owner", "trait", trait_owner);
    }
    for predicate in &item.cfg_predicates {
        identity = identity.with_scope("conditional-compilation", "cfg", predicate.as_str());
    }
    render_canonical_item_locator_line(
        read_path.as_str(),
        item.kind,
        item.line,
        item.end_line,
        &identity,
        render_item_core_line(item),
    )
}

pub(super) fn render_projection_item_locator_line_with_read(
    package_root: &Path,
    path: &Path,
    item: &crate::parser::native_syntax::item_projection::RustItemProjectionNodeSyntax,
) -> Option<String> {
    let identity = item.canonical_item_identity.as_ref()?;
    let mut shared_identity = agent_semantic_content_identity::CanonicalItemIdentity::new(
        identity.language_id.as_str(),
        identity.kind.as_str(),
        identity.symbol.as_str(),
    );
    shared_identity.scopes = identity
        .scopes
        .iter()
        .map(|scope| {
            agent_semantic_content_identity::CanonicalItemScope::new(
                scope.relation.as_str(),
                scope.kind.as_str(),
                scope.symbol.as_str(),
            )
        })
        .collect();
    let read_path = display_project_path(package_root, path);
    let symbol = identity.symbol.as_str();
    let identity_kind = identity.kind.as_str();
    Some(render_canonical_item_locator_line(
        read_path.as_str(),
        item.kind,
        item.line,
        item.end_line,
        &shared_identity,
        format!(
            "|item {} kind={} next=syntax:{}",
            symbol, identity_kind, symbol
        ),
    ))
}

fn render_canonical_item_locator_line(
    read_path: &str,
    kind: &str,
    line: usize,
    end_line: usize,
    identity: &agent_semantic_content_identity::CanonicalItemIdentity,
    core_line: String,
) -> String {
    let structural_selector = format!(
        "rust://{read_path}#{}",
        agent_semantic_content_identity::structural_selector::encode_canonical_item_identity_path(
            identity,
        )
    );
    let canonical_item_selector = agent_semantic_content_identity::CanonicalItemSelector::new(
        identity.clone(),
        &structural_selector,
    );
    let canonical_item_selector = serde_json::to_string(&canonical_item_selector)
        .expect("canonical Rust item selector must serialize");
    format!(
        "{} read={}:{}:{} structuralSelector={} canonicalItemSelector={} syn={} tsqRef={}",
        core_line,
        read_path,
        line,
        end_line,
        structural_selector,
        canonical_item_selector,
        syntax_atom_for_kind(kind),
        RUST_OWNER_ITEMS_QUERY_REF
    )
}

pub(crate) fn canonical_rust_item_kind(kind: &str) -> &str {
    match kind {
        "fn" => "function",
        "mod" => "module",
        "use" | "import" => "reexport",
        other => other,
    }
}

pub(super) fn render_public_api_line(
    package_root: &Path,
    path: &Path,
    dependency: &str,
    item: &RustTopLevelItemSyntax,
) -> Option<String> {
    let name = item.name.as_deref().or(item.function_name.as_deref())?;
    Some(format!(
        "|api {} line={} dep={} kind={} name={} public={} doc={} reason=dependency-owner next=docs:{},tests",
        display_project_path(package_root, path),
        item.line,
        dependency,
        item.kind,
        name,
        item.is_public,
        item.has_doc,
        name
    ))
}

pub(super) fn empty_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(",")
    }
}

pub(super) fn compact_locations(locations: &[String]) -> String {
    if locations.is_empty() {
        return "-".to_string();
    }
    let mut selected = locations.iter().take(8).cloned().collect::<Vec<_>>();
    if locations.len() > 8 {
        selected.push(format!("+{}", locations.len() - 8));
    }
    selected.join(",")
}

pub(super) fn sort_locations(locations: &mut [String]) {
    locations.sort_by(|left, right| {
        location_sort_key(left)
            .cmp(&location_sort_key(right))
            .then_with(|| left.cmp(right))
    });
}

fn location_sort_key(location: &str) -> (usize, usize) {
    let mut parts = location.split(':');
    let line = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    let column = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    (line, column)
}

pub(super) fn append_block(rendered: &mut String, block: &str) {
    if !rendered.is_empty() && !rendered.ends_with('\n') && !block.is_empty() {
        rendered.push('\n');
    }
    rendered.push_str(block);
}

pub(super) fn required_query<'a>(view: &str, query: Option<&'a str>) -> Result<&'a str, String> {
    query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .ok_or_else(|| format!("search {view} requires a query"))
}

pub(super) fn query_set_terms(query: &str) -> Vec<&str> {
    query
        .split(',')
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .collect()
}

pub(super) fn owner_role_for_path(package_root: &Path, path: &Path) -> &'static str {
    let display = display_project_path(package_root, path);
    if display.starts_with("tests/") {
        "test"
    } else if display.starts_with("benches/") {
        "bench"
    } else if display.starts_with("examples/") {
        "example"
    } else {
        "source"
    }
}

pub(super) fn package_roots_for_request(
    project_root: &Path,
    config: &RustHarnessConfig,
    selected_package: Option<&str>,
) -> Result<Vec<PathBuf>, String> {
    if !project_root.exists() {
        return Err(format!(
            "project root does not exist: {}",
            project_root.display()
        ));
    }
    let package_roots = discover_cargo_package_roots(
        project_root,
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    );
    let package_roots = if should_run_member_scopes(project_root, &package_roots) {
        package_roots
    } else {
        vec![project_root.to_path_buf()]
    };
    if let Some(selected_package) = selected_package {
        resolve_package_root(project_root, &package_roots, selected_package).map(|root| vec![root])
    } else {
        Ok(package_roots)
    }
}

pub(super) fn package_label(project_root: &Path, package_root: &Path) -> String {
    display_project_path(project_root, package_root)
}

pub(super) fn resolve_package_root(
    project_root: &Path,
    package_roots: &[PathBuf],
    selected_package: &str,
) -> Result<PathBuf, String> {
    let selected = selected_package.trim();
    package_roots
        .iter()
        .find(|package_root| {
            display_project_path(project_root, package_root) == selected
                || package_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == selected)
        })
        .cloned()
        .ok_or_else(|| format!("unknown package for search prime: {selected}"))
}

pub(super) fn should_run_member_scopes(project_root: &Path, package_roots: &[PathBuf]) -> bool {
    package_roots.len() > 1
        || package_roots
            .first()
            .is_some_and(|root| root != project_root)
}

pub(super) fn display_project_path(root: &Path, path: &Path) -> String {
    let root = normalize_lexical_path(root);
    let path = normalize_lexical_path(path);
    path.strip_prefix(&root)
        .map_or_else(|_| display_path(&path), display_path)
}

fn display_path(path: &Path) -> String {
    let rendered = path.display().to_string().replace('\\', "/");
    if rendered.is_empty() {
        ".".to_string()
    } else {
        rendered
    }
}
