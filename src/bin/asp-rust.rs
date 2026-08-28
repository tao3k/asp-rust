#![deny(dead_code)]

//! Command-line entry point for `asp-rust`.

use std::process::ExitCode;

fn main() -> ExitCode {
    rust_lang_project_harness::run_cli_from_env()
}
