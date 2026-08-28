use std::{env, path::PathBuf};

use crate::cli::{moved_agent_action, rust_package_root_for_path, rust_project_root_for_path};
use crate::runner::RustHarnessRunScope;

use super::{AgentOptions, CliOptions, ResolvedCheckTarget};

impl AgentOptions {
    pub(super) fn parse(
        args: impl IntoIterator<Item = std::ffi::OsString>,
    ) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        let mut doctor_seen = false;
        let mut positional_only = false;
        while let Some(arg) = args.next() {
            let value = arg.to_string_lossy();
            if !positional_only {
                match value.as_ref() {
                    "--" => {
                        positional_only = true;
                        continue;
                    }
                    "--json" => {
                        options.json = true;
                        continue;
                    }
                    "--codex" => {
                        options.set_client("codex")?;
                        continue;
                    }
                    "--client" => {
                        let client = args
                            .next()
                            .ok_or_else(|| "expected value after --client".to_string())?;
                        let client = client
                            .into_string()
                            .map_err(|_| "expected UTF-8 value after --client".to_string())?;
                        options.set_client(&client)?;
                        continue;
                    }
                    "--help" | "-h" => {
                        options.help = true;
                        continue;
                    }
                    value if value.starts_with('-') => {
                        return Err(format!("unknown agent option: {value}"));
                    }
                    _ => {}
                }
            }
            if !doctor_seen {
                let command = arg
                    .into_string()
                    .map_err(|_| "expected UTF-8 agent command".to_string())?;
                match command.as_str() {
                    "doctor" => doctor_seen = true,
                    "guide" => {
                        return Err("asp-rust agent guide moved to asp-rust guide".to_string());
                    }
                    "install" | "hook" | "guard" => return Err(moved_agent_action(&command)),
                    other => return Err(format!("unknown agent command: {other}")),
                }
            } else {
                options.paths.push(PathBuf::from(arg));
            }
        }
        if !doctor_seen && !options.help {
            return Err("expected agent command: doctor".to_string());
        }
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        Ok(options)
    }

    fn set_client(&mut self, client: &str) -> Result<(), String> {
        match self.client.as_deref() {
            Some(existing) if existing != client => Err(format!(
                "conflicting agent clients: {existing} and {client}"
            )),
            _ => {
                self.client = Some(client.to_string());
                Ok(())
            }
        }
    }

    pub(super) fn project_root(&self) -> Result<PathBuf, String> {
        match self.paths.as_slice() {
            [path] => Ok(path.clone()),
            [] => {
                env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }
}

impl CliOptions {
    pub(super) fn parse(
        args: impl IntoIterator<Item = std::ffi::OsString>,
    ) -> Result<Self, String> {
        let mut options = Self::default();
        let mut positional_only = false;
        for arg in args {
            if positional_only {
                options.paths.push(PathBuf::from(arg));
                continue;
            }
            let Some(value) = arg.to_str() else {
                options.paths.push(PathBuf::from(arg));
                continue;
            };
            match value {
                "--" => positional_only = true,
                "--json" => options.json = true,
                "--agent-snapshot" => options.agent_snapshot = true,
                "--help" | "-h" => options.help = true,
                value if value.starts_with('-') => {
                    return Err(format!("unknown option: {value}"));
                }
                _ => options.paths.push(PathBuf::from(arg)),
            }
        }
        if options.paths.len() > 1 {
            return Err("expected at most one PROJECT_ROOT argument".to_string());
        }
        if options.json && options.agent_snapshot {
            return Err("expected only one output mode: --json or --agent-snapshot".to_string());
        }
        Ok(options)
    }

    pub(super) fn target(&self) -> Result<ResolvedCheckTarget, String> {
        match self.paths.as_slice() {
            [path] => Ok(ResolvedCheckTarget {
                root: rust_package_root_for_path(path)?,
                scope: RustHarnessRunScope::Package,
            }),
            [] => {
                let current = env::current_dir()
                    .map_err(|error| format!("failed to read current dir: {error}"))?;
                Ok(ResolvedCheckTarget {
                    root: rust_project_root_for_path(&current)?,
                    scope: RustHarnessRunScope::ProjectWorkspace,
                })
            }
            _ => unreachable!("parse enforces at most one path"),
        }
    }
}
