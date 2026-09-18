use crate::parser::{
    CargoDependencyKind, cargo_workspace_member_roots_from_candidates, parse_cargo_project_facts,
    workspace_member_pattern_matches,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

struct ServiceTimer {
    wall_started: Instant,
    #[cfg(unix)]
    thread_cpu_started: Option<libc::timespec>,
}

impl ServiceTimer {
    fn start() -> Self {
        Self {
            wall_started: Instant::now(),
            #[cfg(unix)]
            thread_cpu_started: thread_cpu_time(),
        }
    }

    fn elapsed_micros(&self) -> u64 {
        #[cfg(unix)]
        if let (Some(started), Some(completed)) = (self.thread_cpu_started, thread_cpu_time()) {
            let started_nanos = i128::from(started.tv_sec)
                .saturating_mul(1_000_000_000)
                .saturating_add(i128::from(started.tv_nsec));
            let completed_nanos = i128::from(completed.tv_sec)
                .saturating_mul(1_000_000_000)
                .saturating_add(i128::from(completed.tv_nsec));
            let elapsed_nanos = completed_nanos.saturating_sub(started_nanos).max(0) as u128;
            return u64::try_from(elapsed_nanos / 1_000).unwrap_or(u64::MAX);
        }
        u64::try_from(self.wall_started.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

#[cfg(unix)]
fn thread_cpu_time() -> Option<libc::timespec> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `value` is valid writable storage and CLOCK_THREAD_CPUTIME_ID
    // reads only the calling thread's monotonic service time.
    (unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut value) } == 0)
        .then_some(value)
}

pub(crate) const PROJECT_RESOLUTION_SCHEMA_ID: &str = "agent.semantic-protocols.project-resolution";
pub(crate) const LANGUAGE_PACKAGE_GRAPH_SCHEMA_ID: &str =
    "agent.semantic-protocols.language-package-graph";
const PARSER_ID: &str = "rust.cargo-toml";
const PROVIDER_ID: &str = "asp-rust";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectResolutionInput {
    pub(crate) candidate_generation: CandidateGeneration,
    pub(crate) collection_scope: ProjectResolutionCollectionScope,
    pub(crate) candidate_paths: Vec<PathBuf>,
    pub(crate) policy_exclusions: Vec<ProjectResolutionPolicyExclusion>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub(crate) enum ProjectResolutionCollectionScope {
    CompleteGeneration,
    ExplicitOwners {
        #[serde(rename = "ownerPaths")]
        owner_paths: Vec<PathBuf>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CandidateGeneration {
    pub(crate) algorithm: String,
    pub(crate) digest: String,
    pub(crate) authorities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectResolutionPolicyExclusion {
    pub(crate) path: PathBuf,
    pub(crate) authority: String,
    pub(crate) reason_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectResolution {
    pub(crate) schema_id: &'static str,
    pub(crate) schema_version: &'static str,
    pub(crate) state: &'static str,
    pub(crate) completeness: &'static str,
    pub(crate) language_id: &'static str,
    pub(crate) provider_id: &'static str,
    pub(crate) parser_id: &'static str,
    pub(crate) candidate_generation_digest: String,
    pub(crate) project_entry: PathBuf,
    pub(crate) package_graph: LanguagePackageGraph,
    pub(crate) source_scopes: Vec<ResolvedSourceScope>,
    pub(crate) conflicts: Vec<ProjectResolutionConflict>,
    pub(crate) metrics: ProjectResolutionMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanguagePackageGraph {
    pub(crate) schema_id: &'static str,
    pub(crate) schema_version: &'static str,
    pub(crate) language_id: &'static str,
    pub(crate) provider_id: &'static str,
    pub(crate) project_entry: PathBuf,
    pub(crate) parser_id: &'static str,
    pub(crate) manifests: Vec<ProjectFile>,
    pub(crate) lockfiles: Vec<ProjectFile>,
    pub(crate) packages: Vec<LanguagePackage>,
    pub(crate) internal_dependency_edges: Vec<InternalDependencyEdge>,
    pub(crate) external_dependencies: Vec<ExternalDependency>,
    pub(crate) unresolved: Vec<UnresolvedProjectReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectFile {
    pub(crate) path: PathBuf,
    pub(crate) kind: &'static str,
    pub(crate) digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanguagePackage {
    pub(crate) package_id: String,
    pub(crate) name: String,
    pub(crate) root: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) workspace_member: bool,
    pub(crate) targets: Vec<LanguageTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanguageTarget {
    pub(crate) target_id: String,
    pub(crate) name: String,
    pub(crate) kind: &'static str,
    pub(crate) explicit: bool,
    pub(crate) source_roots: Vec<PathBuf>,
    pub(crate) entrypoints: Vec<PathBuf>,
    pub(crate) generated_roots: Vec<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InternalDependencyEdge {
    pub(crate) from_package_id: String,
    pub(crate) to_package_id: String,
    pub(crate) kind: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalDependency {
    pub(crate) dependency_id: String,
    pub(crate) name: String,
    pub(crate) kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) requested: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UnresolvedProjectReference {
    pub(crate) state: &'static str,
    pub(crate) path: PathBuf,
    pub(crate) reason_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedSourceScope {
    pub(crate) scope_id: String,
    pub(crate) package_id: String,
    pub(crate) target_id: String,
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) explicit_paths: Vec<PathBuf>,
    pub(crate) extensions: Vec<&'static str>,
    pub(crate) include_authority: &'static str,
    pub(crate) exclusions: Vec<SourceScopeExclusion>,
    pub(crate) classifications: Vec<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceScopeExclusion {
    pub(crate) prefix: PathBuf,
    pub(crate) authority: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectResolutionConflict {
    pub(crate) path: PathBuf,
    pub(crate) include_authority: String,
    pub(crate) exclude_authority: String,
    pub(crate) reason_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectResolutionMetrics {
    pub(crate) parsed_manifest_count: usize,
    pub(crate) parsed_lockfile_count: usize,
    pub(crate) affected_package_count: usize,
    pub(crate) full_workspace_reads: usize,
    pub(crate) full_manifest_reparses: usize,
    pub(crate) db_opens: usize,
    pub(crate) elapsed_micros: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProjectResolutionError {
    NotApplicable { expected: PathBuf },
    ProjectEntryMissing { expected: PathBuf },
    ProjectEntryInvalid { path: PathBuf },
}

impl std::fmt::Display for ProjectResolutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotApplicable { expected } => write!(
                formatter,
                "provider-not-applicable: no parser-owned manifest `{}` is present",
                expected.display()
            ),
            Self::ProjectEntryMissing { expected } => write!(
                formatter,
                "project-entry-missing: expected parser-owned manifest `{}`",
                expected.display()
            ),
            Self::ProjectEntryInvalid { path } => write!(
                formatter,
                "project-entry-invalid: Cargo manifest `{}` declares neither a package nor workspace members",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ProjectResolutionError {}

pub(crate) fn resolve_cargo_project_resolution(
    workspace: &Path,
    input: &ProjectResolutionInput,
) -> Result<ProjectResolution, ProjectResolutionError> {
    let started = ServiceTimer::start();
    let workspace = workspace.to_path_buf();
    let candidate_paths = input
        .candidate_paths
        .iter()
        .map(|candidate| normalize_relative_path(candidate))
        .collect::<BTreeSet<_>>();
    let root_manifest_relative = PathBuf::from("Cargo.toml");
    let has_any_manifest = candidate_paths.iter().any(|path| {
        path.file_name()
            .is_some_and(|name| name == std::ffi::OsStr::new("Cargo.toml"))
    });
    if !has_any_manifest {
        return Err(ProjectResolutionError::NotApplicable {
            expected: root_manifest_relative,
        });
    }
    if !candidate_paths.contains(&root_manifest_relative) {
        return Err(ProjectResolutionError::ProjectEntryMissing {
            expected: root_manifest_relative,
        });
    }
    let root_candidates = package_candidate_paths(&candidate_paths, Path::new("."));
    let (root_facts, root_dependencies, root_manifest) =
        parse_cargo_project_facts(&workspace, &root_candidates, None);
    let root_manifest_path = workspace.join("Cargo.toml");
    let member_roots =
        cargo_workspace_member_roots_from_candidates(&workspace, &root_facts, &candidate_paths);
    let mut package_roots = BTreeSet::new();
    let mut pending_package_roots = BTreeSet::new();
    if root_facts.has_package {
        pending_package_roots.insert(workspace.clone());
    }
    for member_root in member_roots {
        pending_package_roots.insert(member_root);
    }
    pending_package_roots.extend(root_facts.path_dependency_roots.iter().cloned());
    if pending_package_roots.is_empty() {
        return Err(ProjectResolutionError::ProjectEntryInvalid {
            path: root_manifest_path,
        });
    }

    while let Some(package_root) = pending_package_roots.pop_first() {
        let Some(relative_root) = admitted_workspace_package_root(
            &workspace,
            &package_root,
            &candidate_paths,
            &root_facts.workspace_excludes,
        ) else {
            continue;
        };
        let package_root = workspace.join(&relative_root);
        if !package_roots.insert(package_root.clone()) {
            continue;
        }
        let package_candidates = package_candidate_paths(&candidate_paths, &relative_root);
        let (facts, _, _) = if package_root == workspace {
            (
                root_facts.clone(),
                root_dependencies.clone(),
                root_manifest.clone(),
            )
        } else {
            parse_cargo_project_facts(
                &package_root,
                &package_candidates,
                root_manifest
                    .as_ref()
                    .map(|manifest| (manifest, root_manifest_path.as_path())),
            )
        };
        pending_package_roots.extend(facts.path_dependency_roots);
    }

    let mut packages = Vec::new();
    let mut dependencies_by_package = Vec::new();
    let mut manifest_paths = vec![PathBuf::from("Cargo.toml")];
    let mut unresolved = Vec::new();
    for package_root in package_roots {
        let relative_root = relative_to(&workspace, &package_root);
        let (facts, dependencies) = if package_root == workspace {
            (root_facts.clone(), root_dependencies.clone())
        } else {
            let package_candidates = package_candidate_paths(&candidate_paths, &relative_root);
            let (facts, dependencies, _) = parse_cargo_project_facts(
                &package_root,
                &package_candidates,
                root_manifest
                    .as_ref()
                    .map(|manifest| (manifest, root_manifest_path.as_path())),
            );
            (facts, dependencies)
        };
        let Some(package_name) = facts.package_name.clone() else {
            unresolved.push(UnresolvedProjectReference {
                state: "manifest-invalid",
                path: relative_to(&workspace, &package_root.join("Cargo.toml")),
                reason_kind: "package-name-missing".to_string(),
            });
            continue;
        };
        let manifest_path = relative_root.join("Cargo.toml");
        manifest_paths.push(manifest_path.clone());
        let package_id = stable_id(
            "cargo-package",
            format!("{package_name}\0{}", relative_root.display()).as_bytes(),
        );
        let targets = cargo_targets(&workspace, &package_id, &facts);
        dependencies_by_package.push((package_id.clone(), dependencies));
        packages.push(LanguagePackage {
            package_id,
            name: package_name,
            root: display_root(&relative_root),
            manifest_path,
            workspace_member: package_root != workspace,
            targets,
        });
    }
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    manifest_paths.sort();
    manifest_paths.dedup();

    let package_ids = packages
        .iter()
        .map(|package| (package.name.clone(), package.package_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut internal_dependency_edges = Vec::new();
    let mut external_dependencies = Vec::new();
    for (from_package_id, facts) in dependencies_by_package {
        for dependency in facts {
            let kind = dependency_kind(dependency.kind);
            if let Some(to_package_id) = package_ids.get(&dependency.package_name) {
                internal_dependency_edges.push(InternalDependencyEdge {
                    from_package_id: from_package_id.clone(),
                    to_package_id: to_package_id.clone(),
                    kind,
                });
            } else {
                external_dependencies.push(ExternalDependency {
                    dependency_id: stable_id(
                        "cargo-dependency",
                        format!("{from_package_id}\0{}\0{kind}", dependency.package_name)
                            .as_bytes(),
                    ),
                    name: dependency.package_name,
                    kind,
                    requested: dependency.version_req,
                });
            }
        }
    }
    internal_dependency_edges.sort_by(|left, right| {
        (&left.from_package_id, &left.to_package_id, left.kind).cmp(&(
            &right.from_package_id,
            &right.to_package_id,
            right.kind,
        ))
    });
    external_dependencies.sort_by(|left, right| left.dependency_id.cmp(&right.dependency_id));

    let manifests = manifest_paths
        .iter()
        .map(|path| project_file(&workspace, path, "cargo-manifest"))
        .collect::<Vec<_>>();
    let lockfiles = candidate_paths
        .contains(Path::new("Cargo.lock"))
        .then(|| project_file(&workspace, Path::new("Cargo.lock"), "cargo-lock"))
        .into_iter()
        .collect::<Vec<_>>();
    let package_graph = LanguagePackageGraph {
        schema_id: LANGUAGE_PACKAGE_GRAPH_SCHEMA_ID,
        schema_version: "1",
        language_id: "rust",
        provider_id: PROVIDER_ID,
        project_entry: PathBuf::from("Cargo.toml"),
        parser_id: PARSER_ID,
        manifests,
        lockfiles,
        packages,
        internal_dependency_edges,
        external_dependencies,
        unresolved,
    };
    let source_scopes = package_graph
        .packages
        .iter()
        .flat_map(|package| {
            package.targets.iter().map(|target| ResolvedSourceScope {
                scope_id: stable_id(
                    "cargo-source-scope",
                    format!("{}\0{}", package.package_id, target.target_id).as_bytes(),
                ),
                package_id: package.package_id.clone(),
                target_id: target.target_id.clone(),
                roots: target.source_roots.clone(),
                explicit_paths: if target.explicit {
                    target.entrypoints.clone()
                } else {
                    Vec::new()
                },
                extensions: vec![".rs"],
                include_authority: if target.explicit {
                    "manifest-explicit"
                } else {
                    "package-manager"
                },
                exclusions: Vec::new(),
                classifications: vec![target.kind],
            })
        })
        .collect::<Vec<_>>();
    let affected_package_count = package_graph.packages.len();

    Ok(ProjectResolution {
        schema_id: PROJECT_RESOLUTION_SCHEMA_ID,
        schema_version: "1",
        state: "resolved",
        completeness: "exact",
        language_id: "rust",
        provider_id: PROVIDER_ID,
        parser_id: PARSER_ID,
        candidate_generation_digest: input.candidate_generation.digest.clone(),
        project_entry: PathBuf::from("Cargo.toml"),
        source_scopes,
        conflicts: Vec::new(),
        metrics: ProjectResolutionMetrics {
            parsed_manifest_count: package_graph.manifests.len(),
            parsed_lockfile_count: package_graph.lockfiles.len(),
            affected_package_count,
            full_workspace_reads: 0,
            full_manifest_reparses: 0,
            db_opens: 0,
            elapsed_micros: started.elapsed_micros(),
        },
        package_graph,
    })
}

fn package_candidate_paths(
    workspace_candidates: &BTreeSet<PathBuf>,
    package_root: &Path,
) -> BTreeSet<PathBuf> {
    let package_root = (package_root != Path::new(".")).then_some(package_root);
    workspace_candidates
        .iter()
        .filter_map(|path| match package_root {
            Some(root) => path.strip_prefix(root).ok().map(Path::to_path_buf),
            None => Some(path.clone()),
        })
        .collect()
}

fn admitted_workspace_package_root(
    workspace: &Path,
    package_root: &Path,
    candidate_paths: &BTreeSet<PathBuf>,
    workspace_excludes: &[String],
) -> Option<PathBuf> {
    let relative_root = package_root.strip_prefix(workspace).ok()?;
    let relative_root = normalize_relative_path(relative_root);
    if !candidate_paths.contains(&relative_root.join("Cargo.toml")) {
        return None;
    }
    let display = relative_root.to_string_lossy().replace('\\', "/");
    if workspace_excludes
        .iter()
        .any(|pattern| workspace_member_pattern_matches(pattern, &display))
    {
        return None;
    }
    Some(relative_root)
}

fn cargo_targets(
    workspace: &Path,
    package_id: &str,
    facts: &crate::parser::CargoManifestFacts,
) -> Vec<LanguageTarget> {
    let mut targets = facts
        .package_targets
        .iter()
        .map(|target| {
            let entrypoint = relative_to(workspace, &target.path);
            language_target(
                package_id,
                target.name.clone(),
                target.kind,
                entrypoint.clone(),
                !is_cargo_default_entrypoint(&entrypoint, target.kind),
            )
        })
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| left.target_id.cmp(&right.target_id));
    targets.dedup();
    targets
}

fn language_target(
    package_id: &str,
    name: String,
    kind: &'static str,
    entrypoint: PathBuf,
    explicit: bool,
) -> LanguageTarget {
    let target_id = stable_id(
        "cargo-target",
        format!("{package_id}\0{name}\0{kind}\0{}", entrypoint.display()).as_bytes(),
    );
    LanguageTarget {
        target_id,
        name,
        kind,
        explicit,
        source_roots: target_source_roots(&entrypoint),
        entrypoints: vec![entrypoint],
        generated_roots: Vec::new(),
    }
}

fn target_source_roots(entrypoint: &Path) -> Vec<PathBuf> {
    let source_root = entrypoint
        .components()
        .scan(PathBuf::new(), |prefix, component| {
            prefix.push(component);
            Some(prefix.clone())
        })
        .find(|prefix| {
            prefix.file_name().is_some_and(|name| {
                matches!(
                    name.to_str(),
                    Some("src" | "tests" | "examples" | "benches")
                )
            })
        });
    vec![source_root.unwrap_or_else(|| {
        entrypoint
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    })]
}

fn is_cargo_default_entrypoint(path: &Path, kind: &str) -> bool {
    let file_name = path.file_name().and_then(|name| name.to_str());
    let parent_name = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    parent_name == Some("src")
        && matches!(
            (kind, file_name),
            ("library", Some("lib.rs")) | ("binary", Some("main.rs"))
        )
}

fn dependency_kind(kind: CargoDependencyKind) -> &'static str {
    match kind {
        CargoDependencyKind::Normal => "normal",
        CargoDependencyKind::Dev => "dev",
        CargoDependencyKind::Build => "build",
    }
}

fn project_file(workspace: &Path, relative: &Path, kind: &'static str) -> ProjectFile {
    let digest = std::fs::read(workspace.join(relative))
        .map(|contents| format!("blake3:{}", blake3::hash(&contents).to_hex()))
        .unwrap_or_else(|_| "blake3:missing".to_string());
    ProjectFile {
        path: relative.to_path_buf(),
        kind,
        digest,
    }
}

fn display_root(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    }
}

fn relative_to(workspace: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(workspace).unwrap_or(path).to_path_buf()
}

fn normalize_relative_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir | std::path::Component::RootDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            std::path::Component::Normal(component) => normalized.push(component),
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
#[path = "../tests/unit/project_resolution.rs"]
mod tests;

fn stable_id(namespace: &str, basis: &[u8]) -> String {
    let mut digest = blake3::Hasher::new();
    digest.update(namespace.as_bytes());
    digest.update(b"\0");
    digest.update(basis);
    format!("{namespace}-{}", &digest.finalize().to_hex()[..16])
}
