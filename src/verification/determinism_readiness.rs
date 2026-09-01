//! Determinism readiness evidence for direct external state access.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discovery::{asp_rust_scope, discover_rust_files};
use crate::model::AspRustConfig;
use crate::parser::{ParsedRustModule, parse_rust_file};

/// Shared determinism readiness schema id.
pub const RUST_DETERMINISM_READINESS_SCHEMA_ID: &str =
    "agent.semantic-protocols.semantic-determinism-readiness";

/// Shared determinism readiness schema version.
pub const RUST_DETERMINISM_READINESS_SCHEMA_VERSION: &str = "1";

/// Shared determinism readiness protocol id.
pub const RUST_DETERMINISM_READINESS_PROTOCOL_ID: &str =
    "agent.semantic-protocols.determinism-readiness";

/// Shared determinism readiness protocol version.
pub const RUST_DETERMINISM_READINESS_PROTOCOL_VERSION: &str = "1";

/// Input for building a determinism readiness packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustDeterminismReadinessInput {
    /// Project root to scan.
    pub project_root: PathBuf,
    /// Whether test roots should be included.
    pub include_tests: bool,
}

/// Determinism readiness packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDeterminismReadiness {
    /// Shared schema id.
    pub schema_id: RustDeterminismReadinessSchemaId,
    /// Shared schema version.
    pub schema_version: RustDeterminismReadinessSchemaVersion,
    /// Shared protocol id.
    pub protocol_id: RustDeterminismReadinessProtocolId,
    /// Shared protocol version.
    pub protocol_version: RustDeterminismReadinessProtocolVersion,
    /// Stable readiness id.
    pub readiness_id: RustDeterminismReadinessId,
    /// Producer metadata.
    pub producer: RustDeterminismReadinessProducer,
    /// Project metadata.
    pub project: RustDeterminismReadinessProject,
    /// Overall readiness status.
    pub status: RustDeterminismReadinessStatus,
    /// Direct nondeterminism observations.
    pub observations: Vec<RustDeterminismReadinessObservation>,
    /// Suggested injection or explicit-state repairs.
    pub suggestions: Vec<RustDeterminismReadinessSuggestion>,
    /// Linked invariant candidate ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_ids: Vec<String>,
    /// Additional provider-owned fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Stable readiness packet id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessId(pub String);

/// Schema id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessSchemaId(pub String);

/// Schema version newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessSchemaVersion(pub String);

/// Protocol id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessProtocolId(pub String);

/// Protocol version newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessProtocolVersion(pub String);

/// Readiness producer metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDeterminismReadinessProducer {
    /// Source language id.
    pub language_id: RustDeterminismReadinessLanguageId,
    /// Provider id.
    pub provider_id: RustDeterminismReadinessProviderId,
    /// Provider namespace.
    pub namespace: RustDeterminismReadinessNamespace,
}

/// Language id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessLanguageId(pub String);

/// Provider id newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessProviderId(pub String);

/// Provider namespace newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessNamespace(pub String);

/// Readiness project metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDeterminismReadinessProject {
    /// Project root as represented in the packet.
    pub root: PathBuf,
    /// Package name, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// Additional project facts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Overall determinism readiness status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustDeterminismReadinessStatus {
    /// No direct nondeterminism source was detected.
    Ready,
    /// Direct nondeterminism source should be injected or made explicit.
    NeedsInjection,
    /// Provider could not complete the readiness scan.
    Blocked,
    /// Provider cannot classify the project.
    Unknown,
}

/// Direct nondeterminism category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustDeterminismReadinessCategory {
    /// Direct clock or time access.
    Clock,
    /// Direct random source access.
    Random,
    /// Direct filesystem access.
    Filesystem,
    /// Direct network access.
    Network,
    /// Direct environment/process access.
    Environment,
    /// Direct static or global state.
    GlobalState,
}

/// Observation evidence kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustDeterminismReadinessEvidenceKind {
    /// Function call evidence.
    FunctionCall,
    /// Macro invocation evidence.
    MacroInvocation,
    /// Path reference evidence.
    PathReference,
    /// Static item evidence.
    StaticItem,
}

/// Readiness observation severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustDeterminismReadinessSeverity {
    /// Informational observation.
    Info,
    /// Observation should be reviewed.
    Warning,
    /// Observation blocks readiness.
    Error,
}

/// Direct determinism readiness observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDeterminismReadinessObservation {
    /// Stable observation id.
    pub observation_id: RustDeterminismReadinessObservationId,
    /// Direct source category.
    pub category: RustDeterminismReadinessCategory,
    /// Evidence kind.
    pub evidence_kind: RustDeterminismReadinessEvidenceKind,
    /// Review severity.
    pub severity: RustDeterminismReadinessSeverity,
    /// Human-readable summary.
    pub summary: RustDeterminismReadinessSummary,
    /// Project-relative source path.
    pub path: PathBuf,
    /// One-based source line.
    pub line: RustDeterminismReadinessLine,
    /// Terminal symbol, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<RustDeterminismReadinessSymbol>,
    /// Source expression, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<RustDeterminismReadinessExpression>,
    /// Source line excerpt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_line: Option<String>,
    /// Whether the evidence was directly observed by the provider.
    pub direct: bool,
    /// Additional provider-owned fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Stable observation id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessObservationId(pub String);

/// Observation summary newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessSummary(pub String);

/// One-based source line newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessLine(pub usize);

/// Terminal symbol newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessSymbol(pub String);

/// Source expression newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessExpression(pub String);

/// Determinism readiness suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDeterminismReadinessSuggestion {
    /// Suggested repair kind.
    pub kind: RustDeterminismReadinessSuggestionKind,
    /// Category the suggestion addresses.
    pub category: RustDeterminismReadinessCategory,
    /// Suggestion message.
    pub message: RustDeterminismReadinessSuggestionMessage,
    /// Project-relative source path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// One-based source line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<RustDeterminismReadinessLine>,
    /// Suggested trait name, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trait_name: Option<RustDeterminismReadinessTraitName>,
    /// Additional provider-owned fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// Suggestion kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustDeterminismReadinessSuggestionKind {
    /// Introduce a trait-backed dependency.
    TraitInjection,
    /// Pass the value as an explicit parameter.
    ExplicitParameter,
    /// Encapsulate state behind an explicit handle.
    StateHandle,
    /// Use resettable fixture ownership in tests.
    TestFixture,
}

/// Suggestion message newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessSuggestionMessage(pub String);

/// Suggested trait name newtype.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RustDeterminismReadinessTraitName(pub String);

/// Build a Rust determinism readiness packet.
///
/// # Errors
///
/// Returns an error when the project root does not exist.
pub fn build_rust_determinism_readiness(
    input: RustDeterminismReadinessInput,
) -> Result<RustDeterminismReadiness, String> {
    if !input.project_root.exists() {
        return Err(format!(
            "project root does not exist: {}",
            input.project_root.display()
        ));
    }

    let config = AspRustConfig {
        include_tests: input.include_tests,
        ..Default::default()
    };
    let scope = asp_rust_scope(
        &input.project_root,
        config.include_tests,
        &config.source_dir_names,
        &config.test_dir_names,
    );
    let monitored_paths = scope.monitored_paths();
    let parsed_modules = discover_rust_files(
        &monitored_paths,
        &config.ignored_dir_names,
        &config.include_hidden_dir_names,
    )
    .into_iter()
    .map(|path| parse_rust_file(&path))
    .collect::<Vec<_>>();

    let mut observations = parsed_modules
        .iter()
        .flat_map(|module| observations_for_module(&input.project_root, module))
        .collect::<Vec<_>>();
    observations.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.0.cmp(&right.line.0))
            .then(left.category.cmp(&right.category))
            .then(left.observation_id.cmp(&right.observation_id))
    });
    for (index, observation) in observations.iter_mut().enumerate() {
        observation.observation_id = RustDeterminismReadinessObservationId(format!(
            "{}:{}:{}",
            observation.category.as_id(),
            sanitize_id_part(&observation.path.to_string_lossy()),
            index + 1
        ));
    }
    let suggestions = observations
        .iter()
        .map(suggestion_for_observation)
        .collect::<Vec<_>>();
    let status = if observations.is_empty() {
        RustDeterminismReadinessStatus::Ready
    } else {
        RustDeterminismReadinessStatus::NeedsInjection
    };

    Ok(RustDeterminismReadiness {
        schema_id: RustDeterminismReadinessSchemaId(
            RUST_DETERMINISM_READINESS_SCHEMA_ID.to_owned(),
        ),
        schema_version: RustDeterminismReadinessSchemaVersion(
            RUST_DETERMINISM_READINESS_SCHEMA_VERSION.to_owned(),
        ),
        protocol_id: RustDeterminismReadinessProtocolId(
            RUST_DETERMINISM_READINESS_PROTOCOL_ID.to_owned(),
        ),
        protocol_version: RustDeterminismReadinessProtocolVersion(
            RUST_DETERMINISM_READINESS_PROTOCOL_VERSION.to_owned(),
        ),
        readiness_id: RustDeterminismReadinessId("rust.determinism-readiness.project".to_owned()),
        producer: default_producer(),
        project: RustDeterminismReadinessProject {
            root: PathBuf::from("."),
            package: None,
            fields: BTreeMap::new(),
        },
        status,
        observations,
        suggestions,
        candidate_ids: Vec::new(),
        fields: BTreeMap::new(),
    })
}

/// Render determinism readiness as compact text.
#[must_use]
pub fn render_rust_determinism_readiness(readiness: &RustDeterminismReadiness) -> String {
    let mut output = format!(
        "[determinism-readiness] status={:?} observations={} suggestions={}\n",
        readiness.status,
        readiness.observations.len(),
        readiness.suggestions.len()
    );
    for observation in &readiness.observations {
        output.push_str(&format!(
            "|observation category={} evidence={:?} path={} line={} summary={}\n",
            observation.category.as_id(),
            observation.evidence_kind,
            observation.path.display(),
            observation.line.0,
            observation.summary.0
        ));
    }
    output
}

/// Render determinism readiness as JSON.
///
/// # Errors
///
/// Returns an error when JSON serialization fails.
pub fn render_rust_determinism_readiness_json(
    readiness: &RustDeterminismReadiness,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(readiness)
}

fn observations_for_module(
    project_root: &Path,
    module: &ParsedRustModule,
) -> Vec<RustDeterminismReadinessObservation> {
    if !module.report.is_valid {
        return Vec::new();
    }

    let mut observations = Vec::new();
    for call in &module.syntax_facts.function_calls {
        let expression = expression_for_line_and_terminal(module, call.line, &call.terminal_name)
            .unwrap_or_else(|| call.terminal_name.clone());
        if let Some(category) = category_for_call(&expression, &call.terminal_name) {
            observations.push(observation(
                project_root,
                module,
                category,
                RustDeterminismReadinessEvidenceKind::FunctionCall,
                call.line,
                Some(call.terminal_name.clone()),
                Some(expression),
            ));
        }
    }
    for invocation in &module.syntax_facts.macro_invocations {
        if let Some(category) = category_for_macro(&invocation.terminal_name) {
            let expression = format!("{}!", invocation.terminal_name);
            observations.push(observation(
                project_root,
                module,
                category,
                RustDeterminismReadinessEvidenceKind::MacroInvocation,
                invocation.line,
                Some(invocation.terminal_name.clone()),
                Some(expression),
            ));
        }
    }
    for item in &module.syntax_facts.top_level_items {
        if item.kind == "static" {
            observations.push(observation(
                project_root,
                module,
                RustDeterminismReadinessCategory::GlobalState,
                RustDeterminismReadinessEvidenceKind::StaticItem,
                item.line,
                item.name.clone(),
                item.name.as_ref().map(|name| format!("static {name}")),
            ));
        }
    }
    observations
}

fn observation(
    project_root: &Path,
    module: &ParsedRustModule,
    category: RustDeterminismReadinessCategory,
    evidence_kind: RustDeterminismReadinessEvidenceKind,
    line: usize,
    symbol: Option<String>,
    expression: Option<String>,
) -> RustDeterminismReadinessObservation {
    let relative_path = project_relative_path(project_root, &module.report.path);
    let mut fields = BTreeMap::new();
    fields.insert("detection".to_owned(), "parser-owned-direct".to_owned());
    RustDeterminismReadinessObservation {
        observation_id: RustDeterminismReadinessObservationId(format!(
            "{}:{}:{}",
            category.as_id(),
            sanitize_id_part(&relative_path.to_string_lossy()),
            line
        )),
        category,
        evidence_kind,
        severity: RustDeterminismReadinessSeverity::Warning,
        summary: RustDeterminismReadinessSummary(summary_for_category(category, &expression)),
        path: relative_path,
        line: RustDeterminismReadinessLine(line.max(1)),
        symbol: symbol.map(RustDeterminismReadinessSymbol),
        expression: expression.map(RustDeterminismReadinessExpression),
        source_line: source_line(&module.source, line),
        direct: true,
        fields,
    }
}

fn expression_for_line_and_terminal(
    module: &ParsedRustModule,
    line: usize,
    terminal: &str,
) -> Option<String> {
    module
        .syntax_facts
        .path_references
        .iter()
        .filter(|reference| reference.line == line && reference.terminal_name == terminal)
        .max_by_key(|reference| reference.segments.len())
        .map(|reference| reference.segments.join("::"))
}

fn category_for_call(expression: &str, terminal: &str) -> Option<RustDeterminismReadinessCategory> {
    if is_clock_call(expression, terminal) {
        return Some(RustDeterminismReadinessCategory::Clock);
    }
    if is_random_call(expression, terminal) {
        return Some(RustDeterminismReadinessCategory::Random);
    }
    if is_filesystem_call(expression, terminal) {
        return Some(RustDeterminismReadinessCategory::Filesystem);
    }
    if is_network_call(expression) {
        return Some(RustDeterminismReadinessCategory::Network);
    }
    if is_environment_call(expression, terminal) {
        return Some(RustDeterminismReadinessCategory::Environment);
    }
    None
}

fn category_for_macro(macro_name: &str) -> Option<RustDeterminismReadinessCategory> {
    match macro_name {
        "env" | "option_env" => Some(RustDeterminismReadinessCategory::Environment),
        "include_str" | "include_bytes" => Some(RustDeterminismReadinessCategory::Filesystem),
        _ => None,
    }
}

fn is_clock_call(expression: &str, terminal: &str) -> bool {
    matches!(terminal, "now" | "now_utc" | "now_local")
        && (expression.contains("SystemTime")
            || expression.contains("Instant")
            || expression.contains("Utc")
            || expression.contains("Local")
            || contains_utc_adjusted_datetime_type(expression))
}

fn contains_utc_adjusted_datetime_type(expression: &str) -> bool {
    ["Off", "set", "Date", "Time"]
        .into_iter()
        .all(|part| expression.contains(part))
}

fn is_random_call(expression: &str, terminal: &str) -> bool {
    expression.starts_with("rand::")
        || expression.starts_with("getrandom::")
        || matches!(
            terminal,
            "random" | "thread_rng" | "random_range" | "from_entropy"
        )
}

fn is_filesystem_call(expression: &str, terminal: &str) -> bool {
    expression.starts_with("std::fs::")
        || expression.starts_with("fs::")
        || expression.starts_with("File::")
        || expression.starts_with("std::fs::File::")
        || expression.starts_with("OpenOptions::")
        || (expression.contains("fs::")
            && matches!(
                terminal,
                "read"
                    | "read_to_string"
                    | "write"
                    | "create"
                    | "open"
                    | "metadata"
                    | "canonicalize"
                    | "read_dir"
                    | "remove_file"
                    | "remove_dir_all"
            ))
}

fn is_network_call(expression: &str) -> bool {
    expression.starts_with("std::net::")
        || expression.starts_with("TcpStream::")
        || expression.starts_with("UdpSocket::")
        || expression.starts_with("TcpListener::")
        || expression.starts_with("reqwest::")
        || expression.starts_with("ureq::")
        || expression.starts_with("hyper::")
}

fn is_environment_call(expression: &str, terminal: &str) -> bool {
    (expression.starts_with("std::env::") || expression.starts_with("env::"))
        && matches!(
            terminal,
            "var"
                | "var_os"
                | "vars"
                | "vars_os"
                | "args"
                | "args_os"
                | "current_dir"
                | "set_var"
                | "remove_var"
        )
}

fn suggestion_for_observation(
    observation: &RustDeterminismReadinessObservation,
) -> RustDeterminismReadinessSuggestion {
    let (kind, trait_name, message) = match observation.category {
        RustDeterminismReadinessCategory::Clock => (
            RustDeterminismReadinessSuggestionKind::TraitInjection,
            Some("Clock"),
            "Inject a clock provider trait instead of reading time directly.",
        ),
        RustDeterminismReadinessCategory::Random => (
            RustDeterminismReadinessSuggestionKind::TraitInjection,
            Some("RandomSource"),
            "Inject a random source trait or deterministic seed instead of reading randomness directly.",
        ),
        RustDeterminismReadinessCategory::Filesystem => (
            RustDeterminismReadinessSuggestionKind::TraitInjection,
            Some("Filesystem"),
            "Inject a filesystem boundary or explicit path/content parameter instead of reading files directly.",
        ),
        RustDeterminismReadinessCategory::Network => (
            RustDeterminismReadinessSuggestionKind::TraitInjection,
            Some("NetworkClient"),
            "Inject a network client trait instead of opening network connections directly.",
        ),
        RustDeterminismReadinessCategory::Environment => (
            RustDeterminismReadinessSuggestionKind::ExplicitParameter,
            None,
            "Pass environment-derived values explicitly instead of reading process environment directly.",
        ),
        RustDeterminismReadinessCategory::GlobalState => (
            RustDeterminismReadinessSuggestionKind::StateHandle,
            None,
            "Move static state behind an explicit state handle or resettable test fixture.",
        ),
    };
    RustDeterminismReadinessSuggestion {
        kind,
        category: observation.category,
        message: RustDeterminismReadinessSuggestionMessage(message.to_owned()),
        path: Some(observation.path.clone()),
        line: Some(observation.line),
        trait_name: trait_name.map(|name| RustDeterminismReadinessTraitName(name.to_owned())),
        fields: BTreeMap::new(),
    }
}

fn summary_for_category(
    category: RustDeterminismReadinessCategory,
    expression: &Option<String>,
) -> String {
    let subject = expression.as_deref().unwrap_or("direct source");
    match category {
        RustDeterminismReadinessCategory::Clock => {
            format!("Direct clock read uses {subject}.")
        }
        RustDeterminismReadinessCategory::Random => {
            format!("Direct random source uses {subject}.")
        }
        RustDeterminismReadinessCategory::Filesystem => {
            format!("Direct filesystem access uses {subject}.")
        }
        RustDeterminismReadinessCategory::Network => {
            format!("Direct network access uses {subject}.")
        }
        RustDeterminismReadinessCategory::Environment => {
            format!("Direct environment access uses {subject}.")
        }
        RustDeterminismReadinessCategory::GlobalState => {
            format!("Direct global state declaration uses {subject}.")
        }
    }
}

fn default_producer() -> RustDeterminismReadinessProducer {
    RustDeterminismReadinessProducer {
        language_id: RustDeterminismReadinessLanguageId("rust".to_owned()),
        provider_id: RustDeterminismReadinessProviderId("asp-rust".to_owned()),
        namespace: RustDeterminismReadinessNamespace(
            "agent.semantic-protocols.languages.rust.asp-rust".to_owned(),
        ),
    }
}

fn project_relative_path(project_root: &Path, path: &Path) -> PathBuf {
    let root = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let source = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    source
        .strip_prefix(&root)
        .or_else(|_| path.strip_prefix(project_root))
        .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
}

fn source_line(source: &str, line: usize) -> Option<String> {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
}

fn sanitize_id_part(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else if matches!(character, '.' | ':' | '_' | '-') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "x".to_owned()
    } else {
        sanitized
    }
}

impl RustDeterminismReadinessCategory {
    fn as_id(self) -> &'static str {
        match self {
            Self::Clock => "clock",
            Self::Random => "random",
            Self::Filesystem => "filesystem",
            Self::Network => "network",
            Self::Environment => "environment",
            Self::GlobalState => "global-state",
        }
    }
}
