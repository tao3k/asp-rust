use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::{Duration, SystemTime};

use super::command::normalize_token;
use super::constants::{PROJECT_ANCHORS, SECRET_FLAGS};
use super::time::{format_utc_file_timestamp, millis_since_epoch};

pub(super) struct SessionContext {
    pub(super) hook_run_id: Option<String>,
    pub(super) parent_event_id: Option<String>,
    pub(super) session_id: String,
    pub(super) source: String,
}
pub(super) fn resolve_log_root(project_root: &Path) -> Option<PathBuf> {
    if let Some(value) = env_non_empty("SEMANTIC_PROTOCOL_TRACE_DIR") {
        return Some(path_from_env(&value, project_root));
    }
    if let Some(value) = env_non_empty("PRJ_CACHE_HOME") {
        return Some(path_from_env(&value, project_root).join("semantic_protocol"));
    }
    if let Some(value) = env_non_empty("XDG_CACHE_HOME") {
        return Some(
            PathBuf::from(value)
                .join("agent-semantic-protocols")
                .join(stable_hash_hex(&project_root.display().to_string()))
                .join("semantic_protocol"),
        );
    }
    env_non_empty("HOME").map(|home| {
        PathBuf::from(home)
            .join(".cache")
            .join("agent-semantic-protocols")
            .join(stable_hash_hex(&project_root.display().to_string()))
            .join("semantic_protocol")
    })
}

pub(super) fn path_from_env(value: &str, project_root: &Path) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    }
}

pub(super) fn allocate_session_ordinal(log_root: &Path, session_id: &str) -> Option<u64> {
    let dir = log_root.join("rust").join("asp-rust").join("sessions");
    fs::create_dir_all(&dir).ok()?;
    let key = sanitize_file_component(session_id);
    let counter_path = dir.join(format!("{key}.counter"));
    let lock_path = dir.join(format!("{key}.lock"));
    let _guard = acquire_lock(&lock_path)?;
    let current = fs::read_to_string(&counter_path)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let next = current.saturating_add(1);
    fs::write(counter_path, next.to_string()).ok()?;
    Some(next)
}

pub(super) struct LockGuard {
    pub(super) path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(super) fn acquire_lock(lock_path: &Path) -> Option<LockGuard> {
    for _ in 0..50 {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)
        {
            Ok(mut file) => {
                let _ = writeln!(file, "{}", process::id());
                return Some(LockGuard {
                    path: lock_path.to_path_buf(),
                });
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return None,
        }
    }
    None
}

pub(super) fn resolve_session_context(log_root: &Path, project_root_hash: &str) -> SessionContext {
    let env_session_id = env_first(&[
        "SEMANTIC_PROTOCOL_SESSION_ID",
        "CODEX_SESSION_ID",
        "CLAUDE_SESSION_ID",
        "TERM_SESSION_ID",
    ]);
    let env_hook_run_id = env_first(&[
        "SEMANTIC_PROTOCOL_HOOK_RUN_ID",
        "CODEX_HOOK_RUN_ID",
        "AGENT_HOOK_RUN_ID",
    ]);
    let env_parent_event_id = env_first(&["SEMANTIC_PROTOCOL_PARENT_EVENT_ID"]);
    if env_session_id.is_some() || env_hook_run_id.is_some() || env_parent_event_id.is_some() {
        let session_id = env_session_id.unwrap_or_else(|| {
            env_hook_run_id
                .as_ref()
                .map(|hook_run_id| format!("hook-{}", stable_hash_hex(hook_run_id)))
                .unwrap_or_else(|| format!("project-{project_root_hash}"))
        });
        let parent_event_id = env_parent_event_id.or_else(|| env_hook_run_id.clone());
        return SessionContext {
            hook_run_id: env_hook_run_id,
            parent_event_id,
            session_id,
            source: "env".to_string(),
        };
    }
    if let Some(context) = read_active_context(log_root, project_root_hash) {
        return context;
    }
    SessionContext {
        hook_run_id: None,
        parent_event_id: None,
        session_id: format!("project-{project_root_hash}"),
        source: "project-fallback".to_string(),
    }
}

pub(super) fn read_active_context(
    log_root: &Path,
    project_root_hash: &str,
) -> Option<SessionContext> {
    let path = log_root
        .join("dev-context")
        .join(format!("{project_root_hash}.json"));
    let metadata = fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    if modified.elapsed().ok()? > Duration::from_secs(30 * 60) {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    let session_id = extract_json_string(&content, "sessionId")
        .unwrap_or_else(|| format!("project-{project_root_hash}"));
    Some(SessionContext {
        hook_run_id: extract_json_string(&content, "hookRunId"),
        parent_event_id: extract_json_string(&content, "parentEventId"),
        session_id,
        source: "active-context".to_string(),
    })
}

pub(super) fn extract_json_string(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut rest = content.split_once(&needle)?.1.trim_start();
    rest = rest.strip_prefix(':')?.trim_start();
    let mut chars = rest.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            value.push(match ch {
                '"' => '"',
                '\\' => '\\',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(value);
        } else {
            value.push(ch);
        }
    }
    None
}

pub(super) fn env_truthy(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

pub(super) fn env_non_empty(name: &str) -> Option<String> {
    env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(super) fn env_first(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env_non_empty(name))
}

pub(super) fn infer_project_root(argv: &[OsString], cwd: &Path) -> Option<PathBuf> {
    for arg in argv.iter().skip(1).rev() {
        let text = arg.to_string_lossy();
        if text.starts_with('-') {
            continue;
        }
        let path = PathBuf::from(text.as_ref());
        let path = if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        };
        if let Some(root) = project_root_from_path(&path) {
            return Some(root);
        }
    }
    project_root_from_path(cwd)
}

pub(super) fn project_root_from_path(path: &Path) -> Option<PathBuf> {
    let mut cursor = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };
    loop {
        if PROJECT_ANCHORS
            .iter()
            .any(|anchor| cursor.join(anchor).exists())
        {
            return Some(fs::canonicalize(&cursor).unwrap_or(cursor));
        }
        if !cursor.pop() {
            return None;
        }
    }
}

pub(super) fn redact_argv(argv: &[OsString]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(argv.len());
    let mut iter = argv.iter().peekable();
    while let Some(arg) = iter.next() {
        let text = arg.to_string_lossy().to_string();
        if let Some((flag, _)) = text.split_once('=')
            && is_secret_flag(flag)
        {
            redacted.push(format!("{flag}=[REDACTED]"));
            continue;
        }
        if is_secret_flag(&text) {
            redacted.push(text);
            if iter.peek().is_some() {
                let _ = iter.next();
                redacted.push("[REDACTED]".to_string());
            }
            continue;
        }
        redacted.push(text);
    }
    redacted
}

pub(super) fn is_secret_flag(flag: &str) -> bool {
    SECRET_FLAGS.iter().any(|candidate| candidate == &flag)
}

pub(super) fn binary_name(arg: &str) -> String {
    Path::new(arg)
        .file_name()
        .and_then(|name| name.to_str())
        .map(normalize_token)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "asp-rust".to_string())
}
pub(super) fn make_event_id(started_at: SystemTime, session_ordinal: u64) -> String {
    format!(
        "asp-rust-{}-{}-{session_ordinal:06}",
        millis_since_epoch(started_at),
        process::id()
    )
}

pub(super) fn command_log_file_name(
    started_at: SystemTime,
    session_ordinal: u64,
    event_id: &str,
) -> String {
    format!(
        "{}-{session_ordinal:06}-{}.jsonl",
        format_utc_file_timestamp(started_at),
        sanitize_file_component(event_id)
    )
}

pub(super) fn sanitize_file_component(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "unknown".to_string()
    } else {
        output
    }
}

pub(super) fn stable_hash_hex(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
