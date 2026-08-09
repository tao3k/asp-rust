//! ASP-owned compact graph renderer adapter.

use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

const ASP_BINARY_ENV: &str = "SEMANTIC_AGENT_PROTOCOL_BIN";
const DEFAULT_SEED_LIMIT: usize = 8;

pub(in crate::cli) fn render_compact_graph_seed_packet(
    packet_json: &str,
    seed_limit: Option<usize>,
) -> Result<String, String> {
    let binary = env::var_os(ASP_BINARY_ENV).unwrap_or_else(|| "asp".into());
    let seed_limit = seed_limit.unwrap_or(DEFAULT_SEED_LIMIT).to_string();
    let mut child = Command::new(&binary)
        .args([
            "graph",
            "render",
            "--packet",
            "-",
            "--view",
            "seeds",
            "--seeds",
            &seed_limit,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "asp graph renderer not found; set {ASP_BINARY_ENV} or install asp on PATH: {error}"
            )
        })?;
    child
        .stdin
        .take()
        .ok_or_else(|| "asp graph renderer stdin unavailable".to_string())?
        .write_all(packet_json.as_bytes())
        .map_err(|error| {
            format!("failed to write semantic search packet to asp graph renderer: {error}")
        })?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for asp graph renderer: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "asp graph render failed with exit code {}: {}",
            output.status.code().unwrap_or(1),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("asp graph renderer returned non-UTF-8 output: {error}"))
}
