//! Language-neutral compare namespace backed by Rust toolchain evidence.

use std::ffi::OsString;
use std::fmt::Write;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use crate::RustHarnessConfig;

use super::RustSearchOptions;

const COMPARE_SCHEMA_ID: &str = "agent.semantic-protocols.semantic-compare-packet";

pub(super) fn render_search_compare(
    project_root: &Path,
    _config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    Ok(render_compare_packet_lines(&compare_packet(
        project_root,
        query,
        options,
    )))
}

/// Render a schema-owned compare packet for Rust evidence search namespaces.
pub fn render_search_compare_json(
    project_root: &Path,
    _config: &RustHarnessConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    serde_json::to_string(&compare_packet(project_root, query, options))
        .map_err(|error| format!("failed to render compare JSON: {error}"))
}

fn compare_packet(project_root: &Path, query: &str, options: &RustSearchOptions) -> Value {
    if query != "env" {
        return unsupported_compare_packet(query);
    }
    let (left, right) = compare_sides(options);
    let evidence = ToolchainCompareEvidence::collect(project_root, &left, &right);
    toolchain_compare_packet(&evidence)
}

fn unsupported_compare_packet(query: &str) -> Value {
    json!({
        "schemaId": COMPARE_SCHEMA_ID,
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "languageId": "rust",
            "providerId": "asp-rust",
        "namespace": "compare",
        "authority": "unsupported-rust-compare-axis",
        "evidenceGrade": "unknown",
        "quality": "insufficient",
        "query": query,
        "comparisons": [],
        "missing": ["supported-axis:env"],
        "witness": "unsupported-rust-compare-axis",
        "next": "search env toolchain"
    })
}

fn toolchain_compare_packet(evidence: &ToolchainCompareEvidence) -> Value {
    let quality = if evidence.verified {
        "verified"
    } else {
        "insufficient"
    };
    let evidence_grade = if evidence.active_toolchain.is_some() {
        "fact"
    } else {
        "unknown"
    };
    json!({
        "schemaId": COMPARE_SCHEMA_ID,
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "languageId": "rust",
            "providerId": "asp-rust",
        "namespace": "compare",
        "authority": "active-toolchain-vs-requested",
        "evidenceGrade": evidence_grade,
        "quality": quality,
        "query": format!("env {} {}", evidence.left, evidence.right),
        "comparisons": [{
            "id": "env-stable-nightly",
            "summary": "Compare the active Rust toolchain with requested stable/nightly sides before version-sensitive guidance.",
            "evidenceGrade": evidence_grade,
            "witness": evidence.witness,
            "next": "search env toolchain",
            "left": {
                "kind": "active-toolchain",
                "channel": evidence.active_channel,
                "raw": evidence.active_toolchain.as_deref().unwrap_or("-"),
                "rustcVersion": evidence.rustc_version.as_deref().unwrap_or("-"),
                "source": "rustup-active-toolchain"
            },
            "right": {
                "kind": "requested-toolchain-set",
                "left": evidence.left,
                "right": evidence.right,
                "leftAvailable": evidence.left_available,
                "rightAvailable": evidence.right_available,
                "source": "rustup-toolchain-list"
            },
            "result": evidence.result,
            "agentScenario": "agent-needs-to-distinguish-stable-and-nightly-rust-before-writing-version-sensitive-code",
            "intent": "compare-active-toolchain-against-requested-stable-nightly-before-using-version-specific-rust-features",
            "qualitySignals": evidence.quality_signals,
            "failureCases": [
                {
                    "id": "nightly-api-on-stable",
                    "risk": "agent uses nightly-only API while the active toolchain is stable",
                    "correction": "query compare env stable nightly before version-sensitive Rust guidance"
                },
                {
                    "id": "uninstalled-comparison-side",
                    "risk": "agent claims a stable/nightly difference without resolver evidence for both sides",
                    "correction": "keep comparison insufficient until rustup proves both requested sides"
                }
            ]
        }],
        "missing": evidence.missing,
        "witness": evidence.witness,
        "next": "search env toolchain"
    })
}

fn render_compare_packet_lines(packet: &Value) -> String {
    let query = packet["query"].as_str().unwrap_or("-");
    let authority = packet["authority"].as_str().unwrap_or("-");
    let evidence_grade = packet["evidenceGrade"].as_str().unwrap_or("unknown");
    let quality = packet["quality"].as_str().unwrap_or("insufficient");
    let missing = packet["missing"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string());
    let mut rendered = format!(
        "[search-compare] query={} evidenceGrade={evidence_grade} authority={authority} quality={quality} missing={missing}\n",
        field_token(query)
    );
    for comparison in packet["comparisons"].as_array().into_iter().flatten() {
        let _ = writeln!(
            rendered,
            "|compare id={} result={} witness={} next={}",
            comparison["id"].as_str().unwrap_or("-"),
            comparison["result"].as_str().unwrap_or("-"),
            comparison["witness"].as_str().unwrap_or("-"),
            comparison["next"].as_str().unwrap_or("-")
        );
        append_side_line(&mut rendered, "left", &comparison["left"]);
        append_side_line(&mut rendered, "right", &comparison["right"]);
        for failure_case in comparison["failureCases"].as_array().into_iter().flatten() {
            let _ = writeln!(
                rendered,
                "|failureCase id={} risk={} correction={}",
                failure_case["id"].as_str().unwrap_or("-"),
                field_token(failure_case["risk"].as_str().unwrap_or("-")),
                field_token(failure_case["correction"].as_str().unwrap_or("-"))
            );
        }
        for signal in comparison["qualitySignals"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let _ = writeln!(rendered, "|qualitySignal id={signal}");
        }
    }
    let _ = writeln!(rendered, "next={}", packet["next"].as_str().unwrap_or("-"));
    rendered
}

fn append_side_line(rendered: &mut String, label: &str, side: &Value) {
    let Some(object) = side.as_object() else {
        return;
    };
    let mut fields = vec![format!("|{label}")];
    for (key, value) in object {
        let value = match value {
            Value::Bool(value) => value.to_string(),
            Value::String(value) => field_token(value),
            Value::Number(value) => value.to_string(),
            _ => field_token(&value.to_string()),
        };
        fields.push(format!("{key}={value}"));
    }
    rendered.push_str(&fields.join(" "));
    rendered.push('\n');
}

struct ToolchainCompareEvidence {
    left: String,
    right: String,
    active_toolchain: Option<String>,
    active_channel: String,
    rustc_version: Option<String>,
    left_available: bool,
    right_available: bool,
    verified: bool,
    result: &'static str,
    witness: &'static str,
    missing: Vec<String>,
    quality_signals: Vec<&'static str>,
}

impl ToolchainCompareEvidence {
    fn collect(project_root: &Path, left: &str, right: &str) -> Self {
        let active_toolchain =
            command_first_line(project_root, "rustup", &["show", "active-toolchain"]);
        let rustc_version = command_first_line(project_root, "rustc", &["-Vv"]);
        let installed_toolchains = command_lines(project_root, "rustup", &["toolchain", "list"]);
        let active_channel = active_toolchain
            .as_deref()
            .map(toolchain_channel)
            .unwrap_or_else(|| "unknown".to_string());
        let left_available = toolchain_available(&installed_toolchains, left);
        let right_available = toolchain_available(&installed_toolchains, right);
        let verified = active_toolchain.is_some()
            && rustc_version.is_some()
            && left_available
            && right_available
            && (active_channel == left || active_channel == right);
        let mut missing = Vec::new();
        if active_toolchain.is_none() {
            missing.push("rustup-active-toolchain".to_string());
        }
        if rustc_version.is_none() {
            missing.push("rustc-version".to_string());
        }
        if !left_available {
            missing.push(format!("toolchain:{left}"));
        }
        if !right_available {
            missing.push(format!("toolchain:{right}"));
        }
        if active_toolchain.is_some() && active_channel != left && active_channel != right {
            missing.push("active-toolchain-outside-comparison".to_string());
        }
        let result = if !left_available || !right_available {
            "requested-toolchain-unavailable"
        } else if active_channel == left {
            "active-toolchain-matches-left"
        } else if active_channel == right {
            "active-toolchain-matches-right"
        } else {
            "active-toolchain-outside-comparison"
        };
        let witness = if verified {
            "rustup-active-toolchain-and-toolchain-list"
        } else {
            "missing-rustup-or-requested-toolchain"
        };
        let mut quality_signals = vec!["no-memory"];
        if active_toolchain.is_some() {
            quality_signals.push("active-toolchain-fact");
        } else {
            quality_signals.push("active-toolchain-missing");
        }
        if left_available && right_available {
            quality_signals.push("both-requested-toolchains-installed");
        } else {
            quality_signals.push("comparison-side-missing");
        }
        Self {
            left: left.to_string(),
            right: right.to_string(),
            active_toolchain,
            active_channel,
            rustc_version,
            left_available,
            right_available,
            verified,
            result,
            witness,
            missing,
            quality_signals,
        }
    }
}

fn compare_sides(options: &RustSearchOptions) -> (String, String) {
    let sides = options
        .scope
        .as_deref()
        .unwrap_or("stable,nightly")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let left = sides.first().copied().unwrap_or("stable");
    let right = sides.get(1).copied().unwrap_or("nightly");
    (left.to_string(), right.to_string())
}

fn command_first_line(cwd: &Path, program: &str, args: &[&str]) -> Option<String> {
    command_lines(cwd, program, args).into_iter().next()
}

fn command_lines(cwd: &Path, program: &str, args: &[&str]) -> Vec<String> {
    let Ok(output) = Command::new(command_program(program))
        .args(args)
        .current_dir(cwd)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(not(windows))]
fn command_program(program: &str) -> OsString {
    OsString::from(program)
}

#[cfg(windows)]
fn command_program(program: &str) -> OsString {
    let path = Path::new(program);
    if path.components().count() > 1 || path.extension().is_some() {
        return OsString::from(program);
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return OsString::from(program);
    };
    for dir in std::env::split_paths(&paths) {
        for extension in ["exe", "cmd", "bat"] {
            let candidate = dir.join(format!("{program}.{extension}"));
            if candidate.is_file() {
                return candidate.into_os_string();
            }
        }
    }
    OsString::from(program)
}

fn toolchain_available(installed: &[String], requested: &str) -> bool {
    installed
        .iter()
        .any(|toolchain| toolchain_channel(toolchain) == requested)
}

fn toolchain_channel(toolchain: &str) -> String {
    let token = toolchain
        .split_whitespace()
        .next()
        .unwrap_or(toolchain)
        .trim();
    token
        .split_once('-')
        .map_or(token, |(channel, _)| channel)
        .to_string()
}

fn field_token(value: &str) -> String {
    let token = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_whitespace() || matches!(character, '|' | ',' | ';') {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    if token.is_empty() {
        "-".to_string()
    } else {
        token.chars().take(160).collect()
    }
}
