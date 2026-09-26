//! Scenario benchmark receipt and error types.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;

use super::contract::RustScenarioBenchmarkContract;

/// Scenario manifest format that requires a benchmark contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustScenarioBenchmarkManifestKind {
    /// Native ASP Rust scenario contract stored in `scenario.toml`.
    ScenarioToml,
    /// CLI AST patch scenario contract stored in `scenario.json`.
    AstPatchScenarioJson,
}

impl RustScenarioBenchmarkManifestKind {
    /// Return the stable manifest kind token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ScenarioToml => "scenario.toml",
            Self::AstPatchScenarioJson => "ast-patch-scenario.json",
        }
    }
}

/// One scenario root that must carry a benchmark contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkRequirement {
    /// Scenario root directory.
    pub root: PathBuf,
    /// Manifest kind discovered in this root.
    pub manifest_kind: RustScenarioBenchmarkManifestKind,
}

/// Suite-level receipt proving a policy rule has scenario benchmark coverage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkPolicyCoverage {
    /// Agent policy rule id covered by the scenario.
    pub rule_id: RustScenarioBenchmarkPolicyRuleId,
    /// Scenario id that carries the coverage.
    pub scenario_id: RustScenarioBenchmarkScenarioId,
    /// Policy id declared by the scenario metadata.
    pub policy_id: RustScenarioBenchmarkPolicyId,
    /// Scenario root directory.
    pub root: PathBuf,
}

/// Agent policy rule id covered by a scenario benchmark.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RustScenarioBenchmarkPolicyRuleId(String);

impl RustScenarioBenchmarkPolicyRuleId {
    /// Build a policy rule id.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the raw rule id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Scenario id used to prove agent policy coverage.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RustScenarioBenchmarkScenarioId(String);

impl RustScenarioBenchmarkScenarioId {
    /// Build a scenario id.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the raw scenario id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Policy id declared by scenario metadata.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RustScenarioBenchmarkPolicyId(String);

impl RustScenarioBenchmarkPolicyId {
    /// Build a policy id.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the raw policy id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validation receipt for all required scenario benchmark contracts in a crate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkSuiteReceipt {
    /// Crate root used for scenario discovery.
    pub root: PathBuf,
    /// Required scenario roots discovered from fixture conventions.
    pub requirements: Vec<RustScenarioBenchmarkRequirement>,
    /// Successfully loaded per-scenario benchmark receipts.
    pub receipts: Vec<RustScenarioBenchmarkReceipt>,
    /// Agent policy scenario coverage proven from policy-owned requirements.
    pub policy_coverage: Vec<RustScenarioBenchmarkPolicyCoverage>,
    /// Suite-level contract violations, such as a missing `benchmark.toml`.
    pub violations: Vec<RustScenarioBenchmarkViolation>,
    /// Overall suite validation status.
    pub status: RustScenarioBenchmarkStatus,
}

/// Agent-visible scenario metadata loaded from `scenario.toml`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RustScenarioMetadata {
    /// Stable scenario id used in snapshots and policy receipts.
    pub id: String,
    /// Human-readable scenario title.
    pub title: String,
    /// Policy ids exercised by this scenario.
    #[serde(default)]
    pub policy_ids: Vec<String>,
    /// Agent-facing goal that explains how the scenario should be used.
    pub agent_goal: String,
    /// Reference repositories used to derive this scenario.
    #[serde(default)]
    pub reference_repositories: Vec<String>,
    /// Compact engineering patterns observed in those references.
    #[serde(default)]
    pub reference_patterns: Vec<String>,
    /// Relative input fixture directory.
    pub inputs: String,
    /// Relative expected-output fixture directory.
    pub expected: String,
}

/// Validation receipt for one scenario benchmark fixture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkReceipt {
    /// Scenario root that owns `scenario.toml` and `benchmark.toml`.
    pub root: PathBuf,
    /// Agent-visible scenario metadata.
    pub scenario: RustScenarioMetadata,
    /// Numeric benchmark contract and observations.
    pub benchmark: RustScenarioBenchmarkContract,
    /// Overall validation status.
    pub status: RustScenarioBenchmarkStatus,
    /// Contract, performance, or memory violations.
    pub violations: Vec<RustScenarioBenchmarkViolation>,
}

/// Overall scenario benchmark validation status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustScenarioBenchmarkStatus {
    /// Contract and observed numbers satisfy the gate.
    Pass,
    /// Contract is valid but observed performance or memory exceeds the gate.
    Fail,
    /// Required contract metadata or thresholds are invalid.
    Invalid,
}

impl RustScenarioBenchmarkStatus {
    /// Return the stable lowercase status token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Invalid => "invalid",
        }
    }
}

/// One scenario benchmark validation violation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScenarioBenchmarkViolation {
    /// Violation class.
    pub kind: RustScenarioBenchmarkViolationKind,
    /// Stable field path that failed validation.
    pub field: String,
    /// Agent-facing explanation of the failed condition.
    pub message: String,
}

/// Violation class for scenario benchmark validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustScenarioBenchmarkViolationKind {
    /// Required metadata or threshold contract is invalid.
    Contract,
    /// Observed runtime exceeds the allowed duration gate.
    Performance,
    /// Observed memory exceeds the allowed memory gate.
    Memory,
}

impl RustScenarioBenchmarkViolationKind {
    /// Return the stable lowercase violation kind token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::Performance => "performance",
            Self::Memory => "memory",
        }
    }
}

/// Error returned when a scenario benchmark contract cannot be read or parsed.
#[derive(Debug)]
pub struct RustScenarioBenchmarkError {
    path: PathBuf,
    message: String,
}

impl RustScenarioBenchmarkError {
    pub(super) fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for RustScenarioBenchmarkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.message)
    }
}

impl Error for RustScenarioBenchmarkError {}
