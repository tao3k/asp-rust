use std::path::PathBuf;
use std::process::ExitCode;

use crate::{
    RustDeterminismReadinessInput, build_rust_determinism_readiness,
    render_rust_determinism_readiness, render_rust_determinism_readiness_json,
};

pub(super) fn run_determinism(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<ExitCode, String> {
    let options = DeterminismOptions::parse(args)?;
    if options.help {
        print_determinism_help();
        return Ok(ExitCode::SUCCESS);
    }
    if options.command.as_deref() != Some("readiness") {
        return Err("unknown determinism command: expected readiness".to_string());
    }
    let readiness = build_rust_determinism_readiness(RustDeterminismReadinessInput {
        project_root: options.project_root()?,
        include_tests: options.include_tests,
    })?;
    if options.json {
        println!(
            "{}",
            render_rust_determinism_readiness_json(&readiness)
                .map_err(|error| format!("failed to render determinism readiness JSON: {error}"))?
        );
    } else {
        print!("{}", render_rust_determinism_readiness(&readiness));
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Default)]
struct DeterminismOptions {
    command: Option<String>,
    project_root: Option<PathBuf>,
    include_tests: bool,
    json: bool,
    help: bool,
}

impl DeterminismOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let iter = args.into_iter().peekable();
        for arg in iter {
            let arg = arg
                .to_str()
                .ok_or_else(|| format!("non-utf8 argument: {arg:?}"))?;
            match arg {
                "readiness" if options.command.is_none() => {
                    options.command = Some(arg.to_owned());
                }
                "--include-tests" => {
                    options.include_tests = true;
                }
                "--json" => {
                    options.json = true;
                }
                "--help" | "-h" => {
                    options.help = true;
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown determinism option: {value}"));
                }
                value => {
                    if options.project_root.is_some() {
                        return Err(format!("unexpected extra determinism argument: {value}"));
                    }
                    options.project_root = Some(PathBuf::from(value));
                }
            }
        }
        Ok(options)
    }

    fn project_root(&self) -> Result<PathBuf, String> {
        let root = self
            .project_root
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        root.canonicalize()
            .map_err(|error| format!("failed to resolve project root {}: {error}", root.display()))
    }
}

fn print_determinism_help() {
    println!(
        "asp-rust determinism readiness [--include-tests] [--json] [PROJECT_ROOT]\n\n\
Detects direct clock, random, filesystem, network, environment, and global-state access.\n\
Use --json to emit the semantic-determinism-readiness JSON contract."
    );
}
