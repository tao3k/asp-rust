//! Verification report artifact renderers.

use std::fmt;

use crate::model::AspRustConfig;

use super::analysis::{
    build_rust_verification_analysis_profile_with_config,
    render_rust_verification_analysis_profile_json,
};
use super::model::RustVerificationPlan;
use super::performance::build_rust_verification_performance_index;
use super::report_options::{ANALYSIS_PROFILE_ARTIFACT_KEY, STABILITY_PICTURE_ARTIFACT_KEY};
use super::stability::build_rust_verification_stability_index;
use super::stability_picture::{
    build_rust_verification_stability_picture_with_policy,
    render_rust_verification_stability_picture_json,
};
use super::task_index::build_rust_verification_task_index;

/// Error raised while rendering one modular verification report artifact.
#[derive(Debug)]
pub enum RustVerificationReportArtifactRenderError {
    /// A report artifact could not be serialized.
    Json(serde_json::Error),
    /// Analysis profile construction failed before serialization.
    Analysis(String),
}

impl fmt::Display for RustVerificationReportArtifactRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Analysis(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for RustVerificationReportArtifactRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Analysis(_) => None,
        }
    }
}

impl From<serde_json::Error> for RustVerificationReportArtifactRenderError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// Render one modular verification report artifact by contract key.
///
/// # Errors
///
/// Returns a serialization error if the selected artifact cannot be encoded as
/// JSON.
pub fn render_rust_verification_report_artifact_json(
    plan: &RustVerificationPlan,
    key: &str,
) -> Result<Option<String>, serde_json::Error> {
    match key {
        "verification_plan_json" => serde_json::to_string(plan).map(Some),
        "task_index_json" => {
            serde_json::to_string(&build_rust_verification_task_index(plan)).map(Some)
        }
        "performance_index_json" => {
            serde_json::to_string(&build_rust_verification_performance_index(plan)).map(Some)
        }
        "stability_index_json" => {
            serde_json::to_string(&build_rust_verification_stability_index(plan)).map(Some)
        }
        _ => Ok(None),
    }
}

/// Render one modular verification report artifact with access to harness config.
///
/// # Errors
///
/// Returns an error if the selected artifact cannot be produced or serialized.
pub fn render_rust_verification_report_artifact_json_with_config(
    plan: &RustVerificationPlan,
    harness_config: &AspRustConfig,
    key: &str,
) -> Result<Option<String>, RustVerificationReportArtifactRenderError> {
    if key == ANALYSIS_PROFILE_ARTIFACT_KEY {
        let profile = build_rust_verification_analysis_profile_with_config(
            &plan.project_root,
            harness_config,
        )
        .map_err(RustVerificationReportArtifactRenderError::Analysis)?;
        return render_rust_verification_analysis_profile_json(&profile)
            .map(Some)
            .map_err(Into::into);
    }
    if key == STABILITY_PICTURE_ARTIFACT_KEY {
        let config = harness_config
            .verification_policy
            .stability_picture
            .clone()
            .unwrap_or_default();
        let picture = build_rust_verification_stability_picture_with_policy(
            plan,
            &harness_config.verification_policy,
            &config,
        );
        return render_rust_verification_stability_picture_json(&picture)
            .map(Some)
            .map_err(Into::into);
    }
    render_rust_verification_report_artifact_json(plan, key).map_err(Into::into)
}
