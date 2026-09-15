#![deny(dead_code)]

//! Command-line entry point for `asp-rust`.

use std::process::ExitCode;

fn main() -> ExitCode {
    asp_rust::run_provider_server_from_env()
}
