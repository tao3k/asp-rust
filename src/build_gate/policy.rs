//! Downstream and workspace build-gate policy configuration.

use std::path::{Path, PathBuf};

use crate::model::AspRustConfig;

use super::AspRustDependencyBaseline;

/// Explicit authority supplied by a downstream build-support package.
///
/// The harness deliberately does not infer ASP state, Cargo output, or policy
/// registry locations. The downstream owner chooses the cache lifecycle and
/// binds the declarative policy digest used by the cache key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AspRustBuildGateAuthority {
    cache_root: PathBuf,
    policy_digest: String,
}

impl AspRustBuildGateAuthority {
    /// Bind one downstream policy to an explicit cache owner.
    ///
    /// # Errors
    ///
    /// Returns an error when the cache root is empty or the policy digest is
    /// not a content-addressed identity.
    pub fn new(
        cache_root: impl Into<PathBuf>,
        policy_digest: impl Into<String>,
    ) -> Result<Self, String> {
        let cache_root = cache_root.into();
        let policy_digest = policy_digest.into();
        if cache_root.as_os_str().is_empty() {
            return Err("build-gate cache root must not be empty".to_string());
        }
        if !policy_digest.starts_with("blake3-256:") || policy_digest.len() <= "blake3-256:".len() {
            return Err("build-gate policy digest must use blake3-256 identity".to_string());
        }
        Ok(Self {
            cache_root,
            policy_digest,
        })
    }

    /// Return the downstream-owned cache root.
    #[must_use]
    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    /// Return the declarative policy identity.
    #[must_use]
    pub fn policy_digest(&self) -> &str {
        &self.policy_digest
    }
}

/// Downstream crate-owned policy consumed by a thin `build.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AspRustDownstreamPolicy {
    gate_label: String,
    config: AspRustConfig,
    dependency_baseline: Option<AspRustDependencyBaseline>,
}

impl AspRustDownstreamPolicy {
    /// Create a downstream policy around a complete harness config.
    /// Return the build-gate label.
    #[must_use]
    pub fn new(gate_label: impl Into<String>, config: AspRustConfig) -> Self {
        Self {
            gate_label: gate_label.into(),
            config,
            dependency_baseline: None,
        }
    }

    /// Return the harness config.
    #[must_use]
    pub fn gate_label(&self) -> &str {
        &self.gate_label
    }

    /// Attach an exact dependency baseline.
    #[must_use]
    pub fn config(&self) -> &AspRustConfig {
        &self.config
    }

    /// Return the optional dependency baseline.
    #[must_use]
    pub fn with_dependency_baseline(
        mut self,
        dependency_baseline: AspRustDependencyBaseline,
    ) -> Self {
        self.dependency_baseline = Some(dependency_baseline);
        self
    }

    #[must_use]
    pub fn dependency_baseline(&self) -> Option<&AspRustDependencyBaseline> {
        self.dependency_baseline.as_ref()
    }
}

/// Workspace-owned policy baseline shared by multiple downstream crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AspRustWorkspacePolicy {
    workspace_label: String,
    config: AspRustConfig,
    dependency_baseline: Option<AspRustDependencyBaseline>,
}

impl AspRustWorkspacePolicy {
    /// Create a workspace policy around a shared harness config.
    /// Return the workspace label.
    #[must_use]
    pub fn new(workspace_label: impl Into<String>, config: AspRustConfig) -> Self {
        Self {
            workspace_label: workspace_label.into(),
            config,
            dependency_baseline: None,
        }
    }

    /// Return the shared harness config.
    #[must_use]
    pub fn workspace_label(&self) -> &str {
        &self.workspace_label
    }

    /// Attach the dependency baseline inherited by member policies.
    #[must_use]
    pub fn config(&self) -> &AspRustConfig {
        &self.config
    }

    /// Return the optional shared dependency baseline.
    #[must_use]
    pub fn with_dependency_baseline(
        mut self,
        dependency_baseline: AspRustDependencyBaseline,
    ) -> Self {
        self.dependency_baseline = Some(dependency_baseline);
        self
    }

    /// Derive a member policy from the shared config.
    #[must_use]
    pub fn dependency_baseline(&self) -> Option<&AspRustDependencyBaseline> {
        self.dependency_baseline.as_ref()
    }

    /// Derive a member policy after applying a config transformation.
    #[must_use]
    pub fn member_crate(&self, crate_label: impl Into<String>) -> AspRustDownstreamPolicy {
        self.attach_dependency_baseline(AspRustDownstreamPolicy::new(
            self.member_gate_label(crate_label),
            self.config.clone(),
        ))
    }

    #[must_use]
    pub fn member_crate_with_config<F>(
        &self,
        crate_label: impl Into<String>,
        configure: F,
    ) -> AspRustDownstreamPolicy
    where
        F: FnOnce(AspRustConfig) -> AspRustConfig,
    {
        self.attach_dependency_baseline(AspRustDownstreamPolicy::new(
            self.member_gate_label(crate_label),
            configure(self.config.clone()),
        ))
    }

    fn member_gate_label(&self, crate_label: impl Into<String>) -> String {
        format!("{}::{}", self.workspace_label, crate_label.into())
    }

    fn attach_dependency_baseline(
        &self,
        policy: AspRustDownstreamPolicy,
    ) -> AspRustDownstreamPolicy {
        match self.dependency_baseline.clone() {
            Some(dependency_baseline) => policy.with_dependency_baseline(dependency_baseline),
            None => policy,
        }
    }
}
