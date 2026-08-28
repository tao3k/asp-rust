//! Parsed Rust module representation and file parsing.

use std::fs;
use std::path::Path;

use proc_macro2::Span;

use crate::RustModuleReport;

use super::{RustNativeSyntaxFacts, RustSourceMetrics};

pub(crate) struct ParsedRustModule {
    pub report: RustModuleReport,
    pub source: String,
    pub syntax_facts: RustNativeSyntaxFacts,
    pub source_metrics: RustSourceMetrics,
    pub error_span: Option<Span>,
}

pub(crate) fn parse_rust_source_syntax(source: &str) -> Result<syn::File, syn::Error> {
    syn::parse_file(source)
}

pub(crate) fn parse_rust_file(path: &Path) -> ParsedRustModule {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return ParsedRustModule {
                report: RustModuleReport {
                    path: path.to_path_buf(),
                    is_valid: false,
                    parse_error: Some(format!("failed to read Rust source: {error}")),
                },
                source: String::new(),
                syntax_facts: RustNativeSyntaxFacts::default(),
                source_metrics: RustSourceMetrics::default(),
                error_span: None,
            };
        }
    };
    parse_rust_source(path, source)
}

pub(crate) fn parse_rust_source(path: &Path, source: String) -> ParsedRustModule {
    let source_metrics = super::source_metrics::rust_source_metrics(&source);
    match parse_rust_source_syntax(&source) {
        Ok(syntax) => {
            let syntax_facts = super::native_syntax::rust_native_syntax_facts(&syntax, path);
            ParsedRustModule {
                report: RustModuleReport {
                    path: path.to_path_buf(),
                    is_valid: true,
                    parse_error: None,
                },
                source,
                syntax_facts,
                source_metrics,
                error_span: None,
            }
        }
        Err(error) => ParsedRustModule {
            report: RustModuleReport {
                path: path.to_path_buf(),
                is_valid: false,
                parse_error: Some(error.to_string()),
            },
            source,
            syntax_facts: RustNativeSyntaxFacts::default(),
            source_metrics,
            error_span: Some(error.span()),
        },
    }
}
