//! Shared report model for ASP Rust runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::verification::{
    RustOwnerResponsibility, RustVerificationApiPathBaseline, RustVerificationDependencySignal,
    RustVerificationPolicy, RustVerificationProfileHint, RustVerificationReceipt,
    RustVerificationSkillBinding, RustVerificationSkillDescriptor,
    RustVerificationStabilityPictureConfig, RustVerificationTaskContract, RustVerificationTaskKind,
    RustVerificationWaiver,
};

/// Finding severity used by the ASP Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RustDiagnosticSeverity {
    /// Non-blocking repair advice.
    Info,
    /// Blocking policy drift by default.
    Warning,
    /// Blocking syntax or structural failure.
    Error,
}

impl RustDiagnosticSeverity {
    /// Return a stable lowercase severity label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for RustDiagnosticSeverity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Source location for one finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// File path, when the finding is file-backed.
    pub path: Option<PathBuf>,
    /// One-based line number.
    pub line: usize,
    /// Zero-based column number.
    pub column: usize,
}

impl SourceLocation {
    /// Create a source location.
    #[must_use]
    pub fn new(path: Option<PathBuf>, line: usize, column: usize) -> Self {
        Self { path, line, column }
    }
}

/// Stable metadata for one rule pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RulePackDescriptor {
    /// Stable pack id.
    pub id: &'static str,
    /// Rule pack version.
    pub version: &'static str,
    /// Searchable domains for this pack.
    pub domains: &'static [&'static str],
    /// Default execution mode.
    pub default_mode: &'static str,
}

/// Built-in rule packs that can be configured as a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RustRulePack {
    /// Syntax parsing rules.
    Syntax,
    /// Project-level harness and test-layout policy.
    ProjectPolicy,
    /// Rust source modularity and ownership policy.
    Modularity,
    /// Non-blocking repair advice for agents.
    AgentPolicy,
}

impl RustRulePack {
    /// Return the stable rule-pack id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Syntax => "rust.syntax",
            Self::ProjectPolicy => "rust.project_policy",
            Self::Modularity => "rust.modularity",
            Self::AgentPolicy => "rust.agent_policy",
        }
    }

    /// Return all stable rule ids owned by this built-in pack.
    #[must_use]
    pub const fn rule_ids(self) -> &'static [&'static str] {
        match self {
            Self::Syntax => &["RUST-SYN-R001"],
            Self::ProjectPolicy => &[
                "RUST-AGENT-PROJECT-001",
                "RUST-AGENT-PROJECT-002",
                "RUST-AGENT-PROJECT-003",
                "RUST-AGENT-PROJECT-004",
                "RUST-AGENT-PROJECT-005",
                "RUST-AGENT-PROJECT-006",
                "RUST-AGENT-PROJECT-007",
                "RUST-AGENT-PROJECT-008",
                "RUST-AGENT-PROJECT-009",
                "RUST-AGENT-PROJECT-010",
                "RUST-AGENT-PROJECT-011",
                "RUST-AGENT-PROJECT-012",
                "RUST-AGENT-PROJECT-013",
                "RUST-AGENT-PROJECT-014",
                "RUST-AGENT-PROJECT-015",
                "RUST-AGENT-PROJECT-016",
            ],
            Self::Modularity => &[
                "RUST-MOD-R001",
                "RUST-MOD-R002",
                "RUST-MOD-R003",
                "RUST-MOD-R004",
                "RUST-MOD-R005",
                "RUST-MOD-R006",
                "RUST-MOD-R007",
                "RUST-MOD-R008",
                "RUST-MOD-R009",
                "RUST-MOD-R010",
                "RUST-MOD-R011",
            ],
            Self::AgentPolicy => &[
                "RUST-AGENT-DOCS-MODULE-001",
                "RUST-AGENT-DOCS-PUBLIC-002",
                "RUST-AGENT-SOURCE-NAMESPACE-003",
                "RUST-AGENT-API-NAME-004",
                "RUST-AGENT-API-FACADE-005",
                "RUST-AGENT-SOURCE-MODULE-006",
                "RUST-AGENT-SOURCE-PATH-007",
                "RUST-AGENT-DOCS-BRANCH-008",
                "RUST-AGENT-OWNER-GRAPH-009",
                "RUST-AGENT-OWNER-BOUNDARY-010",
                "RUST-AGENT-DOCS-OWNER-FANOUT-011",
                "RUST-AGENT-API-TYPE-012",
                "RUST-AGENT-API-ERROR-013",
                "RUST-AGENT-TEST-SUPPORT-014",
                "RUST-AGENT-CFG-PUBLIC-015",
                "RUST-AGENT-CFG-PUBLIC-016",
                "RUST-AGENT-ITER-PUBLIC-017",
                "RUST-AGENT-API-FLAGS-018",
                "RUST-AGENT-API-PARAMETERS-019",
                "RUST-AGENT-DATA-FIELD-020",
                "RUST-AGENT-DATA-ENUM-PAYLOAD-021",
                "RUST-AGENT-DATA-BOUNDS-022",
                "RUST-AGENT-API-SHAPE-023",
                "RUST-AGENT-DATA-ENUM-TUPLE-024",
                "RUST-AGENT-CFG-IMPL-025",
                "RUST-AGENT-ITER-IMPL-026",
                "RUST-AGENT-API-TYPE-ALIAS-027",
                "RUST-AGENT-DATA-STATE-028",
            ],
        }
    }
}

/// Compact rule metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AspRustRule {
    /// Stable rule id.
    pub rule_id: &'static str,
    /// Stable pack id.
    pub pack_id: &'static str,
    /// Rule severity.
    pub severity: RustDiagnosticSeverity,
    /// Short display title.
    pub title: &'static str,
    /// Precise requirement line.
    pub requirement: &'static str,
    /// Small labels for tooling.
    pub labels: BTreeMap<&'static str, &'static str>,
}

impl AspRustRule {
    /// Build one rule catalog entry.
    #[must_use]
    pub(crate) fn new(
        rule_id: &'static str,
        pack_id: &'static str,
        severity: RustDiagnosticSeverity,
        title: &'static str,
        requirement: &'static str,
        labels: BTreeMap<&'static str, &'static str>,
    ) -> Self {
        Self {
            rule_id,
            pack_id,
            severity,
            title,
            requirement,
            labels,
        }
    }
}

/// One deterministic harness finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspRustFinding {
    /// Stable rule id.
    pub rule_id: String,
    /// Stable pack id.
    pub pack_id: String,
    /// Finding severity.
    pub severity: RustDiagnosticSeverity,
    /// Short title.
    pub title: String,
    /// Concrete finding summary.
    pub summary: String,
    /// Source location.
    pub location: SourceLocation,
    /// Required repair contract.
    pub requirement: String,
    /// Source line at the location, when available.
    pub source_line: Option<String>,
    /// Short pointer label.
    pub label: String,
    /// Small labels for tooling.
    pub labels: BTreeMap<String, String>,
}

/// Stable invariant candidate id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustInvariantId(pub String);

impl RustInvariantId {
    /// Return the id as a borrowed string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Source rule id for an invariant candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustInvariantSourceRuleId(pub String);

impl RustInvariantSourceRuleId {
    /// Return the rule id as a borrowed string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Rule pack id for an invariant candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustInvariantRulePackId(pub String);

impl RustInvariantRulePackId {
    /// Return the pack id as a borrowed string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Machine-facing candidate invariant derived from parser-owned findings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustInvariantCandidate {
    /// Stable candidate id.
    pub invariant_id: RustInvariantId,
    /// Source finding rule id.
    pub source_rule_id: RustInvariantSourceRuleId,
    /// Owning rule pack id.
    pub rule_pack_id: RustInvariantRulePackId,
    /// Candidate invariant kind.
    pub kind: RustInvariantKind,
    /// Candidate lifecycle status.
    pub status: RustInvariantCandidateStatus,
    /// Finding severity after policy configuration.
    pub severity: RustDiagnosticSeverity,
    /// Short title.
    pub title: String,
    /// Machine-readable invariant hypothesis.
    pub hypothesis: String,
    /// Concrete source location.
    pub location: SourceLocation,
    /// Evidence used to raise this candidate.
    pub evidence: Vec<RustInvariantEvidence>,
    /// Receipts expected before this candidate is accepted as verified.
    pub required_receipts: Vec<RustInvariantReceiptKind>,
    /// Proof surfaces that may discharge the candidate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proof_targets: Vec<RustInvariantKind>,
    /// Small labels for tooling.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Candidate invariant category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustInvariantKind {
    /// Public identifier parameter is still represented as a primitive.
    PrimitiveIdentifierBoundary,
    /// Public data shape exposes several semantic primitive fields.
    PublicDataPrimitiveFields,
    /// Public API exposes anonymous tuple payloads.
    AnonymousTupleApiSurface,
    /// Public semantic alias uses a primitive carrier.
    PrimitiveTypeAliasBoundary,
    /// Public data shape exposes stringly state.
    StringlyStateBoundary,
    /// Parser-owned fact invariant.
    ParserFact,
    /// Public API shape invariant.
    PublicApiShape,
    /// Module reasoning tree invariant.
    ModuleReasoningTree,
    /// Dependency graph acyclicity invariant.
    DependencyGraphAcyclicity,
    /// Provider-owned custom invariant kind.
    Custom,
}

/// Candidate invariant lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustInvariantCandidateStatus {
    /// Candidate raised from current parser-owned evidence.
    Candidate,
    /// Candidate accepted as a review/test/proof obligation.
    Accepted,
    /// Candidate discharged by receipts or proof.
    Verified,
    /// Candidate waived with an explicit review reason.
    Waived,
    /// Candidate refers to outdated evidence.
    Stale,
}

/// Receipt family expected for an invariant candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustInvariantReceiptKind {
    /// `cargo check` evidence.
    CargoCheck,
    /// `cargo test` evidence.
    CargoTest,
    /// `cargo clippy` evidence.
    Clippy,
    /// `expect_test` or equivalent behavior snapshot evidence.
    ExpectTest,
    /// `proptest` evidence.
    Proptest,
    /// `cargo fuzz` evidence.
    CargoFuzz,
    /// Kani proof receipt.
    Kani,
    /// Creusot proof receipt.
    Creusot,
    /// Verus proof receipt.
    Verus,
    /// Explicit waiver evidence.
    Waiver,
}

/// Evidence kind attached to an invariant candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustInvariantEvidenceKind {
    /// Evidence came from a harness finding.
    Finding,
    /// Evidence came from parser-owned syntax facts.
    ParserFact,
    /// Evidence came from a verification receipt.
    Receipt,
    /// Evidence came from proof output.
    Proof,
    /// Provider-owned custom evidence kind.
    Custom,
}

/// Evidence item attached to an invariant candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustInvariantEvidence {
    /// Evidence kind.
    pub kind: RustInvariantEvidenceKind,
    /// Compact evidence summary.
    pub summary: String,
    /// Evidence source location, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
    /// Extra machine-readable evidence facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

impl AspRustFinding {
    /// Build a finding from a catalog rule.
    #[must_use]
    pub(crate) fn from_rule(
        rule: &AspRustRule,
        summary: impl Into<String>,
        location: SourceLocation,
        source_line: Option<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule.rule_id.to_string(),
            pack_id: rule.pack_id.to_string(),
            severity: rule.severity,
            title: rule.title.to_string(),
            summary: summary.into(),
            location,
            requirement: rule.requirement.to_string(),
            source_line,
            label: label.into(),
            labels: rule
                .labels
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        }
    }
}

/// Public summary for one parsed Rust source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustModuleReport {
    /// Source path.
    pub path: PathBuf,
    /// Whether `syn` parsed the file successfully.
    pub is_valid: bool,
    /// Syntax error when parsing failed.
    pub parse_error: Option<String>,
}

/// Conventional project paths scanned by the harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspRustScope {
    /// Project root.
    pub project_root: PathBuf,
    /// Source roots such as `src`.
    pub source_paths: Vec<PathBuf>,
    /// Test roots such as `tests`.
    pub test_paths: Vec<PathBuf>,
    /// Package-level Rust target paths such as `build.rs`, `examples`, and `benches`.
    #[serde(default)]
    pub package_paths: Vec<PathBuf>,
    /// Fallback roots used when no conventional roots exist.
    pub fallback_paths: Vec<PathBuf>,
}

impl AspRustScope {
    /// Return the concrete roots scanned by the parser.
    #[must_use]
    pub fn monitored_paths(&self) -> Vec<PathBuf> {
        let mut selected = Vec::new();
        selected.extend(self.source_paths.iter().cloned());
        selected.extend(self.test_paths.iter().cloned());
        selected.extend(self.package_paths.iter().cloned());
        if selected.is_empty() {
            return self.fallback_paths.clone();
        }
        selected
    }
}

/// Configuration for a ASP Rust run.
///
/// The default configuration covers Rust files under conventional `src/`,
/// `tests/`, `examples/`, and `benches/` roots, plus package entrypoint files
/// such as `build.rs`, for package-level harness runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspRustConfig {
    /// Directory names skipped during file discovery.
    pub ignored_dir_names: BTreeSet<String>,
    /// Hidden directory names that discovery may enter.
    #[serde(default)]
    pub include_hidden_dir_names: BTreeSet<String>,
    /// Severities that block assertions.
    pub blocking_severities: BTreeSet<RustDiagnosticSeverity>,
    /// Rule ids that should not emit findings for this run.
    #[serde(default)]
    pub disabled_rules: BTreeSet<String>,
    /// Per-rule severity overrides applied after rule evaluation.
    #[serde(default)]
    pub rule_severity_overrides: BTreeMap<String, RustDiagnosticSeverity>,
    /// Whether project runs include conventional test roots.
    pub include_tests: bool,
    /// Source paths, relative to the project root.
    pub source_dir_names: Vec<String>,
    /// Test paths, relative to the project root.
    pub test_dir_names: Vec<String>,
    /// Required explanations for custom source paths.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub source_path_explanations: BTreeMap<String, String>,
    /// Required explanations for custom test paths.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub test_path_explanations: BTreeMap<String, String>,
    /// Required explanations for excluding default source paths.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub source_path_exclusion_explanations: BTreeMap<String, String>,
    /// Required explanations for excluding default test paths.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub test_path_exclusion_explanations: BTreeMap<String, String>,
    /// Transitional explanation for allowing harness agent advice to pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_advice_allow_explanation: Option<String>,
    /// Required explanation for allowing cargo-check harness advice to pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_check_advice_allow_explanation: Option<String>,
    /// Required explanation for allowing retired cargo-test harness advice to pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_test_advice_allow_explanation: Option<String>,
    /// Library-first verification policy used to plan external skill tasks.
    #[serde(default, skip_serializing_if = "RustVerificationPolicy::is_empty")]
    pub verification_policy: RustVerificationPolicy,
}

impl Default for AspRustConfig {
    fn default() -> Self {
        Self {
            ignored_dir_names: crate::discovery::DEFAULT_IGNORED_DIR_NAMES
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            include_hidden_dir_names: BTreeSet::new(),
            blocking_severities: BTreeSet::from([
                RustDiagnosticSeverity::Warning,
                RustDiagnosticSeverity::Error,
            ]),
            disabled_rules: BTreeSet::new(),
            rule_severity_overrides: BTreeMap::new(),
            include_tests: true,
            source_dir_names: vec!["src".to_string()],
            test_dir_names: vec!["tests".to_string()],
            source_path_explanations: BTreeMap::new(),
            test_path_explanations: BTreeMap::new(),
            source_path_exclusion_explanations: BTreeMap::new(),
            test_path_exclusion_explanations: BTreeMap::new(),
            agent_advice_allow_explanation: None,
            cargo_check_advice_allow_explanation: None,
            cargo_test_advice_allow_explanation: None,
            verification_policy: RustVerificationPolicy::default(),
        }
    }
}

impl AspRustConfig {
    /// Return a config with one rule disabled.
    #[must_use]
    pub fn with_disabled_rule(mut self, rule_id: impl Into<String>) -> Self {
        self.disabled_rules.insert(rule_id.into());
        self
    }

    /// Return a config with several rules disabled.
    #[must_use]
    pub fn with_disabled_rules<I, S>(mut self, rule_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.disabled_rules
            .extend(rule_ids.into_iter().map(Into::into));
        self
    }

    /// Return a config with every rule in one built-in pack disabled.
    #[must_use]
    pub fn with_disabled_rule_pack(mut self, rule_pack: RustRulePack) -> Self {
        self.disabled_rules.extend(
            rule_pack
                .rule_ids()
                .iter()
                .map(|rule_id| (*rule_id).to_string()),
        );
        self
    }

    /// Return a config with one rule severity overridden.
    #[must_use]
    pub fn with_rule_severity(
        mut self,
        rule_id: impl Into<String>,
        severity: RustDiagnosticSeverity,
    ) -> Self {
        self.rule_severity_overrides
            .insert(rule_id.into(), severity);
        self
    }

    /// Return a config with every rule in one built-in pack assigned a severity.
    #[must_use]
    pub fn with_rule_pack_severity(
        mut self,
        rule_pack: RustRulePack,
        severity: RustDiagnosticSeverity,
    ) -> Self {
        self.rule_severity_overrides.extend(
            rule_pack
                .rule_ids()
                .iter()
                .map(|rule_id| ((*rule_id).to_string(), severity)),
        );
        self
    }

    /// Return a config with explicit blocking severities.
    #[must_use]
    pub fn with_blocking_severities<I>(mut self, severities: I) -> Self
    where
        I: IntoIterator<Item = RustDiagnosticSeverity>,
    {
        self.blocking_severities = severities.into_iter().collect();
        self
    }

    /// Return a config with one custom source path and its required explanation.
    #[must_use]
    pub fn with_source_path(
        mut self,
        path: impl Into<String>,
        explanation: impl Into<String>,
    ) -> Self {
        let path = normalize_config_scope_path(path.into());
        push_unique_scope_path(&mut self.source_dir_names, path.clone());
        self.source_path_explanations
            .insert(path, explanation.into());
        self
    }

    /// Return a config with one custom test path and its required explanation.
    #[must_use]
    pub fn with_test_path(
        mut self,
        path: impl Into<String>,
        explanation: impl Into<String>,
    ) -> Self {
        let path = normalize_config_scope_path(path.into());
        push_unique_scope_path(&mut self.test_dir_names, path.clone());
        self.test_path_explanations.insert(path, explanation.into());
        self
    }

    /// Return a config with one default source path excluded and explained.
    #[must_use]
    pub fn with_source_path_excluded(
        mut self,
        path: impl Into<String>,
        explanation: impl Into<String>,
    ) -> Self {
        let path = normalize_config_scope_path(path.into());
        retain_scope_paths_except(&mut self.source_dir_names, &path);
        self.source_path_exclusion_explanations
            .insert(path, explanation.into());
        self
    }

    /// Return a config with one default test path excluded and explained.
    #[must_use]
    pub fn with_test_path_excluded(
        mut self,
        path: impl Into<String>,
        explanation: impl Into<String>,
    ) -> Self {
        let path = normalize_config_scope_path(path.into());
        retain_scope_paths_except(&mut self.test_dir_names, &path);
        self.test_path_exclusion_explanations
            .insert(path, explanation.into());
        self
    }

    /// Return a config that skips test-root parsing with an explanation.
    #[must_use]
    pub fn with_tests_excluded(mut self, explanation: impl Into<String>) -> Self {
        self.include_tests = false;
        self.test_path_exclusion_explanations
            .insert("tests".to_string(), explanation.into());
        self
    }

    /// Return a config that explains why cargo-check harness advice may pass.
    #[must_use]
    pub fn with_cargo_check_advice_allow_explanation(
        mut self,
        explanation: impl Into<String>,
    ) -> Self {
        self.cargo_check_advice_allow_explanation = Some(explanation.into());
        self
    }

    /// Return a config that explains why retired cargo-test harness advice may pass.
    #[must_use]
    pub fn with_cargo_test_advice_allow_explanation(
        mut self,
        explanation: impl Into<String>,
    ) -> Self {
        self.cargo_test_advice_allow_explanation = Some(explanation.into());
        self
    }

    /// Return a transitional config that explains why harness agent advice may pass.
    ///
    /// Prefer `with_cargo_check_advice_allow_explanation(...)` for build-script
    /// gates and `with_cargo_test_advice_allow_explanation(...)` for retired
    /// cargo-test gates.
    #[must_use]
    pub fn with_agent_advice_allow_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.agent_advice_allow_explanation = Some(explanation.into());
        self
    }

    /// Return a config with an explicit verification policy.
    #[must_use]
    pub fn with_verification_policy(mut self, policy: RustVerificationPolicy) -> Self {
        self.verification_policy = policy;
        self
    }

    /// Return a config with one verification profile hint appended.
    #[must_use]
    pub fn with_verification_profile_hint(mut self, hint: RustVerificationProfileHint) -> Self {
        self.verification_policy.profile_hints.push(hint);
        self
    }

    /// Return a config with one API path baseline appended.
    #[must_use]
    pub fn with_verification_api_path_baseline(
        mut self,
        baseline: RustVerificationApiPathBaseline,
    ) -> Self {
        self.verification_policy.api_path_baselines.push(baseline);
        self
    }

    /// Return a config with one verification receipt appended.
    #[must_use]
    pub fn with_verification_receipt(mut self, receipt: RustVerificationReceipt) -> Self {
        self.verification_policy.receipts.push(receipt);
        self
    }

    /// Return a config with one verification waiver appended.
    #[must_use]
    pub fn with_verification_waiver(mut self, waiver: RustVerificationWaiver) -> Self {
        self.verification_policy.waivers.push(waiver);
        self
    }

    /// Return a config with one verification task contract overridden.
    #[must_use]
    pub fn with_verification_task_contract(
        mut self,
        kind: RustVerificationTaskKind,
        contract: RustVerificationTaskContract,
    ) -> Self {
        self.verification_policy
            .task_contract_overrides
            .insert(kind, contract);
        self
    }

    /// Return a config with one verification skill binding configured.
    #[must_use]
    pub fn with_verification_skill_binding(
        mut self,
        kind: RustVerificationTaskKind,
        binding: RustVerificationSkillBinding,
    ) -> Self {
        self.verification_policy
            .skill_bindings
            .insert(kind, binding);
        self
    }

    /// Return a config with one verification skill descriptor configured.
    #[must_use]
    pub fn with_verification_skill_descriptor(
        mut self,
        descriptor: RustVerificationSkillDescriptor,
    ) -> Self {
        self.verification_policy
            .skill_descriptors
            .insert(descriptor.compact_label(), descriptor);
        self
    }

    /// Return a config with the built-in Criterion performance adapter enabled.
    #[must_use]
    pub fn with_criterion_performance_verification(self) -> Self {
        let descriptor = RustVerificationSkillDescriptor::criterion_performance();
        let binding = RustVerificationSkillBinding::new(descriptor.skill_id.clone()).with_adapter(
            descriptor
                .adapter
                .clone()
                .unwrap_or_else(|| "criterion".to_string()),
        );
        self.with_verification_skill_binding(RustVerificationTaskKind::Performance, binding)
            .with_verification_skill_descriptor(descriptor)
    }

    /// Return a config that marks one owner as latency-sensitive performance work.
    #[must_use]
    pub fn with_latency_sensitive_performance_owner(
        self,
        owner_path: impl Into<PathBuf>,
        rationale: impl Into<String>,
    ) -> Self {
        self.with_verification_profile_hint(
            RustVerificationProfileHint::new(
                owner_path,
                [RustOwnerResponsibility::LatencySensitive],
            )
            .with_task_kinds([RustVerificationTaskKind::Performance])
            .with_rationale(rationale),
        )
    }

    /// Return a config that marks one owner as availability-critical stability work.
    #[must_use]
    pub fn with_availability_stability_owner(
        self,
        owner_path: impl Into<PathBuf>,
        rationale: impl Into<String>,
    ) -> Self {
        self.with_verification_profile_hint(
            RustVerificationProfileHint::new(
                owner_path,
                [RustOwnerResponsibility::AvailabilityCritical],
            )
            .with_task_kinds([RustVerificationTaskKind::Stability])
            .with_rationale(rationale),
        )
    }

    /// Return a config with one responsibility mapped to explicit task kinds.
    #[must_use]
    pub fn with_verification_responsibility_task_kinds<I>(
        mut self,
        responsibility: RustOwnerResponsibility,
        task_kinds: I,
    ) -> Self
    where
        I: IntoIterator<Item = RustVerificationTaskKind>,
    {
        self.verification_policy
            .responsibility_task_overrides
            .insert(responsibility, task_kinds.into_iter().collect());
        self
    }

    /// Return a config with one project-owned dependency signal appended.
    #[must_use]
    pub fn with_verification_dependency_signal(
        mut self,
        signal: RustVerificationDependencySignal,
    ) -> Self {
        self.verification_policy.dependency_signals.push(signal);
        self
    }

    /// Return a config with project-owned stability picture requirements.
    #[must_use]
    pub fn with_verification_stability_picture(
        mut self,
        config: RustVerificationStabilityPictureConfig,
    ) -> Self {
        self.verification_policy.stability_picture = Some(config);
        self
    }
}

fn normalize_config_scope_path(path: String) -> String {
    path.trim().trim_matches('/').replace('\\', "/")
}

fn push_unique_scope_path(paths: &mut Vec<String>, path: String) {
    if !paths.iter().any(|candidate| candidate == &path) {
        paths.push(path);
    }
}

fn retain_scope_paths_except(paths: &mut Vec<String>, excluded_path: &str) {
    paths.retain(|path| normalize_config_scope_path(path.clone()) != excluded_path);
}

/// Aggregated harness report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspRustReport {
    /// Parsed source modules.
    pub modules: Vec<RustModuleReport>,
    /// All findings, including advisory findings.
    pub findings: Vec<AspRustFinding>,
    /// Machine-facing candidate invariants derived from findings.
    #[serde(
        default,
        rename = "invariantCandidates",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub invariant_candidates: Vec<RustInvariantCandidate>,
    /// Roots requested by the caller.
    pub root_paths: Vec<PathBuf>,
    /// Severities that block assertions.
    pub blocking_severities: BTreeSet<RustDiagnosticSeverity>,
    /// Project scope, when the project runner was used.
    pub project_resolution: Option<AspRustScope>,
    /// Cargo member scopes, when a workspace or package collection was scanned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workspace_member_scopes: Vec<AspRustScope>,
}

impl AspRustReport {
    /// Number of parsed-valid files.
    #[must_use]
    pub fn parsed_count(&self) -> usize {
        self.modules.iter().filter(|module| module.is_valid).count()
    }

    /// Number of discovered Rust files.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.modules.len()
    }

    /// Return whether there are no configured-blocking findings.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.blocking_findings(None).is_empty()
    }

    /// Return blocking findings for the selected severities.
    #[must_use]
    pub fn blocking_findings(
        &self,
        severities: Option<&BTreeSet<RustDiagnosticSeverity>>,
    ) -> Vec<&AspRustFinding> {
        let selected = severities.unwrap_or(&self.blocking_severities);
        self.findings
            .iter()
            .filter(|finding| selected.contains(&finding.severity))
            .collect()
    }

    /// Return non-blocking advisory findings.
    #[must_use]
    pub fn advisory_findings(&self) -> Vec<&AspRustFinding> {
        self.findings
            .iter()
            .filter(|finding| finding.severity == RustDiagnosticSeverity::Info)
            .collect()
    }

    /// Assert that the report has no non-blocking advisory findings.
    ///
    /// # Panics
    ///
    /// Panics with the compact rendered advice when advisory findings exist.
    #[track_caller]
    pub fn assert_no_advisory_findings(&self) {
        let rendered = crate::render_asp_rust_advice(self);
        assert!(rendered.is_empty(), "{rendered}");
    }

    /// Assert that the report has no configured-blocking findings.
    ///
    /// # Panics
    ///
    /// Panics with the compact rendered report when blocking findings exist.
    #[track_caller]
    pub fn assert_clean(&self) {
        assert!(self.is_clean(), "{}", crate::render_asp_rust(self));
    }
}
