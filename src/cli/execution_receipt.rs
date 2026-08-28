use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::str::FromStr;
use std::time::Instant;

use crate::{
    RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_ID,
    RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_VERSION,
    RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID,
    RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_VERSION, RustVerificationExecutionAdapterId,
    RustVerificationExecutionCommand, RustVerificationExecutionDurationMs,
    RustVerificationExecutionExitCode, RustVerificationExecutionLanguageId,
    RustVerificationExecutionNamespace, RustVerificationExecutionObservation,
    RustVerificationExecutionObservationKind, RustVerificationExecutionObservationMessage,
    RustVerificationExecutionProducer, RustVerificationExecutionProject,
    RustVerificationExecutionProtocolId, RustVerificationExecutionProtocolVersion,
    RustVerificationExecutionProviderId, RustVerificationExecutionReceipt,
    RustVerificationExecutionReceiptId, RustVerificationExecutionSchemaId,
    RustVerificationExecutionSchemaVersion, RustVerificationExecutionStatus,
    RustVerificationExecutionSummary, RustVerificationToolAdapter,
};

pub(super) fn run_receipt(args: impl IntoIterator<Item = OsString>) -> Result<ExitCode, String> {
    let options = ReceiptOptions::parse(args)?;
    if options.help {
        print_receipt_help();
        return Ok(ExitCode::SUCCESS);
    }
    let adapter = options
        .adapter
        .ok_or_else(|| "missing receipt adapter".to_string())?;
    let project_root = options.project_root()?;
    let mut command = options.command_for_adapter(adapter)?;
    command.workdir = Some(project_root.clone());
    let receipt = if options.dry_run {
        skipped_receipt(adapter, &project_root, command)
    } else {
        run_adapter(adapter, &project_root, command)
    };
    print_receipt(&receipt, options.json)?;
    Ok(exit_code_for_receipt(&receipt))
}

#[derive(Debug, Default)]
struct ReceiptOptions {
    adapter: Option<RustVerificationToolAdapter>,
    dry_run: bool,
    json: bool,
    help: bool,
    case_filter: Option<String>,
    fuzz_target: Option<String>,
    fuzz_runs: Option<String>,
    proof_harness: Option<String>,
    proof_file: Option<String>,
    paths: Vec<PathBuf>,
}

impl ReceiptOptions {
    fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let Some(value) = arg.to_str() else {
                return Err("receipt arguments must be UTF-8".to_string());
            };
            match value {
                "--help" | "-h" => options.help = true,
                "--dry-run" => options.dry_run = true,
                "--json" => options.json = true,
                "--case-filter" => {
                    options.case_filter = Some(next_option_value(&mut args, "--case-filter")?);
                }
                "--target" => {
                    options.fuzz_target = Some(next_option_value(&mut args, "--target")?);
                }
                "--runs" => {
                    options.fuzz_runs = Some(next_option_value(&mut args, "--runs")?);
                }
                "--harness" => {
                    options.proof_harness = Some(next_option_value(&mut args, "--harness")?);
                }
                "--file" => {
                    options.proof_file = Some(next_option_value(&mut args, "--file")?);
                }
                option if option.starts_with('-') => {
                    return Err(format!("unknown receipt option: {option}"));
                }
                value if options.adapter.is_none() => {
                    options.adapter = Some(RustVerificationToolAdapter::from_str(value)?);
                }
                path => options.paths.push(PathBuf::from(path)),
            }
        }
        Ok(options)
    }

    fn command_for_adapter(
        &self,
        adapter: RustVerificationToolAdapter,
    ) -> Result<RustVerificationExecutionCommand, String> {
        let mut command = adapter.default_command();
        match adapter {
            RustVerificationToolAdapter::Proptest => {
                if let Some(filter) = &self.case_filter {
                    command.argv = vec![
                        "cargo".to_string(),
                        "test".to_string(),
                        filter.clone(),
                        "--".to_string(),
                        "--nocapture".to_string(),
                    ];
                }
            }
            RustVerificationToolAdapter::CargoFuzz => {
                if let Some(target) = &self.fuzz_target {
                    command.argv = vec![
                        "cargo".to_string(),
                        "fuzz".to_string(),
                        "run".to_string(),
                        target.clone(),
                    ];
                    if let Some(runs) = &self.fuzz_runs {
                        command.argv.push("--".to_string());
                        command.argv.push(format!("-runs={runs}"));
                    }
                } else if !self.dry_run {
                    return Err(
                        "cargo-fuzz receipt requires --target <name> unless --dry-run is set"
                            .to_string(),
                    );
                }
            }
            RustVerificationToolAdapter::Kani => {
                if let Some(harness) = &self.proof_harness {
                    command.argv.push("--harness".to_string());
                    command.argv.push(harness.clone());
                }
            }
            RustVerificationToolAdapter::Verus => {
                if let Some(file) = &self.proof_file {
                    command.argv.push(file.clone());
                } else if !self.dry_run {
                    return Err(
                        "verus receipt requires --file <path> unless --dry-run is set".to_string(),
                    );
                }
            }
            _ => {}
        }
        Ok(command)
    }

    fn project_root(&self) -> Result<PathBuf, String> {
        self.paths
            .last()
            .cloned()
            .map(Ok)
            .unwrap_or_else(std::env::current_dir)
            .map_err(|error| format!("failed to resolve project root: {error}"))
    }
}

fn next_option_value(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<String, String> {
    let value = args
        .next()
        .ok_or_else(|| format!("{option} requires a value"))?;
    value
        .into_string()
        .map_err(|_| format!("{option} value must be UTF-8"))
}

fn run_adapter(
    adapter: RustVerificationToolAdapter,
    project_root: &Path,
    command: RustVerificationExecutionCommand,
) -> RustVerificationExecutionReceipt {
    let argv = command.argv.clone();
    let start = Instant::now();
    let output = argv
        .first()
        .map(|program| {
            Command::new(program)
                .args(argv.iter().skip(1))
                .current_dir(project_root)
                .output()
        })
        .unwrap_or_else(|| Err(std::io::Error::other("adapter command argv is empty")));
    let duration_ms = start.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    match output {
        Ok(output) => {
            let exit_code = output.status.code();
            let status = match exit_code {
                Some(0) => RustVerificationExecutionStatus::Passed,
                Some(_) => RustVerificationExecutionStatus::Failed,
                None => RustVerificationExecutionStatus::Error,
            };
            let mut observations = vec![observation(
                RustVerificationExecutionObservationKind::ExitStatus,
                exit_code.map_or_else(
                    || "process terminated without an exit code".to_string(),
                    |code| format!("process exited with code {code}"),
                ),
            )];
            push_output_observation(
                &mut observations,
                RustVerificationExecutionObservationKind::Stdout,
                &output.stdout,
            );
            push_output_observation(
                &mut observations,
                RustVerificationExecutionObservationKind::Stderr,
                &output.stderr,
            );
            receipt(ExecutionReceiptInput {
                adapter,
                project_root,
                command,
                status,
                exit_code,
                duration_ms: Some(duration_ms),
                summary: summary_for_status(adapter, status),
                observations,
            })
        }
        Err(error) => receipt(ExecutionReceiptInput {
            adapter,
            project_root,
            command,
            status: RustVerificationExecutionStatus::Error,
            exit_code: None,
            duration_ms: Some(duration_ms),
            summary: format!("{} adapter could not run", adapter_name(adapter)),
            observations: vec![observation(
                RustVerificationExecutionObservationKind::Note,
                error.to_string(),
            )],
        }),
    }
}

fn skipped_receipt(
    adapter: RustVerificationToolAdapter,
    project_root: &Path,
    command: RustVerificationExecutionCommand,
) -> RustVerificationExecutionReceipt {
    receipt(ExecutionReceiptInput {
        adapter,
        project_root,
        command,
        status: RustVerificationExecutionStatus::Skipped,
        exit_code: None,
        duration_ms: None,
        summary: format!(
            "{} adapter dry-run skipped execution",
            adapter_name(adapter)
        ),
        observations: vec![observation(
            RustVerificationExecutionObservationKind::Note,
            "--dry-run requested; adapter command was not executed",
        )],
    })
}

struct ExecutionReceiptInput<'a> {
    adapter: RustVerificationToolAdapter,
    project_root: &'a Path,
    command: RustVerificationExecutionCommand,
    status: RustVerificationExecutionStatus,
    exit_code: Option<i32>,
    duration_ms: Option<u64>,
    summary: String,
    observations: Vec<RustVerificationExecutionObservation>,
}

fn receipt(input: ExecutionReceiptInput<'_>) -> RustVerificationExecutionReceipt {
    let adapter_id = input.adapter.adapter_id();
    RustVerificationExecutionReceipt {
        schema_id: RustVerificationExecutionSchemaId(
            RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID.to_string(),
        ),
        schema_version: RustVerificationExecutionSchemaVersion(
            RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_VERSION.to_string(),
        ),
        protocol_id: RustVerificationExecutionProtocolId(
            RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_ID.to_string(),
        ),
        protocol_version: RustVerificationExecutionProtocolVersion(
            RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_VERSION.to_string(),
        ),
        receipt_id: receipt_id(input.adapter, input.status, &adapter_id),
        producer: RustVerificationExecutionProducer {
            language_id: RustVerificationExecutionLanguageId("rust".to_string()),
            provider_id: RustVerificationExecutionProviderId("asp-rust".to_string()),
            adapter_id,
            namespace: RustVerificationExecutionNamespace(
                "agent.semantic-protocols.languages.rust.asp-rust".to_string(),
            ),
        },
        project: Some(RustVerificationExecutionProject {
            name: None,
            workdir: Some(input.project_root.to_path_buf()),
            package: None,
        }),
        tool: input.adapter.tool(),
        status: input.status,
        command: input.command,
        exit_code: input.exit_code.map(RustVerificationExecutionExitCode),
        duration_ms: input.duration_ms.map(RustVerificationExecutionDurationMs),
        observed_at: None,
        summary: RustVerificationExecutionSummary(input.summary),
        observations: input.observations,
        candidate_ids: Vec::new(),
        task_fingerprints: Vec::new(),
        artifacts: Vec::new(),
        fields: BTreeMap::new(),
    }
}

fn receipt_id(
    adapter: RustVerificationToolAdapter,
    status: RustVerificationExecutionStatus,
    adapter_id: &RustVerificationExecutionAdapterId,
) -> RustVerificationExecutionReceiptId {
    RustVerificationExecutionReceiptId(format!(
        "{}:{}:{}",
        adapter_id.0,
        adapter_name(adapter),
        status_name(status)
    ))
}

fn observation(
    kind: RustVerificationExecutionObservationKind,
    message: impl Into<String>,
) -> RustVerificationExecutionObservation {
    RustVerificationExecutionObservation {
        kind,
        message: RustVerificationExecutionObservationMessage(message.into()),
        path: None,
        line: None,
        fields: BTreeMap::new(),
    }
}

fn push_output_observation(
    observations: &mut Vec<RustVerificationExecutionObservation>,
    kind: RustVerificationExecutionObservationKind,
    bytes: &[u8],
) {
    if let Some(message) = compact_output(bytes) {
        observations.push(observation(kind, message));
    }
}

fn compact_output(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let line = text.lines().find(|line| !line.trim().is_empty())?.trim();
    let mut compact = line.chars().take(240).collect::<String>();
    if line.chars().count() > 240 {
        compact.push_str("...");
    }
    Some(compact)
}

fn summary_for_status(
    adapter: RustVerificationToolAdapter,
    status: RustVerificationExecutionStatus,
) -> String {
    format!("{} adapter {}", adapter_name(adapter), status_name(status))
}

fn exit_code_for_receipt(receipt: &RustVerificationExecutionReceipt) -> ExitCode {
    match receipt.status {
        RustVerificationExecutionStatus::Passed | RustVerificationExecutionStatus::Skipped => {
            ExitCode::SUCCESS
        }
        RustVerificationExecutionStatus::Failed | RustVerificationExecutionStatus::Error => {
            ExitCode::FAILURE
        }
    }
}

fn print_receipt(receipt: &RustVerificationExecutionReceipt, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(receipt)
                .map_err(|error| format!("failed to render receipt JSON: {error}"))?
        );
    } else {
        println!(
            "[receipt] tool={} status={} summary=\"{}\"",
            adapter_name(adapter_for_tool(receipt)),
            status_name(receipt.status),
            receipt.summary.0
        );
    }
    Ok(())
}

fn adapter_for_tool(receipt: &RustVerificationExecutionReceipt) -> RustVerificationToolAdapter {
    match receipt.tool {
        crate::RustVerificationExecutionTool::CargoCheck => RustVerificationToolAdapter::CargoCheck,
        crate::RustVerificationExecutionTool::CargoTest => RustVerificationToolAdapter::CargoTest,
        crate::RustVerificationExecutionTool::Clippy => RustVerificationToolAdapter::Clippy,
        crate::RustVerificationExecutionTool::ExpectTest => RustVerificationToolAdapter::ExpectTest,
        crate::RustVerificationExecutionTool::Proptest => RustVerificationToolAdapter::Proptest,
        crate::RustVerificationExecutionTool::CargoFuzz => RustVerificationToolAdapter::CargoFuzz,
        crate::RustVerificationExecutionTool::Kani => RustVerificationToolAdapter::Kani,
        crate::RustVerificationExecutionTool::Creusot => RustVerificationToolAdapter::Creusot,
        crate::RustVerificationExecutionTool::Verus => RustVerificationToolAdapter::Verus,
    }
}

fn adapter_name(adapter: RustVerificationToolAdapter) -> &'static str {
    match adapter {
        RustVerificationToolAdapter::CargoCheck => "cargo-check",
        RustVerificationToolAdapter::CargoTest => "cargo-test",
        RustVerificationToolAdapter::Clippy => "clippy",
        RustVerificationToolAdapter::ExpectTest => "expect-test",
        RustVerificationToolAdapter::Proptest => "proptest",
        RustVerificationToolAdapter::CargoFuzz => "cargo-fuzz",
        RustVerificationToolAdapter::Kani => "kani",
        RustVerificationToolAdapter::Creusot => "creusot",
        RustVerificationToolAdapter::Verus => "verus",
    }
}

fn status_name(status: RustVerificationExecutionStatus) -> &'static str {
    match status {
        RustVerificationExecutionStatus::Passed => "passed",
        RustVerificationExecutionStatus::Failed => "failed",
        RustVerificationExecutionStatus::Skipped => "skipped",
        RustVerificationExecutionStatus::Error => "error",
    }
}

fn print_receipt_help() {
    println!(
        "asp-rust receipt <adapter> [--dry-run] [--json] [PROJECT_ROOT]\n\
asp-rust receipt proptest [--case-filter TEST_FILTER] [--dry-run] [--json] [PROJECT_ROOT]\n\
asp-rust receipt cargo-fuzz --target TARGET [--runs N] [--dry-run] [--json] [PROJECT_ROOT]\n\
asp-rust receipt kani [--harness NAME] [--dry-run] [--json] [PROJECT_ROOT]\n\
asp-rust receipt verus --file PATH [--dry-run] [--json] [PROJECT_ROOT]\n\n\
Adapters: cargo-check, cargo-test, clippy, expect-test, proptest, cargo-fuzz, kani, creusot, verus.\n\
Use --dry-run to emit a skipped receipt without executing the adapter command.\n\
Use --json to emit the semantic-verification-receipt JSON contract."
    );
}
