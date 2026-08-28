use serde_json::Value;
use std::fs;
use std::process::Command;

use crate::cli::support::configure_shared_asp_renderer;

#[test]
fn cli_dev_mode_records_ordered_command_log_jsonl() {
    let project = tempfile::tempdir().expect("temp project");
    fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname = \"dev-log-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    fs::create_dir_all(project.path().join("src")).expect("create src");
    fs::write(project.path().join("src/lib.rs"), "pub fn fixture() {}\n").expect("write lib.rs");

    let trace = tempfile::tempdir().expect("trace dir");
    let project_root = fs::canonicalize(project.path()).expect("canonical project root");
    let project_root_hash = stable_hash_hex(&project_root.display().to_string());
    let context_dir = trace.path().join("dev-context");
    fs::create_dir_all(&context_dir).expect("create context dir");
    fs::write(
        context_dir.join(format!("{project_root_hash}.json")),
        r#"{"sessionId":"session-from-hook","parentEventId":"hook-parent-1","hookRunId":"hook-run-1"}"#,
    )
    .expect("write active context marker");

    let mut command = Command::new(env!("CARGO_BIN_EXE_asp-rust"));
    configure_shared_asp_renderer(&mut command);
    let output = command
        .arg("guide")
        .arg(project.path())
        .env("SEMANTIC_PROTOCOL_DEV_MODE", "1")
        .env("SEMANTIC_PROTOCOL_TRACE_DIR", trace.path())
        .env_remove("SEMANTIC_PROTOCOL_SESSION_ID")
        .env_remove("SEMANTIC_PROTOCOL_PARENT_EVENT_ID")
        .env_remove("SEMANTIC_PROTOCOL_HOOK_RUN_ID")
        .output()
        .expect("run rs-harness");

    assert!(
        output.status.success(),
        "rs-harness failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let command_dir = trace.path().join("rust/rs-harness/commands");
    let entries = fs::read_dir(&command_dir)
        .expect("command log dir")
        .collect::<Result<Vec<_>, _>>()
        .expect("command log entries");
    assert_eq!(entries.len(), 1);
    let file_name = entries[0].file_name().to_string_lossy().into_owned();
    assert!(file_name.starts_with("20"), "{file_name}");
    assert!(file_name.contains('T'), "{file_name}");
    assert!(file_name.contains("-000001-"), "{file_name}");
    assert!(file_name.ends_with(".jsonl"), "{file_name}");

    let content = fs::read_to_string(entries[0].path()).expect("read command log");
    let event = serde_json::from_str::<Value>(content.trim()).expect("command log json");

    assert_eq!(
        event["schemaId"],
        "agent.semantic-protocols.dev-command-log"
    );
    assert_eq!(event["languageId"], "rust");
    assert_eq!(event["providerId"], "rs-harness");
    assert_eq!(event["sessionId"], "session-from-hook");
    assert_eq!(event["sessionOrdinal"], 1);
    assert_eq!(event["parentEventId"], "hook-parent-1");
    assert_eq!(event["hookRunId"], "hook-run-1");
    assert_eq!(event["fields"]["contextSource"], "active-context");
    assert!(event["startedAtUtc"].as_str().is_some());
    assert!(event["finishedAtUtc"].as_str().is_some());
    assert!(event.get("stdout").is_none());
    assert!(event.get("stderr").is_none());
}

fn stable_hash_hex(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
