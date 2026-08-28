// Development command log lifecycle and JSONL event writer.

use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use super::command::normalize_command;
use super::context::{
    allocate_session_ordinal, binary_name, command_log_file_name, env_truthy, infer_project_root,
    make_event_id, redact_argv, resolve_log_root, resolve_session_context, stable_hash_hex,
};
use super::json::{
    push_array_field, push_command_field, push_fields_field, push_result_field, push_string_field,
    push_u64_field,
};
use super::record::ActiveCommandLog;
use super::time::format_utc_timestamp;

use super::constants::{PROTOCOL_ID, PROTOCOL_VERSION, SCHEMA_ID, SCHEMA_VERSION};

pub(crate) enum DevCommandLog {
    Disabled,
    Active(Box<ActiveCommandLog>),
}

impl DevCommandLog {
    pub(crate) fn start(argv: &[OsString]) -> Self {
        if !env_truthy("SEMANTIC_PROTOCOL_DEV_MODE") {
            return Self::Disabled;
        }

        let started_at_system = SystemTime::now();
        let started_at_instant = Instant::now();
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let project_root = infer_project_root(argv, &cwd).unwrap_or_else(|| cwd.clone());
        let project_root_hash = stable_hash_hex(&project_root.display().to_string());
        let log_root = match resolve_log_root(&project_root) {
            Some(path) => path,
            None => return Self::Disabled,
        };
        let session = resolve_session_context(&log_root, &project_root_hash);
        let session_ordinal = allocate_session_ordinal(&log_root, &session.session_id).unwrap_or(0);
        let event_id = make_event_id(started_at_system, session_ordinal);
        let log_file = log_root
            .join("rust")
            .join("asp-rust")
            .join("commands")
            .join(command_log_file_name(
                started_at_system,
                session_ordinal,
                &event_id,
            ));
        let argv = redact_argv(argv);
        let binary = argv
            .first()
            .map(|arg| binary_name(arg))
            .unwrap_or_else(|| "asp-rust".to_string());
        let command = normalize_command(&argv);

        Self::Active(Box::new(ActiveCommandLog {
            argv,
            binary,
            command,
            context_source: session.source,
            cwd,
            event_id,
            hook_run_id: session.hook_run_id,
            log_file,
            parent_event_id: session.parent_event_id,
            project_root,
            project_root_hash,
            session_id: session.session_id,
            session_ordinal,
            started_at_instant,
            started_at_system,
        }))
    }

    pub(crate) fn finish(self, exit_code: i32) {
        if let Self::Active(active) = self {
            let _ = write_event(active.as_ref(), exit_code);
        }
    }
}

fn write_event(active: &ActiveCommandLog, exit_code: i32) -> io::Result<()> {
    let finished_at_system = SystemTime::now();
    let elapsed_ms = active
        .started_at_instant
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX));
    let parent = active.log_file.parent().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "dev command log file does not have a parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    let mut line = String::new();
    let mut first = true;
    line.push('{');
    push_string_field(&mut line, &mut first, "schemaId", SCHEMA_ID);
    push_string_field(&mut line, &mut first, "schemaVersion", SCHEMA_VERSION);
    push_string_field(&mut line, &mut first, "protocolId", PROTOCOL_ID);
    push_string_field(&mut line, &mut first, "protocolVersion", PROTOCOL_VERSION);
    push_string_field(
        &mut line,
        &mut first,
        "timestampUtc",
        &format_utc_timestamp(finished_at_system),
    );
    push_string_field(
        &mut line,
        &mut first,
        "startedAtUtc",
        &format_utc_timestamp(active.started_at_system),
    );
    push_string_field(
        &mut line,
        &mut first,
        "finishedAtUtc",
        &format_utc_timestamp(finished_at_system),
    );
    push_string_field(&mut line, &mut first, "eventId", &active.event_id);
    push_string_field(&mut line, &mut first, "sessionId", &active.session_id);
    push_u64_field(
        &mut line,
        &mut first,
        "sessionOrdinal",
        active.session_ordinal,
    );
    if let Some(parent_event_id) = &active.parent_event_id {
        push_string_field(&mut line, &mut first, "parentEventId", parent_event_id);
    }
    push_string_field(&mut line, &mut first, "languageId", "rust");
    push_string_field(&mut line, &mut first, "providerId", "asp-rust");
    push_string_field(&mut line, &mut first, "binary", &active.binary);
    push_array_field(&mut line, &mut first, "argv", &active.argv);
    push_string_field(
        &mut line,
        &mut first,
        "cwd",
        &active.cwd.display().to_string(),
    );
    push_string_field(
        &mut line,
        &mut first,
        "projectRoot",
        &active.project_root.display().to_string(),
    );
    push_string_field(
        &mut line,
        &mut first,
        "projectRootHash",
        &active.project_root_hash,
    );
    if let Some(hook_run_id) = &active.hook_run_id {
        push_string_field(&mut line, &mut first, "hookRunId", hook_run_id);
    }
    push_command_field(&mut line, &mut first, &active.command);
    push_result_field(&mut line, &mut first, exit_code, elapsed_ms as u64);
    push_fields_field(&mut line, &mut first, &active.context_source);
    line.push('}');
    line.push('\n');

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&active.log_file)?;
    file.write_all(line.as_bytes())
}
