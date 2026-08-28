use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    RUST_BEHAVIOR_SNAPSHOT_PROTOCOL_ID, RUST_BEHAVIOR_SNAPSHOT_SCHEMA_ID,
    RUST_DETERMINISM_READINESS_PROTOCOL_ID, RUST_DETERMINISM_READINESS_SCHEMA_ID,
    RUST_FORMAL_PROOF_PILOT_PROTOCOL_ID, RUST_FORMAL_PROOF_PILOT_SCHEMA_ID,
    RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_ID, RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID,
    RustBehaviorSnapshot, RustDeterminismReadiness, RustFormalProofPilot, RustHarnessRunScope,
    RustReviewPacketInput, RustReviewPacketWaiver, RustVerificationExecutionReceipt,
    build_rust_review_packet, render_rust_review_packet, render_rust_review_packet_json,
    run_rust_project_harness_for_scope,
};

pub(super) fn run_review(args: impl IntoIterator<Item = OsString>) -> Result<ExitCode, String> {
    let options = ReviewOptions::parse(args)?;
    if options.help {
        print_review_help();
        return Ok(ExitCode::SUCCESS);
    }
    if options.command.as_deref() != Some("packet") {
        return Err("expected `asp-rust review packet`".to_owned());
    }

    let project_root = options.project_root.unwrap_or_else(|| PathBuf::from("."));
    let report =
        run_rust_project_harness_for_scope(&project_root, RustHarnessRunScope::ProjectWorkspace)?;
    let receipts = read_packet_json_inputs::<RustVerificationExecutionReceipt>(
        &options.receipt_json_paths,
        "receipt",
    )?;
    let behavior_snapshots = read_packet_json_inputs::<RustBehaviorSnapshot>(
        &options.behavior_json_paths,
        "behavior snapshot",
    )?;
    let determinism_readiness = read_packet_json_inputs::<RustDeterminismReadiness>(
        &options.determinism_json_paths,
        "determinism readiness",
    )?;
    let proof_pilots =
        read_packet_json_inputs::<RustFormalProofPilot>(&options.proof_json_paths, "proof pilot")?;
    let waivers =
        read_json_inputs::<RustReviewPacketWaiver>(&options.waiver_json_paths, "review waiver")?;

    let packet = build_rust_review_packet(RustReviewPacketInput {
        project_root,
        report,
        receipts,
        behavior_snapshots,
        determinism_readiness,
        proof_pilots,
        waivers,
    });

    if options.json {
        println!("{}", render_rust_review_packet_json(&packet)?);
    } else {
        println!("{}", render_rust_review_packet(&packet));
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Default)]
struct ReviewOptions {
    command: Option<String>,
    project_root: Option<PathBuf>,
    receipt_json_paths: Vec<PathBuf>,
    behavior_json_paths: Vec<PathBuf>,
    determinism_json_paths: Vec<PathBuf>,
    proof_json_paths: Vec<PathBuf>,
    waiver_json_paths: Vec<PathBuf>,
    json: bool,
    help: bool,
}

impl ReviewOptions {
    fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let text = os_string_to_string(arg)?;
            match text.as_str() {
                "--help" | "-h" => options.help = true,
                "--json" => options.json = true,
                "--receipt-json" => options
                    .receipt_json_paths
                    .push(next_path(&mut args, "--receipt-json")?),
                "--behavior-json" => options
                    .behavior_json_paths
                    .push(next_path(&mut args, "--behavior-json")?),
                "--determinism-json" => options
                    .determinism_json_paths
                    .push(next_path(&mut args, "--determinism-json")?),
                "--proof-json" => options
                    .proof_json_paths
                    .push(next_path(&mut args, "--proof-json")?),
                "--waiver-json" => options
                    .waiver_json_paths
                    .push(next_path(&mut args, "--waiver-json")?),
                other if other.starts_with('-') => {
                    return Err(format!("unknown review option: {other}"));
                }
                other if options.command.is_none() => options.command = Some(other.to_owned()),
                other if options.project_root.is_none() => {
                    options.project_root = Some(PathBuf::from(other));
                }
                other => return Err(format!("unexpected review argument: {other}")),
            }
        }
        Ok(options)
    }
}

fn read_json_inputs<T>(paths: &[PathBuf], label: &str) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    paths
        .iter()
        .map(|path| read_json_input(path, label))
        .collect::<Result<Vec<_>, _>>()
        .map(|items| items.into_iter().flatten().collect())
}

fn read_packet_json_inputs<T>(paths: &[PathBuf], label: &str) -> Result<Vec<T>, String>
where
    T: DeserializeOwned + ReviewInputPacket,
{
    paths
        .iter()
        .map(|path| read_packet_json_input(path, label))
        .collect::<Result<Vec<_>, _>>()
        .map(|items| items.into_iter().flatten().collect())
}

fn read_json_input<T>(path: &Path, label: &str) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {label} JSON {}: {error}", path.display()))?;
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("failed to parse {label} JSON {}: {error}", path.display()))?;
    match value {
        Value::Array(items) => {
            serde_json::from_value::<Vec<T>>(Value::Array(items)).map_err(|error| {
                format!(
                    "failed to decode {label} JSON array {}: {error}",
                    path.display()
                )
            })
        }
        value => serde_json::from_value::<T>(value)
            .map(|item| vec![item])
            .map_err(|error| format!("failed to decode {label} JSON {}: {error}", path.display())),
    }
}

fn read_packet_json_input<T>(path: &Path, label: &str) -> Result<Vec<T>, String>
where
    T: DeserializeOwned + ReviewInputPacket,
{
    let items = read_json_input::<T>(path, label)?;
    for (index, item) in items.iter().enumerate() {
        validate_review_input_packet(item, path, label, index)?;
    }
    Ok(items)
}

fn validate_review_input_packet<T>(
    item: &T,
    path: &Path,
    label: &str,
    index: usize,
) -> Result<(), String>
where
    T: ReviewInputPacket,
{
    if item.schema_id() != T::expected_schema_id() {
        return Err(format!(
            "{label} JSON {} item {index} has schemaId {}; expected {}",
            path.display(),
            item.schema_id(),
            T::expected_schema_id()
        ));
    }
    if item.protocol_id() != T::expected_protocol_id() {
        return Err(format!(
            "{label} JSON {} item {index} has protocolId {}; expected {}",
            path.display(),
            item.protocol_id(),
            T::expected_protocol_id()
        ));
    }
    Ok(())
}

trait ReviewInputPacket {
    fn schema_id(&self) -> &str;

    fn protocol_id(&self) -> &str;

    fn expected_schema_id() -> &'static str;

    fn expected_protocol_id() -> &'static str;
}

impl ReviewInputPacket for RustVerificationExecutionReceipt {
    fn schema_id(&self) -> &str {
        self.schema_id.0.as_str()
    }

    fn protocol_id(&self) -> &str {
        self.protocol_id.0.as_str()
    }

    fn expected_schema_id() -> &'static str {
        RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID
    }

    fn expected_protocol_id() -> &'static str {
        RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_ID
    }
}

impl ReviewInputPacket for RustBehaviorSnapshot {
    fn schema_id(&self) -> &str {
        self.schema_id.0.as_str()
    }

    fn protocol_id(&self) -> &str {
        self.protocol_id.0.as_str()
    }

    fn expected_schema_id() -> &'static str {
        RUST_BEHAVIOR_SNAPSHOT_SCHEMA_ID
    }

    fn expected_protocol_id() -> &'static str {
        RUST_BEHAVIOR_SNAPSHOT_PROTOCOL_ID
    }
}

impl ReviewInputPacket for RustDeterminismReadiness {
    fn schema_id(&self) -> &str {
        self.schema_id.0.as_str()
    }

    fn protocol_id(&self) -> &str {
        self.protocol_id.0.as_str()
    }

    fn expected_schema_id() -> &'static str {
        RUST_DETERMINISM_READINESS_SCHEMA_ID
    }

    fn expected_protocol_id() -> &'static str {
        RUST_DETERMINISM_READINESS_PROTOCOL_ID
    }
}

impl ReviewInputPacket for RustFormalProofPilot {
    fn schema_id(&self) -> &str {
        self.schema_id.0.as_str()
    }

    fn protocol_id(&self) -> &str {
        self.protocol_id.0.as_str()
    }

    fn expected_schema_id() -> &'static str {
        RUST_FORMAL_PROOF_PILOT_SCHEMA_ID
    }

    fn expected_protocol_id() -> &'static str {
        RUST_FORMAL_PROOF_PILOT_PROTOCOL_ID
    }
}

fn next_path(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{option} requires a path"))
}

fn os_string_to_string(value: OsString) -> Result<String, String> {
    value
        .into_string()
        .map_err(|value| format!("non-utf8 argument: {value:?}"))
}

fn print_review_help() {
    println!(
        "asp-rust review packet [--receipt-json PATH] [--behavior-json PATH] [--determinism-json PATH] [--proof-json PATH] [--waiver-json PATH] [--json] [PROJECT_ROOT]\n\n\
         Builds a reviewer-first packet from invariant candidates and new evidence API packets."
    );
}
