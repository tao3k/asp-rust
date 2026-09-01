//! Provider-owned Cargo dependency topology packet rendering.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::parser::{
    CargoDependencyFacts, CargoDependencyKind, parse_cargo_dependency_facts, parse_cargo_manifest,
    parse_cargo_workspace_member_roots,
};

use super::contract::{LANGUAGE_ID, PROVIDER_ID};
use super::graph_helpers::{display_project_path, stable_node_id};

const SCHEMA_ID: &str = "agent.semantic-protocols.semantic-dependency-topology";
const SCHEMA_VERSION: &str = "1";
const PROTOCOL_ID: &str = "agent.semantic-protocols.semantic-language";
const PROTOCOL_VERSION: &str = "1";
const PACKET_KIND: &str = "dependency-topology";
const METADATA_PACKET_KIND: &str = "dependency-topology-metadata";
const PACKAGE_MANAGER: &str = "cargo";
const MANIFEST_FILE: &str = "Cargo.toml";
const LOCKFILE_FILE: &str = "Cargo.lock";

/// Renders Cargo dependency topology metadata for cache and freshness checks.
pub fn render_asp_rust_dependency_topology_metadata_json(
    project_root: &Path,
) -> Result<String, String> {
    let packet = build_dependency_topology_metadata_packet(project_root);
    serde_json::to_string_pretty(&packet)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .map_err(|error| format!("failed to render dependency topology metadata JSON: {error}"))
}

/// Renders the Cargo dependency topology packet for Rust search facts.
pub fn render_asp_rust_dependency_topology_json(project_root: &Path) -> Result<String, String> {
    let packet = build_dependency_topology_packet(project_root);
    serde_json::to_string_pretty(&packet)
        .map(|mut text| {
            text.push('\n');
            text
        })
        .map_err(|error| format!("failed to render dependency topology JSON: {error}"))
}

fn build_dependency_topology_metadata_packet(project_root: &Path) -> Value {
    let metadata = dependency_topology_metadata(project_root);
    let fingerprint = cache_key_fingerprint(
        &metadata.project_package_name,
        &metadata.manifest_hash,
        &metadata.lockfile_hash,
    );
    json!({
        "schemaId": SCHEMA_ID,
        "schemaVersion": SCHEMA_VERSION,
        "protocolId": PROTOCOL_ID,
        "protocolVersion": PROTOCOL_VERSION,
        "packetKind": METADATA_PACKET_KIND,
        "languageId": LANGUAGE_ID,
        "projectRoot": project_root.display().to_string().replace('\\', "/"),
        "fingerprint": fingerprint,
        "generatedAt": generated_at(),
        "cacheKey": {
            "languageId": LANGUAGE_ID,
            "packageManager": PACKAGE_MANAGER,
            "manifestHash": metadata.manifest_hash,
            "lockfileHash": metadata.lockfile_hash,
            "projectPackageName": metadata.project_package_name,
        },
        "sourceSummary": {
            "manifestCount": metadata.manifests.len(),
            "lockfileCount": metadata.lockfiles.len(),
            "usageSiteCount": 0,
        },
    })
}

fn build_dependency_topology_packet(project_root: &Path) -> Value {
    let metadata = dependency_topology_metadata(project_root);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes = BTreeSet::new();
    let mut seen_edges = BTreeSet::new();
    let workspace_id = workspace_node_id(&metadata.project_package_name);
    push_node(
        &mut nodes,
        &mut seen_nodes,
        json!({
            "id": workspace_id,
            "kind": "workspace",
            "role": "cargo-workspace",
            "value": metadata.project_package_name,
            "action": "package",
            "path": MANIFEST_FILE,
            "startLine": 1,
            "endLine": 1,
            "locator": format!("{MANIFEST_FILE}:1:1"),
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "packageManager": PACKAGE_MANAGER,
                "provenance": "parser",
                "confidence": "exact",
                "freshness": "fresh",
            },
        }),
    );

    for package_root in &metadata.package_roots {
        push_package_topology(
            project_root,
            package_root,
            &workspace_id,
            &mut nodes,
            &mut edges,
            &mut seen_nodes,
            &mut seen_edges,
        );
    }

    let fingerprint = topology_fingerprint(
        &metadata.project_package_name,
        &metadata.manifest_hash,
        &metadata.lockfile_hash,
        &nodes,
        &edges,
    );
    json!({
        "schemaId": SCHEMA_ID,
        "schemaVersion": SCHEMA_VERSION,
        "protocolId": PROTOCOL_ID,
        "protocolVersion": PROTOCOL_VERSION,
        "packetKind": PACKET_KIND,
        "languageId": LANGUAGE_ID,
        "projectRoot": project_root.display().to_string().replace('\\', "/"),
        "fingerprint": fingerprint,
        "generatedAt": generated_at(),
        "cacheKey": {
            "languageId": LANGUAGE_ID,
            "packageManager": PACKAGE_MANAGER,
            "manifestHash": metadata.manifest_hash,
            "lockfileHash": metadata.lockfile_hash,
            "projectPackageName": metadata.project_package_name,
        },
        "sources": {
            "manifests": source_values(metadata.manifests),
            "lockfiles": source_values(metadata.lockfiles),
            "usageSites": [],
        },
        "graph": {
            "nodes": nodes,
            "edges": edges,
        },
    })
}

fn dependency_topology_metadata(project_root: &Path) -> DependencyTopologyMetadata {
    let root_manifest = parse_cargo_manifest(project_root);
    let project_package_name = root_manifest
        .package_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| project_root_name(project_root));
    let package_roots = cargo_package_roots(project_root, root_manifest.package_name.is_some());
    let manifests = source_files(
        project_root,
        std::iter::once(project_root.to_path_buf())
            .chain(package_roots.iter().cloned())
            .map(|root| root.join(MANIFEST_FILE)),
    );
    let lockfiles = source_files(project_root, [project_root.join(LOCKFILE_FILE)]);
    let manifest_hash = combined_sources_hash(&manifests);
    let lockfile_hash = combined_sources_hash(&lockfiles);
    DependencyTopologyMetadata {
        project_package_name,
        package_roots,
        manifests,
        lockfiles,
        manifest_hash,
        lockfile_hash,
    }
}

pub(super) fn cargo_package_roots(project_root: &Path, root_has_package: bool) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    if root_has_package {
        roots.insert(project_root.to_path_buf());
    }
    roots.extend(parse_cargo_workspace_member_roots(project_root));
    roots.into_iter().collect()
}

fn push_package_topology(
    project_root: &Path,
    package_root: &Path,
    workspace_id: &str,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let manifest = parse_cargo_manifest(package_root);
    let Some(package_name) = manifest.package_name.as_deref() else {
        return;
    };
    let manifest_path = display_project_path(project_root, &package_root.join(MANIFEST_FILE));
    let package_id = package_node_id(package_name);
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": package_id,
            "kind": "package",
            "role": "crate",
            "value": package_name,
            "action": "package",
            "path": manifest_path,
            "startLine": 1,
            "endLine": 1,
            "locator": format!("{manifest_path}:1:1"),
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "packageManager": PACKAGE_MANAGER,
                "provenance": "parser",
                "confidence": "exact",
                "freshness": "fresh",
                "packageName": package_name,
                "manifestPath": manifest_path,
            },
        }),
    );
    push_edge(edges, seen_edges, workspace_id, &package_id, "contains");
    for dependency in parse_cargo_dependency_facts(package_root) {
        push_dependency_topology(
            package_name,
            &manifest_path,
            &dependency,
            nodes,
            edges,
            seen_nodes,
            seen_edges,
        );
    }
}

fn push_dependency_topology(
    package_name: &str,
    manifest_path: &str,
    dependency: &CargoDependencyFacts,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
    seen_nodes: &mut BTreeSet<String>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
) {
    let package_id = package_node_id(package_name);
    let dependency_group = dependency_kind_label(dependency.kind);
    let dependency_id = dependency_node_id(package_name, dependency);
    push_node(
        nodes,
        seen_nodes,
        json!({
            "id": dependency_id,
            "kind": "dependency",
            "role": dependency_group,
            "value": dependency.package_name,
            "action": "deps",
            "path": manifest_path,
            "startLine": 1,
            "endLine": 1,
            "locator": format!("{manifest_path}:1:1"),
            "fields": {
                "languageId": LANGUAGE_ID,
                "providerId": PROVIDER_ID,
                "packageManager": PACKAGE_MANAGER,
                "dependencyName": dependency.package_name,
                "dependencyGroup": dependency_group,
                "dependencyKey": dependency.dependency_key,
                "importName": dependency.import_name,
                "optional": dependency.optional,
                "features": dependency.features,
                "target": dependency.target,
                "provenance": "parser",
                "confidence": "exact",
                "freshness": "fresh",
            },
        }),
    );
    push_edge(edges, seen_edges, &package_id, &dependency_id, "depends_on");
    if let Some(version_req) = dependency.version_req.as_deref() {
        let version_id = dependency_version_node_id(package_name, dependency, version_req);
        push_node(
            nodes,
            seen_nodes,
            json!({
                "id": version_id,
                "kind": "dependency-version",
                "role": "version-req",
                "value": version_req,
                "action": "deps",
                "path": manifest_path,
                "startLine": 1,
                "endLine": 1,
                "locator": format!("{manifest_path}:1:1"),
                "fields": {
                    "languageId": LANGUAGE_ID,
                    "providerId": PROVIDER_ID,
                    "packageManager": PACKAGE_MANAGER,
                    "dependencyName": dependency.package_name,
                    "dependencyGroup": dependency_group,
                    "version": version_req,
                    "provenance": "parser",
                    "confidence": "exact",
                    "freshness": "fresh",
                },
            }),
        );
        push_edge(
            edges,
            seen_edges,
            &dependency_id,
            &version_id,
            "version_locked",
        );
    }
}

#[derive(Clone)]
struct SourceFile {
    path: String,
    sha256: String,
}

struct DependencyTopologyMetadata {
    project_package_name: String,
    package_roots: Vec<PathBuf>,
    manifests: Vec<SourceFile>,
    lockfiles: Vec<SourceFile>,
    manifest_hash: String,
    lockfile_hash: String,
}

fn source_files(project_root: &Path, paths: impl IntoIterator<Item = PathBuf>) -> Vec<SourceFile> {
    let mut seen = BTreeSet::new();
    let mut sources = paths
        .into_iter()
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let display_path = display_project_path(project_root, &path);
            seen.insert(display_path.clone()).then(|| SourceFile {
                path: display_path,
                sha256: file_sha256(&path),
            })
        })
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| left.path.cmp(&right.path));
    sources
}

fn source_values(sources: Vec<SourceFile>) -> Vec<Value> {
    sources
        .into_iter()
        .map(|source| {
            json!({
                "path": source.path,
                "sha256": source.sha256,
            })
        })
        .collect()
}

fn combined_sources_hash(sources: &[SourceFile]) -> String {
    let mut hasher = Sha256::new();
    for source in sources {
        hasher.update(source.path.as_bytes());
        hasher.update([0]);
        hasher.update(source.sha256.as_bytes());
        hasher.update([0]);
    }
    sha256_string(hasher.finalize())
}

fn file_sha256(path: &Path) -> String {
    let content = fs::read(path).unwrap_or_else(|error| {
        format!(
            "missing-dependency-topology-source:{}:{error}",
            path.display()
        )
        .into_bytes()
    });
    let mut hasher = Sha256::new();
    hasher.update(content);
    sha256_string(hasher.finalize())
}

fn cache_key_fingerprint(
    project_package_name: &str,
    manifest_hash: &str,
    lockfile_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(LANGUAGE_ID.as_bytes());
    hasher.update([0]);
    hasher.update(PACKAGE_MANAGER.as_bytes());
    hasher.update([0]);
    hasher.update(project_package_name.as_bytes());
    hasher.update([0]);
    hasher.update(manifest_hash.as_bytes());
    hasher.update([0]);
    hasher.update(lockfile_hash.as_bytes());
    sha256_string(hasher.finalize())
}

fn topology_fingerprint(
    project_package_name: &str,
    manifest_hash: &str,
    lockfile_hash: &str,
    nodes: &[Value],
    edges: &[Value],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_package_name.as_bytes());
    hasher.update([0]);
    hasher.update(manifest_hash.as_bytes());
    hasher.update([0]);
    hasher.update(lockfile_hash.as_bytes());
    hasher.update([0]);
    for node in nodes {
        hasher.update(node.get("id").and_then(Value::as_str).unwrap_or_default());
        hasher.update([0]);
        hasher.update(node.get("kind").and_then(Value::as_str).unwrap_or_default());
        hasher.update([0]);
        hasher.update(
            node.get("value")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        hasher.update([0]);
    }
    for edge in edges {
        hasher.update(
            edge.get("source")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        hasher.update([0]);
        hasher.update(
            edge.get("target")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        hasher.update([0]);
        hasher.update(
            edge.get("relation")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        hasher.update([0]);
    }
    sha256_string(hasher.finalize())
}

fn sha256_string(bytes: impl AsRef<[u8]>) -> String {
    let mut rendered = String::from("sha256:");
    for byte in bytes.as_ref() {
        use std::fmt::Write as _;
        let _ = write!(&mut rendered, "{byte:02x}");
    }
    rendered
}

fn generated_at() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("unix:{seconds}")
}

fn project_root_name(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("workspace")
        .to_string()
}

fn push_node(nodes: &mut Vec<Value>, seen_nodes: &mut BTreeSet<String>, node: Value) {
    let Some(id) = node.get("id").and_then(Value::as_str) else {
        return;
    };
    if seen_nodes.insert(id.to_string()) {
        nodes.push(node);
    }
}

fn push_edge(
    edges: &mut Vec<Value>,
    seen_edges: &mut BTreeSet<(String, String, String)>,
    source: &str,
    target: &str,
    relation: &str,
) {
    let key = (source.to_string(), target.to_string(), relation.to_string());
    if seen_edges.insert(key) {
        edges.push(json!({
            "source": source,
            "target": target,
            "relation": relation,
        }));
    }
}

fn workspace_node_id(project_package_name: &str) -> String {
    stable_node_id("workspace", project_package_name)
}

fn package_node_id(package_name: &str) -> String {
    stable_node_id("package", package_name)
}

fn dependency_node_id(package_name: &str, dependency: &CargoDependencyFacts) -> String {
    stable_node_id(
        "dependency",
        &format!(
            "{}:{}:{}:{}",
            package_name,
            dependency_kind_label(dependency.kind),
            dependency.package_name,
            dependency.target.as_deref().unwrap_or("all")
        ),
    )
}

fn dependency_version_node_id(
    package_name: &str,
    dependency: &CargoDependencyFacts,
    version_req: &str,
) -> String {
    stable_node_id(
        "dependency-version",
        &format!(
            "{}:{}:{}:{}",
            package_name,
            dependency.package_name,
            dependency_kind_label(dependency.kind),
            version_req
        ),
    )
}

fn dependency_kind_label(kind: CargoDependencyKind) -> &'static str {
    match kind {
        CargoDependencyKind::Normal => "normal",
        CargoDependencyKind::Dev => "dev",
        CargoDependencyKind::Build => "build",
    }
}
