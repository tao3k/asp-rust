//! Rust syntax rule pack.

use crate::parser::{ParsedRustModule, source_line, span_location};
use crate::{AspRustFinding, AspRustRule, RustDiagnosticSeverity};

use super::{display_path, labels};

const PACK_ID: &str = "rust.syntax";
const RUST_SYN_R001: &str = "RUST-SYN-R001";

/// Return compact metadata for Rust syntax rules.
#[must_use]
pub fn rust_syntax_rules() -> Vec<AspRustRule> {
    vec![rule()]
}

pub(crate) fn evaluate(modules: &[ParsedRustModule]) -> Vec<AspRustFinding> {
    let rule = rule();
    modules
        .iter()
        .filter(|module| !module.report.is_valid)
        .map(|module| {
            let location = module.error_span.map_or_else(
                || crate::parser::file_location(&module.report.path),
                |span| span_location(Some(module.report.path.clone()), span),
            );
            let line = source_line(&module.source, location.line);
            AspRustFinding::from_rule(
                &rule,
                format!(
                    "{} could not be parsed as Rust syntax: {}",
                    display_path(&module.report.path),
                    module
                        .report
                        .parse_error
                        .as_deref()
                        .unwrap_or("unknown parse error")
                ),
                location,
                line,
                "repair Rust syntax near this token",
            )
        })
        .collect()
}

fn rule() -> AspRustRule {
    AspRustRule::new(
        RUST_SYN_R001,
        PACK_ID,
        RustDiagnosticSeverity::Error,
        "Rust source does not parse",
        "Fix Rust syntax so the file parses through syn before project policy rules run.",
        labels("syntax"),
    )
}
