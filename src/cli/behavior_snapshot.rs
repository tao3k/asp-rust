use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use crate::{
    RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID, RustBehaviorSnapshot, RustBehaviorSnapshotId,
    RustBehaviorSnapshotInput, RustBehaviorSnapshotObservation,
    RustBehaviorSnapshotObservationKind, RustBehaviorSnapshotObservationMessage,
    RustBehaviorSnapshotStatus, RustBehaviorSnapshotSubject, RustBehaviorSnapshotSubjectKind,
    RustBehaviorSnapshotSymbol, RustBehaviorSnapshotValue, RustInvariantId,
    RustVerificationExecutionReceipt, RustVerificationExecutionStatus,
    RustVerificationExecutionTool,
};

pub(super) fn run_behavior(args: impl IntoIterator<Item = OsString>) -> Result<ExitCode, String> {
    let options = BehaviorOptions::parse(args)?;
    if options.help {
        print_behavior_help();
        return Ok(ExitCode::SUCCESS);
    }
    if options.command.as_deref() != Some("snapshot") {
        return Err("unknown behavior command: expected snapshot".to_string());
    }
    let snapshot = options.snapshot()?;
    print_snapshot(&snapshot, options.json)?;
    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Default)]
struct BehaviorOptions {
    command: Option<String>,
    kind: Option<RustBehaviorSnapshotSubjectKind>,
    path: Option<PathBuf>,
    symbol: Option<String>,
    status: Option<RustBehaviorSnapshotStatus>,
    expected: Option<String>,
    actual: Option<String>,
    diff: Option<String>,
    receipt_ids: Vec<String>,
    receipt_json_paths: Vec<PathBuf>,
    candidate_ids: Vec<RustInvariantId>,
    json: bool,
    help: bool,
}

impl BehaviorOptions {
    fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let Some(value) = arg.to_str() else {
                return Err("behavior arguments must be UTF-8".to_string());
            };
            match value {
                "--help" | "-h" => options.help = true,
                "--json" => options.json = true,
                "--kind" => {
                    options.kind = Some(RustBehaviorSnapshotSubjectKind::from_str(
                        &next_option_value(&mut args, "--kind")?,
                    )?);
                }
                "--path" => {
                    options.path = Some(PathBuf::from(next_option_value(&mut args, "--path")?));
                }
                "--symbol" => {
                    options.symbol = Some(next_option_value(&mut args, "--symbol")?);
                }
                "--status" => {
                    options.status = Some(RustBehaviorSnapshotStatus::from_str(
                        &next_option_value(&mut args, "--status")?,
                    )?);
                }
                "--expected" => {
                    options.expected = Some(next_option_value(&mut args, "--expected")?);
                }
                "--actual" => {
                    options.actual = Some(next_option_value(&mut args, "--actual")?);
                }
                "--diff" => {
                    options.diff = Some(next_option_value(&mut args, "--diff")?);
                }
                "--receipt-id" => {
                    options
                        .receipt_ids
                        .push(next_option_value(&mut args, "--receipt-id")?);
                }
                "--receipt-json" => {
                    options
                        .receipt_json_paths
                        .push(PathBuf::from(next_option_value(
                            &mut args,
                            "--receipt-json",
                        )?));
                }
                "--candidate-id" => {
                    options
                        .candidate_ids
                        .push(RustInvariantId(next_option_value(
                            &mut args,
                            "--candidate-id",
                        )?));
                }
                option if option.starts_with('-') => {
                    return Err(format!("unknown behavior option: {option}"));
                }
                command if options.command.is_none() => {
                    options.command = Some(command.to_string());
                }
                other => return Err(format!("unexpected behavior argument: {other}")),
            }
        }
        Ok(options)
    }

    fn snapshot(&self) -> Result<RustBehaviorSnapshot, String> {
        let linked_receipts = self.load_receipts()?;
        let path = self.path.clone().ok_or_else(|| {
            "behavior snapshot requires --path <project-relative-path>".to_string()
        })?;
        if path.is_absolute() {
            return Err("behavior snapshot --path must be project-relative".to_string());
        }
        let kind = self
            .kind
            .unwrap_or(RustBehaviorSnapshotSubjectKind::PublicApi);
        let status = self
            .status
            .unwrap_or_else(|| self.infer_status(&linked_receipts));
        let mut observations = vec![RustBehaviorSnapshotObservation {
            kind: observation_kind_for(status, self.diff.as_ref()),
            message: RustBehaviorSnapshotObservationMessage(message_for(status).to_string()),
            path: None,
            line: None,
            fields: BTreeMap::new(),
        }];
        observations.extend(linked_receipts.iter().map(receipt_observation));
        let mut snapshot = RustBehaviorSnapshot::new(RustBehaviorSnapshotInput {
            snapshot_id: RustBehaviorSnapshotId(snapshot_id(kind, &path, self.symbol.as_deref())),
            subject: RustBehaviorSnapshotSubject {
                kind,
                path,
                symbol: self.symbol.clone().map(RustBehaviorSnapshotSymbol),
                command: Vec::new(),
                fields: BTreeMap::new(),
            },
            status,
            observations,
            expected: self
                .expected
                .as_ref()
                .map(|value| RustBehaviorSnapshotValue::text(value.clone())),
            actual: self
                .actual
                .as_ref()
                .map(|value| RustBehaviorSnapshotValue::text(value.clone())),
            diff: self
                .diff
                .as_ref()
                .map(|value| RustBehaviorSnapshotValue::text(value.clone())),
        });
        snapshot.receipt_ids.clone_from(&self.receipt_ids);
        snapshot.receipt_ids.extend(
            linked_receipts
                .iter()
                .map(|receipt| receipt.receipt_id.0.clone()),
        );
        snapshot.candidate_ids.clone_from(&self.candidate_ids);
        Ok(snapshot)
    }

    fn load_receipts(&self) -> Result<Vec<RustVerificationExecutionReceipt>, String> {
        self.receipt_json_paths
            .iter()
            .map(|path| {
                let text = fs::read_to_string(path).map_err(|error| {
                    format!("failed to read receipt JSON {}: {error}", path.display())
                })?;
                let receipt = serde_json::from_str::<RustVerificationExecutionReceipt>(&text)
                    .map_err(|error| {
                        format!("failed to parse receipt JSON {}: {error}", path.display())
                    })?;
                if receipt.schema_id.0 != RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID {
                    return Err(format!(
                        "receipt JSON {} has unsupported schemaId {}",
                        path.display(),
                        receipt.schema_id.0
                    ));
                }
                Ok(receipt)
            })
            .collect()
    }

    fn infer_status(
        &self,
        linked_receipts: &[RustVerificationExecutionReceipt],
    ) -> RustBehaviorSnapshotStatus {
        if let Some(status) = infer_status_from_receipts(linked_receipts) {
            return status;
        }
        match (&self.expected, &self.actual, &self.diff) {
            (_, _, Some(_)) => RustBehaviorSnapshotStatus::Changed,
            (Some(expected), Some(actual), _) if expected != actual => {
                RustBehaviorSnapshotStatus::Changed
            }
            (Some(_), Some(_), _) => RustBehaviorSnapshotStatus::Matched,
            _ => RustBehaviorSnapshotStatus::Skipped,
        }
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

fn snapshot_id(
    kind: RustBehaviorSnapshotSubjectKind,
    path: &std::path::Path,
    symbol: Option<&str>,
) -> String {
    let mut id = format!(
        "rust.behavior.{}.{}",
        subject_kind_name(kind),
        path.display().to_string().replace(['/', '\\'], ".")
    );
    if let Some(symbol) = symbol {
        id.push('.');
        id.push_str(&symbol.replace("::", "."));
    }
    id
}

fn observation_kind_for(
    status: RustBehaviorSnapshotStatus,
    diff: Option<&String>,
) -> RustBehaviorSnapshotObservationKind {
    if diff.is_some() {
        return RustBehaviorSnapshotObservationKind::Diff;
    }
    match status {
        RustBehaviorSnapshotStatus::Matched | RustBehaviorSnapshotStatus::Changed => {
            RustBehaviorSnapshotObservationKind::Snapshot
        }
        RustBehaviorSnapshotStatus::Missing
        | RustBehaviorSnapshotStatus::Skipped
        | RustBehaviorSnapshotStatus::Error => RustBehaviorSnapshotObservationKind::Note,
    }
}

fn infer_status_from_receipts(
    linked_receipts: &[RustVerificationExecutionReceipt],
) -> Option<RustBehaviorSnapshotStatus> {
    if linked_receipts
        .iter()
        .any(|receipt| receipt.status == RustVerificationExecutionStatus::Error)
    {
        return Some(RustBehaviorSnapshotStatus::Error);
    }
    if linked_receipts
        .iter()
        .any(|receipt| receipt.status == RustVerificationExecutionStatus::Failed)
    {
        return Some(RustBehaviorSnapshotStatus::Changed);
    }
    if linked_receipts
        .iter()
        .any(|receipt| receipt.status == RustVerificationExecutionStatus::Skipped)
    {
        return Some(RustBehaviorSnapshotStatus::Skipped);
    }
    linked_receipts
        .iter()
        .any(|receipt| receipt.status == RustVerificationExecutionStatus::Passed)
        .then_some(RustBehaviorSnapshotStatus::Matched)
}

fn receipt_observation(
    receipt: &RustVerificationExecutionReceipt,
) -> RustBehaviorSnapshotObservation {
    let mut fields = BTreeMap::new();
    fields.insert("receiptId".to_string(), receipt.receipt_id.0.clone());
    fields.insert(
        "tool".to_string(),
        receipt_tool_name(receipt.tool).to_string(),
    );
    RustBehaviorSnapshotObservation {
        kind: match receipt.tool {
            RustVerificationExecutionTool::ExpectTest => {
                RustBehaviorSnapshotObservationKind::Snapshot
            }
            _ => RustBehaviorSnapshotObservationKind::Note,
        },
        message: RustBehaviorSnapshotObservationMessage(format!(
            "linked {} receipt {}",
            receipt_tool_name(receipt.tool),
            receipt_status_name(receipt.status)
        )),
        path: None,
        line: None,
        fields,
    }
}

fn receipt_tool_name(tool: RustVerificationExecutionTool) -> &'static str {
    match tool {
        RustVerificationExecutionTool::CargoCheck => "cargo-check",
        RustVerificationExecutionTool::CargoTest => "cargo-test",
        RustVerificationExecutionTool::Clippy => "clippy",
        RustVerificationExecutionTool::ExpectTest => "expect-test",
        RustVerificationExecutionTool::Proptest => "proptest",
        RustVerificationExecutionTool::CargoFuzz => "cargo-fuzz",
        RustVerificationExecutionTool::Kani => "kani",
        RustVerificationExecutionTool::Creusot => "creusot",
        RustVerificationExecutionTool::Verus => "verus",
    }
}

fn receipt_status_name(status: RustVerificationExecutionStatus) -> &'static str {
    match status {
        RustVerificationExecutionStatus::Passed => "passed",
        RustVerificationExecutionStatus::Failed => "failed",
        RustVerificationExecutionStatus::Skipped => "skipped",
        RustVerificationExecutionStatus::Error => "error",
    }
}

fn message_for(status: RustBehaviorSnapshotStatus) -> &'static str {
    match status {
        RustBehaviorSnapshotStatus::Matched => "behavior snapshot matched",
        RustBehaviorSnapshotStatus::Changed => "behavior snapshot changed",
        RustBehaviorSnapshotStatus::Missing => "behavior snapshot missing",
        RustBehaviorSnapshotStatus::Skipped => "behavior snapshot skipped",
        RustBehaviorSnapshotStatus::Error => "behavior snapshot errored",
    }
}

fn print_snapshot(snapshot: &RustBehaviorSnapshot, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(snapshot)
                .map_err(|error| format!("failed to render behavior snapshot JSON: {error}"))?
        );
    } else {
        println!(
            "[behavior] subject={} path={} status={}",
            subject_kind_name(snapshot.subject.kind),
            snapshot.subject.path.display(),
            status_name(snapshot.status)
        );
    }
    Ok(())
}

fn subject_kind_name(kind: RustBehaviorSnapshotSubjectKind) -> &'static str {
    match kind {
        RustBehaviorSnapshotSubjectKind::PublicApi => "public-api",
        RustBehaviorSnapshotSubjectKind::Function => "function",
        RustBehaviorSnapshotSubjectKind::Method => "method",
        RustBehaviorSnapshotSubjectKind::Module => "module",
        RustBehaviorSnapshotSubjectKind::Cli => "cli",
        RustBehaviorSnapshotSubjectKind::Test => "test",
        RustBehaviorSnapshotSubjectKind::Custom => "custom",
    }
}

fn status_name(status: RustBehaviorSnapshotStatus) -> &'static str {
    match status {
        RustBehaviorSnapshotStatus::Matched => "matched",
        RustBehaviorSnapshotStatus::Changed => "changed",
        RustBehaviorSnapshotStatus::Missing => "missing",
        RustBehaviorSnapshotStatus::Skipped => "skipped",
        RustBehaviorSnapshotStatus::Error => "error",
    }
}

fn print_behavior_help() {
    println!(
        "asp-rust behavior snapshot --kind KIND --path PATH [--symbol SYMBOL] [--status STATUS] [--expected TEXT] [--actual TEXT] [--diff TEXT] [--receipt-id ID] [--receipt-json PATH] [--candidate-id ID] [--json]\n\n\
Kinds: public-api, function, method, module, cli, test, custom.\n\
Statuses: matched, changed, missing, skipped, error.\n\
Use --json to emit the semantic-behavior-snapshot JSON contract."
    );
}
