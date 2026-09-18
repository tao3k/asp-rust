//! Embedded ASP Rust rule rendering for downstream test fixtures.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Source-embedded ASP Rust rule list.
pub const ASP_RUST_RULES_MD: &str = include_str!("asp-rust-rules.md");

/// Return the source-embedded ASP Rust rule list.
#[must_use]
pub const fn asp_rust_rules_markdown() -> &'static str {
    ASP_RUST_RULES_MD
}

/// Render the source-embedded ASP Rust rules as generated markdown.
#[must_use]
pub fn render_asp_rust_rules_markdown() -> String {
    let mut output = String::from(
        "# asp-rust\n\n\
         ## ASP Rust Rules\n\n\
         Generated from embedded `src/asp-rust-rules.md`.\n\n",
    );
    for line in ASP_RUST_RULES_MD.lines() {
        if let Some(item) = line.strip_prefix("- ")
            && let Some((rule_id, sentence)) = item.split_once(": ")
        {
            output.push_str("- **");
            output.push_str(rule_id);
            output.push_str("**: ");
            output.push_str(sentence);
            output.push('\n');
        }
    }
    output
}

/// Write the generated ASP Rust rules into a downstream unit test directory.
///
/// Downstream crates can call this from `build.rs` after adding ASP Rust as
/// a build dependency, then commit or assert the generated unit fixture.
pub fn write_asp_rust_rules_to_unit_tests(unit_test_dir: impl AsRef<Path>) -> io::Result<PathBuf> {
    let output_path = unit_test_dir.as_ref().join("asp-rust-rules.generated.md");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, render_asp_rust_rules_markdown())?;
    Ok(output_path)
}
