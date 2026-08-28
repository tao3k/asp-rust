use crate::verification::{
    RUST_EVIDENCE_GRAPH_PROTOCOL_ID, RUST_EVIDENCE_GRAPH_SCHEMA_ID, RUST_REVIEW_PACKET_PROTOCOL_ID,
    RUST_REVIEW_PACKET_SCHEMA_ID, RustAssuranceCaseInput, RustEvidenceGraph,
    RustEvidenceGraphAnalysisInput, RustEvidenceGraphInput, RustReviewPacket,
    build_rust_assurance_case_set, build_rust_evidence_graph,
    build_rust_evidence_graph_analysis_request, render_rust_assurance_case_set,
    render_rust_assurance_case_set_json, render_rust_evidence_graph,
    render_rust_evidence_graph_analysis_request, render_rust_evidence_graph_analysis_request_json,
    render_rust_evidence_graph_json,
};
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(super) fn run_evidence(args: impl IntoIterator<Item = OsString>) -> Result<ExitCode, String> {
    let options = EvidenceOptions::parse(args)?;
    if options.help {
        print_evidence_help();
        return Ok(ExitCode::SUCCESS);
    }
    match options.command.as_deref() {
        Some("graph") => run_evidence_graph(options),
        Some("assurance") => run_assurance_case(options),
        Some("analyze" | "analysis") => run_evidence_analysis(options),
        _ => Err("expected `asp-rust evidence <graph|assurance|analyze>`".to_owned()),
    }
}

fn run_evidence_graph(options: EvidenceOptions) -> Result<ExitCode, String> {
    if options.review_packet_json_paths.is_empty() {
        return Err("evidence graph requires at least one --review-packet-json PATH".to_owned());
    }
    let project_root = options.project_root.unwrap_or_else(|| PathBuf::from("."));
    let review_packets = read_review_packet_json_inputs(&options.review_packet_json_paths)?;
    let graph = build_rust_evidence_graph(RustEvidenceGraphInput {
        project_root,
        review_packets,
    });
    if options.json {
        println!(
            "{}",
            render_rust_evidence_graph_json(&graph)
                .map_err(|error| format!("failed to render evidence graph JSON: {error}"))?
        );
    } else {
        println!("{}", render_rust_evidence_graph(&graph));
    }
    Ok(ExitCode::SUCCESS)
}

fn run_assurance_case(options: EvidenceOptions) -> Result<ExitCode, String> {
    if options.evidence_graph_json_paths.is_empty() {
        return Err("assurance case requires at least one --evidence-graph-json PATH".to_owned());
    }
    let project_root = options.project_root.unwrap_or_else(|| PathBuf::from("."));
    let evidence_graphs = read_evidence_graph_json_inputs(&options.evidence_graph_json_paths)?;
    let case_set = build_rust_assurance_case_set(RustAssuranceCaseInput {
        project_root,
        evidence_graphs,
    });
    if options.json {
        println!(
            "{}",
            render_rust_assurance_case_set_json(&case_set)
                .map_err(|error| format!("failed to render assurance case JSON: {error}"))?
        );
    } else {
        println!("{}", render_rust_assurance_case_set(&case_set));
    }
    Ok(ExitCode::SUCCESS)
}

fn run_evidence_analysis(options: EvidenceOptions) -> Result<ExitCode, String> {
    if options.evidence_graph_json_paths.is_empty() {
        return Err("evidence analyze requires at least one --evidence-graph-json PATH".to_owned());
    }
    let project_root = options.project_root.unwrap_or_else(|| PathBuf::from("."));
    let evidence_graphs = read_evidence_graph_json_inputs(&options.evidence_graph_json_paths)?;
    let request = build_rust_evidence_graph_analysis_request(RustEvidenceGraphAnalysisInput {
        project_root,
        evidence_graphs,
    });
    if options.json {
        println!(
            "{}",
            render_rust_evidence_graph_analysis_request_json(&request).map_err(|error| {
                format!("failed to render evidence analysis request JSON: {error}")
            })?
        );
    } else {
        println!("{}", render_rust_evidence_graph_analysis_request(&request));
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Default)]
struct EvidenceOptions {
    command: Option<String>,
    project_root: Option<PathBuf>,
    review_packet_json_paths: Vec<PathBuf>,
    evidence_graph_json_paths: Vec<PathBuf>,
    json: bool,
    help: bool,
}

impl EvidenceOptions {
    fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "-h" | "--help" => options.help = true,
                "--json" => options.json = true,
                "--review-packet-json" => options
                    .review_packet_json_paths
                    .push(next_path(&mut args, "--review-packet-json")?),
                "--evidence-graph-json" => options
                    .evidence_graph_json_paths
                    .push(next_path(&mut args, "--evidence-graph-json")?),
                value if value.starts_with('-') => {
                    return Err(format!("unknown evidence option: {value}"));
                }
                value if options.command.is_none() => options.command = Some(value.to_owned()),
                _ if options.project_root.is_none() => {
                    options.project_root = Some(PathBuf::from(arg));
                }
                value => return Err(format!("unexpected evidence argument: {value}")),
            }
        }
        Ok(options)
    }
}

fn read_review_packet_json_inputs(paths: &[PathBuf]) -> Result<Vec<RustReviewPacket>, String> {
    paths
        .iter()
        .map(|path| read_review_packet_json_input(path))
        .collect::<Result<Vec<_>, _>>()
        .map(|items| items.into_iter().flatten().collect())
}

fn read_review_packet_json_input(path: &Path) -> Result<Vec<RustReviewPacket>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read review packet JSON {}: {error}",
            path.display()
        )
    })?;
    let value = serde_json::from_str::<Value>(&text).map_err(|error| {
        format!(
            "failed to parse review packet JSON {}: {error}",
            path.display()
        )
    })?;
    let values = match value {
        Value::Array(items) => items,
        value => vec![value],
    };
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| decode_review_packet_json(value, path, index))
        .collect()
}

fn decode_review_packet_json(
    value: Value,
    path: &Path,
    index: usize,
) -> Result<RustReviewPacket, String> {
    validate_review_packet_json(&value, path, index)?;
    serde_json::from_value::<RustReviewPacket>(value).map_err(|error| {
        format!(
            "failed to decode review packet JSON {} item {index}: {error}",
            path.display()
        )
    })
}

fn validate_review_packet_json(value: &Value, path: &Path, index: usize) -> Result<(), String> {
    let schema_id = value
        .get("schemaId")
        .and_then(Value::as_str)
        .unwrap_or("<missing>");
    if schema_id != RUST_REVIEW_PACKET_SCHEMA_ID {
        return Err(format!(
            "review packet JSON {} item {index} has schemaId {}; expected {}",
            path.display(),
            schema_id,
            RUST_REVIEW_PACKET_SCHEMA_ID
        ));
    }
    let protocol_id = value
        .get("protocolId")
        .and_then(Value::as_str)
        .unwrap_or("<missing>");
    if protocol_id != RUST_REVIEW_PACKET_PROTOCOL_ID {
        return Err(format!(
            "review packet JSON {} item {index} has protocolId {}; expected {}",
            path.display(),
            protocol_id,
            RUST_REVIEW_PACKET_PROTOCOL_ID
        ));
    }
    Ok(())
}

fn read_evidence_graph_json_inputs(paths: &[PathBuf]) -> Result<Vec<RustEvidenceGraph>, String> {
    paths
        .iter()
        .map(|path| read_evidence_graph_json_input(path))
        .collect::<Result<Vec<_>, _>>()
        .map(|items| items.into_iter().flatten().collect())
}

fn read_evidence_graph_json_input(path: &Path) -> Result<Vec<RustEvidenceGraph>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read evidence graph JSON {}: {error}",
            path.display()
        )
    })?;
    let value = serde_json::from_str::<Value>(&text).map_err(|error| {
        format!(
            "failed to parse evidence graph JSON {}: {error}",
            path.display()
        )
    })?;
    let values = match value {
        Value::Array(items) => items,
        value => vec![value],
    };
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| decode_evidence_graph_json(value, path, index))
        .collect()
}

fn decode_evidence_graph_json(
    value: Value,
    path: &Path,
    index: usize,
) -> Result<RustEvidenceGraph, String> {
    validate_evidence_graph_json(&value, path, index)?;
    serde_json::from_value::<RustEvidenceGraph>(value).map_err(|error| {
        format!(
            "failed to decode evidence graph JSON {} item {index}: {error}",
            path.display()
        )
    })
}

fn validate_evidence_graph_json(value: &Value, path: &Path, index: usize) -> Result<(), String> {
    let schema_id = value
        .get("schemaId")
        .and_then(Value::as_str)
        .unwrap_or("<missing>");
    if schema_id != RUST_EVIDENCE_GRAPH_SCHEMA_ID {
        return Err(format!(
            "evidence graph JSON {} item {index} has schemaId {}; expected {}",
            path.display(),
            schema_id,
            RUST_EVIDENCE_GRAPH_SCHEMA_ID
        ));
    }
    let protocol_id = value
        .get("protocolId")
        .and_then(Value::as_str)
        .unwrap_or("<missing>");
    if protocol_id != RUST_EVIDENCE_GRAPH_PROTOCOL_ID {
        return Err(format!(
            "evidence graph JSON {} item {index} has protocolId {}; expected {}",
            path.display(),
            protocol_id,
            RUST_EVIDENCE_GRAPH_PROTOCOL_ID
        ));
    }
    Ok(())
}

fn next_path(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{option} requires a path"))
}

fn print_evidence_help() {
    println!(
        "asp-rust evidence graph --review-packet-json PATH [--json] [PROJECT_ROOT]\n\
         asp-rust evidence assurance --evidence-graph-json PATH [--json] [PROJECT_ROOT]\n\
         asp-rust evidence analyze --evidence-graph-json PATH [--json] [PROJECT_ROOT]\n\n\
         Builds portable evidence graph, assurance case, and graph-turbo analysis request packets."
    );
}
