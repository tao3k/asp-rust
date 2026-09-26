//! Modularity rule catalog.

use std::collections::BTreeMap;

use crate::rules::labels;
use crate::{AspRustRule, RustDiagnosticSeverity};

use super::{
    PACK_ID, RUST_MOD_R001, RUST_MOD_R002, RUST_MOD_R003, RUST_MOD_R004, RUST_MOD_R005,
    RUST_MOD_R006, RUST_MOD_R007, RUST_MOD_R008, RUST_MOD_R009, RUST_MOD_R010, RUST_MOD_R011,
};

pub(super) fn rules_by_id() -> BTreeMap<&'static str, AspRustRule> {
    [
        AspRustRule::new(
            RUST_MOD_R001,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "mod.rs contains implementation",
            "Keep mod.rs as an interface file with module declarations and re-exports only.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R002,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Source file carries too many responsibilities",
            "Split oversized source files into smaller owner modules with clear public boundaries.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R003,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Deep relative import crosses owner boundaries",
            "Replace super::super imports with crate::... owner/facade imports; if the target is a leaf implementation, expose the needed API through an owner facade first.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R004,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "lib.rs contains implementation",
            "Keep lib.rs as a crate facade with external module declarations, re-exports, and parser-proven boundary macros only.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R005,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Binary entrypoint contains implementation",
            "Keep src/main.rs and src/bin entrypoints as thin adapters with use declarations and fn main only.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R006,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Build script contains implementation",
            "Keep root build.rs as a thin Cargo build-script entrypoint with use declarations and fn main only.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R007,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Module source has file and mod.rs forms",
            "Do not keep both foo.rs and foo/mod.rs for the same Rust module owner; choose one source layout.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R008,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Source module is inline",
            "Keep reasoning-tree branches file-backed; move inline source modules into external module files.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R009,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Source file is orphaned from the module tree",
            "Every scanned source file must be reachable from a crate or binary root through external mod declarations.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R010,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Glob import hides owner scope",
            "Avoid every Rust glob import, including absolute crate globs; import owner names explicitly so module dependencies stay visible.",
            labels("modularity"),
        ),
        AspRustRule::new(
            RUST_MOD_R011,
            PACK_ID,
            RustDiagnosticSeverity::Warning,
            "Sibling file and directory share an owner name",
            "Do not keep both foo.rs and foo/ child sources at the same filesystem level; move the owner interface to foo/mod.rs and keep implementation under foo/*.",
            labels("modularity"),
        ),
    ]
    .into_iter()
    .map(|rule| (rule.rule_id, rule))
    .collect()
}
