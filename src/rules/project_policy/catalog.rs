//! Project-policy rule catalog.

use std::collections::BTreeMap;

use crate::rules::labels;
use crate::rules::project_policy::RUST_PROJ_R024;
use crate::{AspRustRule, RustDiagnosticSeverity};

use super::{
    PACK_ID, RUST_PROJ_R001, RUST_PROJ_R002, RUST_PROJ_R003, RUST_PROJ_R004, RUST_PROJ_R005,
    RUST_PROJ_R006, RUST_PROJ_R007, RUST_PROJ_R008, RUST_PROJ_R009, RUST_PROJ_R010, RUST_PROJ_R011,
    RUST_PROJ_R012, RUST_PROJ_R013, RUST_PROJ_R014, RUST_PROJ_R015, RUST_PROJ_R016, RUST_PROJ_R017,
    RUST_PROJ_R018, RUST_PROJ_R019, RUST_PROJ_R020, RUST_PROJ_R021, RUST_PROJ_R022, RUST_PROJ_R023,
};

pub(super) fn rules_by_id() -> BTreeMap<&'static str, AspRustRule> {
    [
        AspRustRule::new(
            RUST_PROJ_R001,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Root test file lacks explicit harness role",
            "Move root-level test files under tests/unit or tests/integration and declare nonstandard isolated targets directly in Cargo.toml.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R002,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Unexpected tests directory",
            "Keep only standard suite directories directly under tests; project-local layout exceptions are not accepted.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R003,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Inline source test module",
            "Keep test bodies out of src files; mount source-backed unit tests from tests/unit with #[path].",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R004,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "External test mount is missing or misplaced",
            "External source-backed tests must resolve to existing files under tests/unit.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R005,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Large test leaf should split",
            "Split oversized test leaves into folder-first suites with focused modules.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R006,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Retired cargo-test target gate should migrate",
            "Root Cargo test target harness gates should move parser-native project policy to the cargo-check build gate and keep root test targets as thin suite aggregates.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R007,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Root test target contains implementation",
            "Keep root Cargo test targets as thin aggregates with external module mounts only.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R008,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Root test module mount is implicit or misplaced",
            "Root Cargo test targets must mount external modules with explicit #[path] attributes pointing under an allowed tests suite directory.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R009,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Retired source cargo-test gate should migrate",
            "Source cargo-test harness gates should move parser-native project policy to the cargo-check build gate and remove the source test gate once the build gate is active.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R010,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Performance verification skill lacks Cargo bench target",
            "When a Rust-native performance skill is configured, expose a runnable harness=false [[bench]] target and benchmark framework dev-dependency; keep build.rs as a structural gate and record benchmark runs through performance receipts.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R011,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Cargo-check harness gate uses empty verification config",
            "Mount cargo-check build gates with explicit verification profile hints, task suppressions, or skill bindings so parser-native policy tells agents which verification duties apply before tests run.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R012,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "ASP Rust build-time contract evidence is incomplete",
            "ASP Rust-enabled packages should emit lightweight build-time contract evidence while full parser-native project policy remains in the test/evaluation layer.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R013,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Custom harness scope path lacks explanation",
            "Custom source or test scope paths must be added with an explicit explanation so agents cannot shrink the harness to avoid existing policy debt.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R014,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Cargo-backed harness scope reduction lacks explanation",
            "Cargo-backed source or test scopes that exist in the package must stay covered or be removed with an explicit explanation so agents cannot skip existing source or test debt.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R015,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Retired cargo-test advice allowance lacks explanation",
            "Retired cargo-test advice allowance must include an explicit explanation so agents cannot silence advisory policy just to pass the test layer.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R016,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Retired cargo-test gate uses empty verification config",
            "Source cargo-test gates that remain in use must declare verification profile hints, task suppressions, or skill bindings for the test layer.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R017,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Advice allowance explanation is too weak",
            "Advice allowances must state scope, owner, finding category, why the code is safe now, and the cleanup trigger so agents cannot use explanations as blanket waivers.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R018,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Policy identity fallback is fake",
            "Policy and evidence identity must fail closed when Cargo identity is missing; do not synthesize unknown/default/todo labels that can enter receipts.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R019,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Workspace member build-gate wrapper alias is redundant",
            "Expose one canonical workspace member build-gate entrypoint; duplicate public aliases need deprecation metadata and a bounded migration plan.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R020,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Evidence pipeline silently defaults missing data",
            "Search, graph, policy, and evidence code must not turn missing lineage, identity, or candidate data into a default value without a typed reason or diagnostic.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R021,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Source location conversion uses sentinel",
            "Line, column, range, and source-location conversions must not use sentinel values such as usize::MAX; reject the candidate or report a decode error.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R022,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Candidate loop drops rows without telemetry",
            "Streaming candidate loops that skip scored rows should record filtered or rejected rows so query quality and recall gaps are observable.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R023,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Cargo package edition is stale",
            "Agent-authored Rust packages should target edition 2024 by default; older editions need an explicit compatibility reason and migration plan.",
            labels("project-policy"),
        ),
        AspRustRule::new(
            RUST_PROJ_R024,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Test support module is too large",
            "Split oversized test support and fixture modules into folder-first support owners with focused responsibilities.",
            labels("project-policy"),
        ),
    ]
    .into_iter()
    .map(|rule| (rule.rule_id, rule))
    .collect()
}
