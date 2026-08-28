use std::process::ExitCode;

use crate::{
    RustFormalProofPilotInput, build_rust_dependency_graph_acyclicity_proof_pilot,
    render_rust_formal_proof_pilot, render_rust_formal_proof_pilot_json,
};

pub(super) fn run_proof(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<ExitCode, String> {
    let options = ProofOptions::parse(args)?;
    if options.help {
        print_proof_help();
        return Ok(ExitCode::SUCCESS);
    }
    if options.command.as_deref() != Some("pilot")
        || options.target.as_deref() != Some("dependency-graph-acyclicity")
    {
        return Err(
            "unknown proof command: expected pilot dependency-graph-acyclicity".to_string(),
        );
    }
    let proof = build_rust_dependency_graph_acyclicity_proof_pilot(RustFormalProofPilotInput {
        max_nodes: options.max_nodes,
    })?;
    if options.json {
        println!(
            "{}",
            render_rust_formal_proof_pilot_json(&proof)
                .map_err(|error| format!("failed to render formal proof pilot JSON: {error}"))?
        );
    } else {
        print!("{}", render_rust_formal_proof_pilot(&proof));
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Debug)]
struct ProofOptions {
    command: Option<String>,
    target: Option<String>,
    max_nodes: usize,
    json: bool,
    help: bool,
}

impl Default for ProofOptions {
    fn default() -> Self {
        Self {
            command: None,
            target: None,
            max_nodes: 4,
            json: false,
            help: false,
        }
    }
}

impl ProofOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            let arg = arg
                .to_str()
                .ok_or_else(|| format!("non-utf8 argument: {arg:?}"))?;
            match arg {
                "pilot" if options.command.is_none() => {
                    options.command = Some(arg.to_owned());
                }
                "dependency-graph-acyclicity" if options.target.is_none() => {
                    options.target = Some(arg.to_owned());
                }
                "--max-nodes" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "--max-nodes requires a value".to_string())?;
                    let value = value
                        .to_str()
                        .ok_or_else(|| format!("non-utf8 --max-nodes value: {value:?}"))?;
                    options.max_nodes = value
                        .parse::<usize>()
                        .map_err(|error| format!("invalid --max-nodes value {value}: {error}"))?;
                }
                "--json" => {
                    options.json = true;
                }
                "--help" | "-h" => {
                    options.help = true;
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown proof option: {value}"));
                }
                value => {
                    return Err(format!("unexpected proof argument: {value}"));
                }
            }
        }
        Ok(options)
    }
}

fn print_proof_help() {
    println!(
        "asp-rust proof pilot dependency-graph-acyclicity [--max-nodes N] [--json]\n\n\
Runs the P4 proof pilot over the RUST-AGENT-OWNER-GRAPH-009 dependency graph cycle core.\n\
Use --json to emit the semantic-formal-proof-pilot JSON contract."
    );
}
