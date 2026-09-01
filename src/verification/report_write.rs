//! Filesystem persistence for modular verification report artifacts.

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::AspRustConfig;

use super::analysis::{
    RustVerificationAnalysisProfile, build_rust_verification_analysis_profile_with_config,
};
use super::model::RustVerificationPlan;
use super::report::{
    RustVerificationReportArtifact, RustVerificationReportBundle, RustVerificationReportSidecar,
    build_rust_verification_report_bundle_with_options,
};
use super::report_artifact::{
    RustVerificationReportArtifactRenderError,
    render_rust_verification_report_artifact_json_with_config,
};
use super::report_manifest::RustVerificationReportManifestSchema;
use super::report_options::{
    RustVerificationReportArtifactRole, RustVerificationReportOptions,
    RustVerificationReportPersistence, RustVerificationReportSidecarRole,
};
use super::report_select::{
    build_rust_verification_report_selection_advice,
    render_rust_verification_report_selection_advice_json,
};

/// Filesystem layout used to persist modular verification reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustVerificationReportWriteConfig {
    /// Project root whose absolute path may appear in rendered artifacts.
    pub project_root: PathBuf,
    /// Source-controlled directory for compact baseline artifacts.
    pub source_baseline_dir: PathBuf,
    /// Runtime cache directory for verbose or machine-local artifacts.
    pub runtime_cache_dir: PathBuf,
    /// Stable placeholder used when compacting `project_root` in JSON output.
    pub project_root_placeholder: String,
}

impl RustVerificationReportWriteConfig {
    /// Build a report write config.
    #[must_use]
    pub fn new(
        project_root: impl Into<PathBuf>,
        source_baseline_dir: impl Into<PathBuf>,
        runtime_cache_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            project_root: project_root.into(),
            source_baseline_dir: source_baseline_dir.into(),
            runtime_cache_dir: runtime_cache_dir.into(),
            project_root_placeholder: "$CRATE_ROOT".to_string(),
        }
    }

    /// Return the default source-controlled report baseline directory.
    #[must_use]
    pub fn recommended_source_baseline_dir(project_root: impl Into<PathBuf>) -> PathBuf {
        project_root.into().join("resources/verification/reports")
    }

    /// Override the placeholder used for project-root path compaction.
    #[must_use]
    pub fn with_project_root_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.project_root_placeholder = placeholder.into();
        self
    }
}

/// Paths written by `write_rust_verification_reports`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportWriteReceipt {
    /// Manifest schema used for all written manifest files.
    pub manifest_schema: RustVerificationReportManifestSchema,
    /// Files written under the source baseline directory.
    pub source_baseline_paths: Vec<PathBuf>,
    /// Files written under the runtime cache directory.
    pub runtime_cache_paths: Vec<PathBuf>,
    /// Structured paths for written report artifacts.
    pub artifact_paths: Vec<RustVerificationReportArtifactWriteReceipt>,
    /// Optional runtime-cache selection advice sidecar.
    pub selection_advice_path: Option<PathBuf>,
    /// Structured paths for written sidecars.
    pub sidecar_paths: Vec<RustVerificationReportSidecarWriteReceipt>,
    /// Agent-facing repair advice when source baselines were configured for a
    /// transient or non-repository path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materialization_advice: Vec<RustVerificationReportMaterializationAdvice>,
}

/// Advice emitted when durable verification reports are configured for an
/// unsuitable filesystem location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportMaterializationAdvice {
    /// Persistence class that needs a durable location.
    pub persistence: RustVerificationReportPersistence,
    /// Configured path that should be moved.
    pub path: PathBuf,
    /// Recommended repository-owned baseline directory.
    pub recommended_dir: PathBuf,
    /// Stable reason code for the advice.
    pub reason: String,
    /// Stable Agent action token.
    pub action: String,
    /// Report artifact keys affected by this advice.
    pub artifact_keys: Vec<String>,
}

/// Filesystem receipt for one persisted report artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportArtifactWriteReceipt {
    /// Stable report contract key.
    pub key: String,
    /// Agent-facing artifact role.
    pub role: RustVerificationReportArtifactRole,
    /// Persistence target used for the artifact.
    pub persistence: RustVerificationReportPersistence,
    /// Persisted artifact path.
    pub path: PathBuf,
}

/// Filesystem receipt for one persisted report sidecar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustVerificationReportSidecarWriteReceipt {
    /// Stable sidecar contract key.
    pub key: String,
    /// Agent-facing sidecar role.
    pub role: RustVerificationReportSidecarRole,
    /// Persisted sidecar path.
    pub path: PathBuf,
}

impl RustVerificationReportWriteReceipt {
    /// Return the source-baseline manifest path from this receipt, if present.
    #[must_use]
    pub fn source_manifest_path(&self) -> Option<&PathBuf> {
        find_manifest_path(&self.source_baseline_paths)
    }

    /// Return the runtime-cache manifest path from this receipt, if present.
    #[must_use]
    pub fn runtime_manifest_path(&self) -> Option<&PathBuf> {
        find_manifest_path(&self.runtime_cache_paths)
    }

    /// Return the persisted path for one report artifact contract key.
    #[must_use]
    pub fn artifact_path(&self, key: &str) -> Option<&PathBuf> {
        self.artifact_paths
            .iter()
            .find(|artifact| artifact.key == key)
            .map(|artifact| &artifact.path)
    }

    /// Return the persisted path for one report sidecar contract key.
    #[must_use]
    pub fn sidecar_path(&self, key: &str) -> Option<&PathBuf> {
        self.sidecar_paths
            .iter()
            .find(|sidecar| sidecar.key == key)
            .map(|sidecar| &sidecar.path)
    }
}

impl RustVerificationReportArtifactWriteReceipt {
    fn from_artifact(artifact: &RustVerificationReportArtifact, path: PathBuf) -> Self {
        Self {
            key: artifact.key.clone(),
            role: artifact.role,
            persistence: artifact.persistence,
            path,
        }
    }
}

/// Error raised while writing modular verification reports.
#[derive(Debug)]
pub enum RustVerificationReportWriteError {
    /// A report artifact could not be rendered.
    Render(RustVerificationReportArtifactRenderError),
    /// A report artifact could not be serialized.
    Json(serde_json::Error),
    /// A filesystem operation failed.
    Io {
        /// Path being created or written when the error occurred.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

impl fmt::Display for RustVerificationReportWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(error) => {
                write!(formatter, "failed to render verification report: {error}")
            }
            Self::Json(error) => write!(formatter, "failed to render verification report: {error}"),
            Self::Io { path, source } => write!(
                formatter,
                "failed to write verification report at {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for RustVerificationReportWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Render(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl From<serde_json::Error> for RustVerificationReportWriteError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RustVerificationReportArtifactRenderError> for RustVerificationReportWriteError {
    fn from(error: RustVerificationReportArtifactRenderError) -> Self {
        Self::Render(error)
    }
}

/// Write modular verification report artifacts using their persistence policy.
///
/// The source baseline manifest contains only source-controlled artifacts. The
/// runtime cache manifest contains the full bundle so local tooling can inspect
/// both source and cache report responsibilities from one machine-local file.
///
/// # Errors
///
/// Returns an error if directories cannot be created, artifacts cannot be
/// serialized, or files cannot be written.
pub fn write_rust_verification_reports(
    plan: &RustVerificationPlan,
    config: &RustVerificationReportWriteConfig,
) -> Result<RustVerificationReportWriteReceipt, RustVerificationReportWriteError> {
    write_rust_verification_reports_with_options(
        plan,
        &AspRustConfig::default(),
        config,
        &RustVerificationReportOptions::default(),
    )
}

/// Write modular verification reports using explicit report options.
///
/// This variant can render opt-in artifacts that need the original harness
/// config, such as the analysis profile.
///
/// # Errors
///
/// Returns an error if directories cannot be created, artifacts cannot be
/// rendered, serialized, or files cannot be written.
pub fn write_rust_verification_reports_with_options(
    plan: &RustVerificationPlan,
    harness_config: &AspRustConfig,
    config: &RustVerificationReportWriteConfig,
    options: &RustVerificationReportOptions,
) -> Result<RustVerificationReportWriteReceipt, RustVerificationReportWriteError> {
    prepare_report_directories(config)?;
    let bundle = build_rust_verification_report_bundle_with_options(plan, options);
    let (source_artifacts, cache_artifacts) = collect_report_artifact_sets(&bundle);
    let (source_sidecars, cache_sidecars) = collect_report_sidecar_sets(&bundle);
    let selection_profile = build_selection_advice_profile(plan, harness_config, options)?;
    let mut receipt = RustVerificationReportWriteReceipt {
        manifest_schema: bundle.schema.clone(),
        materialization_advice: source_baseline_materialization_advice(
            config,
            &source_artifacts,
            &source_sidecars,
        ),
        ..RustVerificationReportWriteReceipt::default()
    };
    let source_bundle = bundle.with_parts(source_artifacts.clone(), source_sidecars);

    write_report_manifest(
        &source_bundle,
        &config.source_baseline_dir,
        config,
        &mut receipt.source_baseline_paths,
    )?;
    write_report_manifest(
        &bundle,
        &config.runtime_cache_dir,
        config,
        &mut receipt.runtime_cache_paths,
    )?;
    write_report_artifacts(
        plan,
        harness_config,
        source_artifacts,
        &config.source_baseline_dir,
        config,
        &mut receipt.source_baseline_paths,
        &mut receipt.artifact_paths,
    )?;
    write_report_artifacts(
        plan,
        harness_config,
        cache_artifacts,
        &config.runtime_cache_dir,
        config,
        &mut receipt.runtime_cache_paths,
        &mut receipt.artifact_paths,
    )?;
    write_report_sidecars(
        &bundle,
        selection_profile.as_ref(),
        cache_sidecars,
        config,
        &mut receipt,
    )?;

    Ok(receipt)
}

/// Render a report write receipt as JSON.
///
/// # Errors
///
/// Returns a serialization error if the receipt cannot be encoded.
pub fn render_rust_verification_report_write_receipt_json(
    receipt: &RustVerificationReportWriteReceipt,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(receipt)
}

/// Render a compact write receipt for Agents.
#[must_use]
pub fn render_rust_verification_report_write_receipt(
    receipt: &RustVerificationReportWriteReceipt,
) -> String {
    let mut rendered = String::new();
    let _ = write!(
        rendered,
        "[verify-report-write] schema={} source_paths={} runtime_paths={} sidecars={}",
        receipt.manifest_schema.compact_label(),
        receipt.source_baseline_paths.len(),
        receipt.runtime_cache_paths.len(),
        receipt.sidecar_paths.len()
    );
    if let Some(path) = receipt.source_manifest_path() {
        let _ = writeln!(rendered);
        let _ = write!(rendered, "   |source_manifest: {}", display_path(path));
    }
    if let Some(path) = receipt.runtime_manifest_path() {
        let _ = writeln!(rendered);
        let _ = write!(rendered, "   |runtime_manifest: {}", display_path(path));
    }
    if let Some(path) = &receipt.selection_advice_path {
        let _ = writeln!(rendered);
        let _ = write!(rendered, "   |selection_advice: {}", display_path(path));
    }
    for sidecar in &receipt.sidecar_paths {
        let _ = writeln!(rendered);
        let _ = write!(
            rendered,
            "   |sidecar: role={} key={} path={}",
            sidecar.role.as_str(),
            sidecar.key,
            display_path(&sidecar.path)
        );
    }
    for advice in &receipt.materialization_advice {
        let _ = writeln!(rendered);
        let _ = write!(
            rendered,
            "   |materialize: {} reason={} action={} path={} recommend={} artifacts={}",
            advice.persistence.as_str(),
            advice.reason,
            advice.action,
            display_path(&advice.path),
            display_path(&advice.recommended_dir),
            advice.artifact_keys.join(",")
        );
    }
    rendered
}

fn build_selection_advice_profile(
    plan: &RustVerificationPlan,
    harness_config: &AspRustConfig,
    options: &RustVerificationReportOptions,
) -> Result<Option<RustVerificationAnalysisProfile>, RustVerificationReportWriteError> {
    if !options.include_selection_advice || !options.include_analysis_profile {
        return Ok(None);
    }
    build_rust_verification_analysis_profile_with_config(&plan.project_root, harness_config)
        .map(Some)
        .map_err(RustVerificationReportArtifactRenderError::Analysis)
        .map_err(Into::into)
}

fn write_report_sidecars(
    bundle: &RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
    sidecars: Vec<RustVerificationReportSidecar>,
    config: &RustVerificationReportWriteConfig,
    receipt: &mut RustVerificationReportWriteReceipt,
) -> Result<(), RustVerificationReportWriteError> {
    for sidecar in sidecars {
        write_report_sidecar(bundle, analysis_profile, &sidecar, config, receipt)?;
    }
    Ok(())
}

fn write_report_sidecar(
    bundle: &RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
    sidecar: &RustVerificationReportSidecar,
    config: &RustVerificationReportWriteConfig,
    receipt: &mut RustVerificationReportWriteReceipt,
) -> Result<(), RustVerificationReportWriteError> {
    let Some(payload) = render_report_sidecar_json(bundle, analysis_profile, sidecar)? else {
        return Ok(());
    };
    let path = config.runtime_cache_dir.join(&sidecar.artifact_name);
    write_json(
        &path,
        &compact_project_root(
            &payload,
            &config.project_root,
            &config.project_root_placeholder,
        ),
    )?;
    if sidecar.role == RustVerificationReportSidecarRole::SelectionAdvice {
        receipt.selection_advice_path = Some(path.clone());
    }
    receipt
        .sidecar_paths
        .push(RustVerificationReportSidecarWriteReceipt {
            key: sidecar.key.clone(),
            role: sidecar.role,
            path: path.clone(),
        });
    receipt.runtime_cache_paths.push(path);
    Ok(())
}

fn render_report_sidecar_json(
    bundle: &RustVerificationReportBundle,
    analysis_profile: Option<&RustVerificationAnalysisProfile>,
    sidecar: &RustVerificationReportSidecar,
) -> Result<Option<String>, RustVerificationReportWriteError> {
    match sidecar.role {
        RustVerificationReportSidecarRole::SelectionAdvice => {
            let advice = build_rust_verification_report_selection_advice(bundle, analysis_profile);
            render_rust_verification_report_selection_advice_json(&advice)
                .map(Some)
                .map_err(Into::into)
        }
        RustVerificationReportSidecarRole::Custom => Ok(None),
    }
}

fn prepare_report_directories(
    config: &RustVerificationReportWriteConfig,
) -> Result<(), RustVerificationReportWriteError> {
    create_dir_all(&config.source_baseline_dir)?;
    create_dir_all(&config.runtime_cache_dir)?;
    Ok(())
}

fn collect_report_artifact_sets(
    bundle: &RustVerificationReportBundle,
) -> (
    Vec<RustVerificationReportArtifact>,
    Vec<RustVerificationReportArtifact>,
) {
    let source_artifacts = bundle
        .source_baseline_artifacts()
        .into_iter()
        .cloned()
        .collect();
    let cache_artifacts = bundle
        .runtime_cache_artifacts()
        .into_iter()
        .cloned()
        .collect();
    (source_artifacts, cache_artifacts)
}

fn collect_report_sidecar_sets(
    bundle: &RustVerificationReportBundle,
) -> (
    Vec<RustVerificationReportSidecar>,
    Vec<RustVerificationReportSidecar>,
) {
    let source_sidecars = bundle
        .source_baseline_sidecars()
        .into_iter()
        .cloned()
        .collect();
    let cache_sidecars = bundle
        .runtime_cache_sidecars()
        .into_iter()
        .cloned()
        .collect();
    (source_sidecars, cache_sidecars)
}

fn source_baseline_materialization_advice(
    config: &RustVerificationReportWriteConfig,
    source_artifacts: &[RustVerificationReportArtifact],
    source_sidecars: &[RustVerificationReportSidecar],
) -> Vec<RustVerificationReportMaterializationAdvice> {
    if source_artifacts.is_empty() && source_sidecars.is_empty() {
        return Vec::new();
    }
    let Some(reason) = source_baseline_dir_issue(config) else {
        return Vec::new();
    };
    vec![RustVerificationReportMaterializationAdvice {
        persistence: RustVerificationReportPersistence::SourceBaseline,
        path: config.source_baseline_dir.clone(),
        recommended_dir: RustVerificationReportWriteConfig::recommended_source_baseline_dir(
            &config.project_root,
        ),
        reason: reason.to_string(),
        action: "move_source_baseline_reports_to_repo".to_string(),
        artifact_keys: source_baseline_artifact_keys(source_artifacts, source_sidecars),
    }]
}

fn source_baseline_artifact_keys(
    source_artifacts: &[RustVerificationReportArtifact],
    source_sidecars: &[RustVerificationReportSidecar],
) -> Vec<String> {
    source_artifacts
        .iter()
        .map(|artifact| artifact.key.clone())
        .chain(source_sidecars.iter().map(|sidecar| sidecar.key.clone()))
        .collect()
}

fn source_baseline_dir_issue(config: &RustVerificationReportWriteConfig) -> Option<&'static str> {
    let source_dir = &config.source_baseline_dir;
    if source_dir.as_os_str().is_empty() {
        return Some("source_baseline_dir_empty");
    }
    if !source_dir.is_absolute() {
        return Some("source_baseline_dir_not_project_anchored");
    }
    if path_same_or_under(source_dir, &config.runtime_cache_dir) {
        return Some("source_baseline_dir_under_runtime_cache");
    }
    if !path_same_or_under(source_dir, &config.project_root) {
        return Some("source_baseline_dir_outside_project_root");
    }
    if source_dir_is_under_transient_project_dir(source_dir, &config.project_root) {
        return Some("source_baseline_dir_under_transient_project_dir");
    }
    None
}

fn path_same_or_under(path: &Path, root: &Path) -> bool {
    !root.as_os_str().is_empty() && (path == root || path.starts_with(root))
}

fn source_dir_is_under_transient_project_dir(source_dir: &Path, project_root: &Path) -> bool {
    let Ok(relative) = source_dir.strip_prefix(project_root) else {
        return false;
    };
    relative
        .components()
        .next()
        .is_some_and(|component| transient_project_dir(component.as_os_str().to_string_lossy()))
}

fn transient_project_dir(component: std::borrow::Cow<'_, str>) -> bool {
    matches!(
        component.as_ref(),
        ".cache" | ".run" | "target" | "tmp" | "temp"
    )
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn write_report_manifest(
    bundle: &RustVerificationReportBundle,
    directory: &Path,
    config: &RustVerificationReportWriteConfig,
    paths: &mut Vec<PathBuf>,
) -> Result<(), RustVerificationReportWriteError> {
    let manifest = directory.join("verification_report_manifest.json");
    write_json(
        &manifest,
        &compact_project_root(
            &serde_json::to_string(bundle)?,
            &config.project_root,
            &config.project_root_placeholder,
        ),
    )?;
    paths.push(manifest);
    Ok(())
}

fn write_report_artifacts(
    plan: &RustVerificationPlan,
    harness_config: &AspRustConfig,
    artifacts: Vec<RustVerificationReportArtifact>,
    directory: &Path,
    config: &RustVerificationReportWriteConfig,
    paths: &mut Vec<PathBuf>,
    artifact_paths: &mut Vec<RustVerificationReportArtifactWriteReceipt>,
) -> Result<(), RustVerificationReportWriteError> {
    for artifact in artifacts {
        if let Some(path) = write_artifact(plan, harness_config, &artifact, directory, config)? {
            paths.push(path.clone());
            artifact_paths.push(RustVerificationReportArtifactWriteReceipt::from_artifact(
                &artifact, path,
            ));
        }
    }
    Ok(())
}

fn write_artifact(
    plan: &RustVerificationPlan,
    harness_config: &AspRustConfig,
    artifact: &RustVerificationReportArtifact,
    directory: &Path,
    config: &RustVerificationReportWriteConfig,
) -> Result<Option<PathBuf>, RustVerificationReportWriteError> {
    let Some(payload) = render_rust_verification_report_artifact_json_with_config(
        plan,
        harness_config,
        &artifact.key,
    )?
    else {
        return Ok(None);
    };
    let path = directory.join(&artifact.artifact_name);
    write_json(
        &path,
        &compact_project_root(
            &payload,
            &config.project_root,
            &config.project_root_placeholder,
        ),
    )?;
    Ok(Some(path))
}

fn create_dir_all(path: &Path) -> Result<(), RustVerificationReportWriteError> {
    std::fs::create_dir_all(path).map_err(|source| RustVerificationReportWriteError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_json(path: &Path, payload: &str) -> Result<(), RustVerificationReportWriteError> {
    std::fs::write(path, payload).map_err(|source| RustVerificationReportWriteError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn compact_project_root(payload: &str, project_root: &Path, placeholder: &str) -> String {
    let root = project_root.to_string_lossy();
    if root.is_empty() {
        return payload.to_string();
    }

    let replacement = json_string_fragment(placeholder);
    let mut compacted = payload.to_string();
    for candidate in project_root_compaction_candidates(root.as_ref()) {
        compacted = compacted.replace(&candidate, &replacement);
    }
    compacted
}

fn project_root_compaction_candidates(root: &str) -> BTreeSet<String> {
    let normalized = root.replace('\\', "/");
    [root, normalized.as_str()]
        .into_iter()
        .flat_map(|candidate| [candidate.to_string(), json_string_fragment(candidate)])
        .filter(|candidate| !candidate.is_empty())
        .collect()
}

fn json_string_fragment(value: &str) -> String {
    serde_json::to_string(value)
        .ok()
        .and_then(|encoded| {
            encoded
                .strip_prefix('"')
                .and_then(|trimmed| trimmed.strip_suffix('"'))
                .map(str::to_string)
        })
        .unwrap_or_else(|| value.to_string())
}

fn find_manifest_path(paths: &[PathBuf]) -> Option<&PathBuf> {
    paths.iter().find(|path| {
        path.file_name()
            .is_some_and(|name| name == "verification_report_manifest.json")
    })
}
