use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::{
    RustHarnessConfig, RustHarnessReport, RustProjectHarnessDependencyBaselinePackageReceipt,
    RustProjectHarnessDownstreamPolicyReceipt, RustVerificationPlan,
};
use sha2::{Digest, Sha256};

use crate::runner::RustHarnessRunScope;

pub(super) const RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_ID: &str =
    "agent.semantic-protocols.rust-project-harness.build-gate-cache";
pub(super) const RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_VERSION: &str = "1";

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RustProjectHarnessBuildGateSnapshot {
    pub digest: String,
    pub file_count: usize,
    pub byte_count: u64,
    pub files: Vec<RustProjectHarnessBuildGateSnapshotFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RustProjectHarnessBuildGateSnapshotFile {
    pub path: PathBuf,
    pub byte_count: u64,
    pub content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RustProjectHarnessBuildGateCacheRecord {
    pub schema_id: String,
    pub schema_version: String,
    pub cache_key: String,
    pub snapshot: RustProjectHarnessBuildGateSnapshot,
    pub payload_digest: String,
    pub report: RustHarnessReport,
    pub verification_plan: RustVerificationPlan,
    pub downstream_policy_receipt: RustProjectHarnessDownstreamPolicyReceipt,
    pub dependency_baseline_receipts: Vec<RustProjectHarnessDependencyBaselinePackageReceipt>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RustProjectHarnessBuildGateCacheKey<'a> {
    schema_id: &'static str,
    schema_version: &'static str,
    harness_version: &'static str,
    harness_provider_digest: &'a str,
    scope: &'static str,
    config: &'a RustHarnessConfig,
    dependency_baseline_receipts: &'a [RustProjectHarnessDependencyBaselinePackageReceipt],
    content_snapshot_digest: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RustProjectHarnessBuildGateCachePayload<'a> {
    report: &'a RustHarnessReport,
    verification_plan: &'a RustVerificationPlan,
    downstream_policy_receipt: &'a RustProjectHarnessDownstreamPolicyReceipt,
    dependency_baseline_receipts: &'a [RustProjectHarnessDependencyBaselinePackageReceipt],
}

pub(super) fn build_gate_cache_root_from_env(project_root: &Path) -> Option<PathBuf> {
    build_gate_cache_root(project_root, std::env::var_os("ASP_STATE_HOME"))
}

pub(super) fn build_gate_cache_root(
    project_root: &Path,
    state_home: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    let canonical_root = project_root.canonicalize().ok()?;
    let project_identity = cache_digest_hex(
        b"rust-lang-project-harness.build-gate-cache.project.v1",
        canonical_root.as_os_str().as_encoded_bytes(),
    );
    let base = PathBuf::from(state_home?)
        .join("runtime")
        .join("build-gates");
    Some(project_cache_root(base, &project_identity))
}

pub(super) fn snapshot_build_gate_inputs(
    project_root: &Path,
    config: &RustHarnessConfig,
) -> Result<RustProjectHarnessBuildGateSnapshot, String> {
    let project_root = project_root
        .canonicalize()
        .map_err(|error| format!("canonicalize build-gate project root: {error}"))?;
    let scope = crate::discovery::rust_project_harness_scope(
        &project_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let mut inputs = scope.monitored_paths();
    let manifest = project_root.join("Cargo.toml");
    if manifest.is_file() {
        inputs.push(manifest);
    }
    let excluded_package_roots = crate::discovery::cargo_package_exclusion_roots(
        &project_root,
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    );
    let mut owned_files = BTreeMap::new();
    for input in inputs {
        collect_snapshot_path(
            &project_root,
            &input,
            &excluded_package_roots,
            config,
            &mut owned_files,
        )?;
    }
    let files = owned_files.into_values().collect::<Vec<_>>();
    let byte_count = files
        .iter()
        .try_fold(0_u64, |total, file| total.checked_add(file.byte_count))
        .ok_or_else(|| "build-gate snapshot byte count overflow".to_string())?;
    let digest = content_digest(
        &serde_json::to_vec(&files)
            .map_err(|error| format!("serialize build-gate snapshot files: {error}"))?,
    );
    Ok(RustProjectHarnessBuildGateSnapshot {
        digest,
        file_count: files.len(),
        byte_count,
        files,
    })
}

pub(super) fn build_gate_cache_key(
    config: &RustHarnessConfig,
    scope: RustHarnessRunScope,
    dependency_baseline_receipts: &[RustProjectHarnessDependencyBaselinePackageReceipt],
    snapshot: &RustProjectHarnessBuildGateSnapshot,
) -> Result<String, String> {
    let harness_provider_digest = harness_provider_digest()?;
    build_gate_cache_key_with_contract(
        config,
        scope,
        dependency_baseline_receipts,
        snapshot,
        BuildGateCacheContract {
            schema_id: RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_ID,
            schema_version: RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_VERSION,
            harness_version: env!("CARGO_PKG_VERSION"),
            harness_provider_digest: &harness_provider_digest,
        },
    )
}

fn harness_provider_digest() -> Result<String, String> {
    static DIGEST: std::sync::OnceLock<Result<String, String>> = std::sync::OnceLock::new();
    DIGEST
        .get_or_init(|| {
            snapshot_build_gate_inputs(
                Path::new(env!("CARGO_MANIFEST_DIR")),
                &RustHarnessConfig::default(),
            )
            .map(|snapshot| snapshot.digest)
        })
        .clone()
}

struct BuildGateCacheContract<'a> {
    schema_id: &'static str,
    schema_version: &'static str,
    harness_version: &'static str,
    harness_provider_digest: &'a str,
}

fn build_gate_cache_key_with_contract(
    config: &RustHarnessConfig,
    scope: RustHarnessRunScope,
    dependency_baseline_receipts: &[RustProjectHarnessDependencyBaselinePackageReceipt],
    snapshot: &RustProjectHarnessBuildGateSnapshot,
    contract: BuildGateCacheContract<'_>,
) -> Result<String, String> {
    let scope = match scope {
        RustHarnessRunScope::Package => "package",
        RustHarnessRunScope::ProjectWorkspace => "project-workspace",
    };
    let mut dependency_baseline_receipts = dependency_baseline_receipts.to_vec();
    dependency_baseline_receipts.sort_by(|left, right| {
        (
            left.name.as_str(),
            left.version.as_str(),
            left.source_contains.as_str(),
        )
            .cmp(&(
                right.name.as_str(),
                right.version.as_str(),
                right.source_contains.as_str(),
            ))
    });
    let material = RustProjectHarnessBuildGateCacheKey {
        schema_id: contract.schema_id,
        schema_version: contract.schema_version,
        harness_version: contract.harness_version,
        harness_provider_digest: contract.harness_provider_digest,
        scope,
        config,
        dependency_baseline_receipts: &dependency_baseline_receipts,
        content_snapshot_digest: &snapshot.digest,
    };
    serde_json::to_vec(&material)
        .map(|bytes| content_digest(&bytes))
        .map_err(|error| format!("serialize build-gate cache key: {error}"))
}

fn project_cache_root(base: PathBuf, project_identity: &str) -> PathBuf {
    base.join("rph")
        .join("bg")
        .join(format!(
            "v{}",
            RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_VERSION
        ))
        .join(project_identity_stem(project_identity))
}

pub(super) fn load_build_gate_cache(
    cache_root: &Path,
    cache_key: &str,
) -> Option<RustProjectHarnessBuildGateCacheRecord> {
    let bytes = fs::read(cache_path(cache_root, cache_key)).ok()?;
    let record = serde_json::from_slice::<RustProjectHarnessBuildGateCacheRecord>(&bytes).ok()?;
    let byte_count = record
        .snapshot
        .files
        .iter()
        .try_fold(0_u64, |total, file| total.checked_add(file.byte_count))?;
    (record.schema_id == RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_ID
        && record.schema_version == RUST_PROJECT_HARNESS_BUILD_GATE_CACHE_SCHEMA_VERSION
        && record.cache_key == cache_key
        && record.snapshot.file_count == record.snapshot.files.len()
        && record.snapshot.byte_count == byte_count
        && record.snapshot.digest
            == content_digest(&serde_json::to_vec(&record.snapshot.files).ok()?)
        && record.payload_digest
            == build_gate_cache_payload_digest(
                &record.report,
                &record.verification_plan,
                &record.downstream_policy_receipt,
                &record.dependency_baseline_receipts,
            )
            .ok()?)
    .then_some(record)
}

pub(super) fn build_gate_cache_payload_digest(
    report: &RustHarnessReport,
    verification_plan: &RustVerificationPlan,
    downstream_policy_receipt: &RustProjectHarnessDownstreamPolicyReceipt,
    dependency_baseline_receipts: &[RustProjectHarnessDependencyBaselinePackageReceipt],
) -> Result<String, String> {
    serde_json::to_vec(&RustProjectHarnessBuildGateCachePayload {
        report,
        verification_plan,
        downstream_policy_receipt,
        dependency_baseline_receipts,
    })
    .map(|bytes| content_digest(&bytes))
    .map_err(|error| format!("serialize build-gate cache payload: {error}"))
}

pub(super) fn store_build_gate_cache(
    cache_root: &Path,
    record: &RustProjectHarnessBuildGateCacheRecord,
) -> Result<(), String> {
    fs::create_dir_all(cache_root)
        .map_err(|error| format!("create build-gate cache directory: {error}"))?;
    let destination = cache_path(cache_root, &record.cache_key);
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = cache_root.join(format!(".{}.{}.tmp", std::process::id(), sequence));
    let bytes = serde_json::to_vec(record)
        .map_err(|error| format!("serialize build-gate cache record: {error}"))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("create build-gate cache temporary file: {error}"))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("write build-gate cache temporary file: {error}"))?;
    drop(file);
    if let Err(error) = fs::rename(&temporary, &destination) {
        let _ = fs::remove_file(&temporary);
        if !destination.is_file() {
            return Err(format!("publish build-gate cache record: {error}"));
        }
    }
    Ok(())
}

fn collect_snapshot_path(
    project_root: &Path,
    path: &Path,
    excluded_package_roots: &BTreeSet<PathBuf>,
    config: &RustHarnessConfig,
    files: &mut BTreeMap<PathBuf, RustProjectHarnessBuildGateSnapshotFile>,
) -> Result<(), String> {
    if crate::discovery::is_symlink_path(path) {
        return Ok(());
    }
    if path.is_file() {
        let content = fs::read(path).map_err(|error| {
            format!("read build-gate snapshot file {}: {error}", path.display())
        })?;
        let relative_path = path
            .strip_prefix(project_root)
            .map_err(|error| format!("relativize build-gate snapshot path: {error}"))?
            .to_path_buf();
        files.insert(
            relative_path.clone(),
            RustProjectHarnessBuildGateSnapshotFile {
                path: relative_path,
                byte_count: content.len().min(u64::MAX as usize) as u64,
                content_digest: content_digest(&content),
            },
        );
        return Ok(());
    }
    if !path.is_dir() {
        return Ok(());
    }
    if excluded_package_roots.contains(&crate::path::normalize_lexical_path(path)) {
        return Ok(());
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("read build-gate snapshot directory: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read build-gate snapshot entry: {error}"))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if crate::discovery::is_symlink_path(&path) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|error| format!("inspect build-gate snapshot entry: {error}"))?;
        if file_type.is_dir() {
            if should_skip_directory(&entry.file_name(), config) {
                continue;
            }
            collect_snapshot_path(project_root, &path, excluded_package_roots, config, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let content = fs::read(&path).map_err(|error| {
            format!("read build-gate snapshot file {}: {error}", path.display())
        })?;
        let relative_path = path
            .strip_prefix(project_root)
            .map_err(|error| format!("relativize build-gate snapshot path: {error}"))?
            .to_path_buf();
        files.insert(
            relative_path.clone(),
            RustProjectHarnessBuildGateSnapshotFile {
                path: relative_path,
                byte_count: content.len().min(u64::MAX as usize) as u64,
                content_digest: content_digest(&content),
            },
        );
    }
    Ok(())
}

fn should_skip_directory(name: &OsStr, config: &RustHarnessConfig) -> bool {
    let name = name.to_string_lossy();
    config.ignored_dir_names.contains(name.as_ref())
        || (name.starts_with('.') && !config.include_hidden_dir_names.contains(name.as_ref()))
}

fn cache_path(cache_root: &Path, cache_key: &str) -> PathBuf {
    cache_root.join(format!("{}.json", cache_file_stem(cache_key)))
}

fn cache_file_stem(cache_key: &str) -> &str {
    cache_key
        .rsplit_once(':')
        .map(|(_, digest)| digest)
        .unwrap_or(cache_key)
}

fn project_identity_stem(project_identity: &str) -> &str {
    project_identity.get(..32).unwrap_or(project_identity)
}

fn content_digest(content: &[u8]) -> String {
    format!(
        "sha256:{}",
        cache_digest_hex(
            b"rust-lang-project-harness.build-gate-cache.content.v1",
            content,
        )
    )
}

fn cache_digest_hex(namespace: &[u8], content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((namespace.len() as u64).to_be_bytes());
    hasher.update(namespace);
    hasher.update((content.len() as u64).to_be_bytes());
    hasher.update(content);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/build_gate/cache.rs"]
mod tests;
