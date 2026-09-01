use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::RustSearchOptions;
use super::cargo;
use super::context::{PackageSearchContext, search_contexts};
use super::format::{
    append_block, compact_locations, display_project_path, package_label,
    render_cargo_dependency_line,
};
use super::hits::{dependency_usage, matching_dependencies};
use super::limits::SEARCH_HIT_LIMIT;
use super::scope::module_is_scope;
use crate::AspRustConfig;

pub(super) fn render_search_code(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    if query != "comments" {
        return Ok(format!(
            "[search-code] q={query} claim=0 fact=0 witness=0\n|quality status=insufficient missing=code-namespace-query next=code:comments\n"
        ));
    }
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let claims = comment_claims(&context, options);
        let mut block = format!(
            "[search-code] q=comments pkg={} claim={} fact=0 witness=0\n",
            package_label(project_root, &context.package_root),
            claims.len()
        );
        for claim in claims.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(
                block,
                "|claim kind={} owner={} line={} evidenceGrade=claim evidence=comment verdict=unverified text={}",
                claim.kind, claim.owner, claim.line, claim.text
            );
        }
        if claims.is_empty() {
            let _ = writeln!(block, "|note kind=no-comment-claims evidenceGrade=fact");
        }
        let next = claims
            .first()
            .map(|claim| format!("owner:{}", claim.owner))
            .unwrap_or_else(|| "owner:<path>".to_string());
        let _ = writeln!(
            block,
            "|quality status=partial missing=parser-verdict,witness next={next}"
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

pub(super) fn render_search_env(
    project_root: &Path,
    config: &AspRustConfig,
    query: Option<&str>,
    options: &RustSearchOptions,
) -> Result<String, String> {
    match query.unwrap_or("toolchain") {
        "toolchain" => render_search_env_toolchain(project_root, config, options),
        "cfg" => render_search_env_cfg(project_root, config, options),
        other => Ok(format!(
            "[search-env] q={other} fact=0 witness=0\n|quality status=insufficient missing=env-namespace-query next=env:toolchain,env:cfg\n"
        )),
    }
}

pub(super) fn render_search_extension(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let extension = query.replace('_', "-");
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let dependencies = matching_dependencies(&context.cargo_dependencies, &extension);
        let usage = dependency_usage(&context, &extension);
        let mut block = format!(
            "[search-extension] q={} pkg={} extension={} dep={} own={}\n",
            query,
            package_label(project_root, &context.package_root),
            extension,
            dependencies.len(),
            usage.len()
        );
        for dependency in dependencies.iter().take(SEARCH_HIT_LIMIT) {
            let _ = writeln!(block, "{}", render_cargo_dependency_line(dependency));
        }
        if dependencies.is_empty() {
            let _ = writeln!(
                block,
                "|quality status=insufficient missing=dependency-activation next=deps:{extension}"
            );
            append_block(&mut rendered, &block);
            continue;
        }
        let _ = writeln!(
            block,
            "|extension {extension} status=activated source=manifest evidenceGrade=fact"
        );
        for hit in usage.iter().take(SEARCH_HIT_LIMIT) {
            let owner = display_project_path(&context.package_root, &hit.path);
            let _ = writeln!(
                block,
                "|owner {owner} hit_kind=extension-usage extension={extension} locations={} evidenceGrade=fact next=owner:{owner}",
                compact_locations(&hit.locations)
            );
        }
        if let Some(guidance) = cargo::dependency_capability_guidance(&extension, &usage) {
            let guidance = guidance.replacen("|dependency-guidance", "|extension-guidance", 1);
            let _ = writeln!(
                block,
                "{guidance} source=provider-capability-catalog evidenceGrade=fact"
            );
        }
        let status = if usage.is_empty() { "partial" } else { "ok" };
        let missing = if usage.is_empty() {
            "source-usage,witness"
        } else {
            "witness"
        };
        let _ = writeln!(
            block,
            "|quality status={status} missing={missing} next=deps:{extension},pattern:{extension}-boundary"
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_env_toolchain(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let manifest = manifest_env_facts(&context.package_root);
        let rustc = command_first_line(&context.package_root, "rustc", &["-Vv"]);
        let rustup = command_first_line(
            &context.package_root,
            "rustup",
            &["show", "active-toolchain"],
        );
        let toolchain_file = nearest_toolchain_file(project_root, &context.package_root);
        let mut block = format!(
            "[search-env] q=toolchain pkg={} fact=3 witness={}\n",
            package_label(project_root, &context.package_root),
            usize::from(rustc.is_some()) + usize::from(rustup.is_some())
        );
        let _ = writeln!(
            block,
            "|env toolchainFile={} source=file evidenceGrade=fact",
            toolchain_file
                .as_ref()
                .map(|path| display_project_path(project_root, path))
                .unwrap_or_else(|| "-".to_string())
        );
        let _ = writeln!(
            block,
            "|env rustcVersion={} source=rustc-version evidenceGrade=witness status={}",
            rustc
                .as_deref()
                .map(field_token)
                .unwrap_or_else(|| "-".to_string()),
            status_label(rustc.is_some())
        );
        let _ = writeln!(
            block,
            "|env rustupActiveToolchain={} source=rustup-active-toolchain evidenceGrade=witness status={}",
            rustup
                .as_deref()
                .map(field_token)
                .unwrap_or_else(|| "-".to_string()),
            status_label(rustup.is_some())
        );
        let _ = writeln!(
            block,
            "|env cargoManifest edition={} resolver={} features={} source=manifest manager=cargo evidenceGrade=fact",
            manifest.edition, manifest.resolver, manifest.feature_count
        );
        let _ = writeln!(
            block,
            "|env cargoLock present={} path={} source=file evidenceGrade=fact",
            manifest.lock_path.is_some(),
            manifest
                .lock_path
                .as_ref()
                .map(|path| display_project_path(project_root, path))
                .unwrap_or_else(|| "-".to_string())
        );
        let _ = writeln!(
            block,
            "|quality status=partial missing=cargo-metadata,resolved-features next=env:cfg"
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_search_env_cfg(
    project_root: &Path,
    config: &AspRustConfig,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let cfg_lines = command_lines(&context.package_root, "rustc", &["--print", "cfg"]);
        let mut block = format!(
            "[search-env] q=cfg pkg={} cfg={} witness={}\n",
            package_label(project_root, &context.package_root),
            cfg_lines.len(),
            usize::from(!cfg_lines.is_empty())
        );
        for cfg in cfg_lines.iter().take(SEARCH_HIT_LIMIT) {
            let (key, value) = cfg_key_value(cfg);
            let _ = writeln!(
                block,
                "|env cfg key={} value={} source=rustc-print-cfg evidenceGrade=witness",
                field_token(&key),
                value.map_or_else(|| "-".to_string(), |value| field_token(&value))
            );
        }
        if cfg_lines.is_empty() {
            let _ = writeln!(
                block,
                "|quality status=insufficient missing=rustc-print-cfg next=env:toolchain"
            );
        } else {
            let _ = writeln!(
                block,
                "|quality status=partial missing=cargo-metadata,resolved-feature-cfg next=cfg:<name>"
            );
        }
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

struct CommentClaim {
    owner: String,
    line: usize,
    kind: &'static str,
    text: String,
}

fn comment_claims(
    context: &PackageSearchContext,
    options: &RustSearchOptions,
) -> Vec<CommentClaim> {
    let mut claims = Vec::new();
    for module in &context.parsed_modules {
        if !module_allowed_for_comments(context, module, options) {
            continue;
        }
        let owner = display_project_path(&context.package_root, &module.report.path);
        for (line_index, line) in module.source.lines().enumerate() {
            let trimmed = line.trim_start();
            let Some((kind, text)) = comment_line_claim(trimmed) else {
                continue;
            };
            claims.push(CommentClaim {
                owner: owner.clone(),
                line: line_index + 1,
                kind,
                text: field_token(text),
            });
        }
    }
    claims
}

fn module_allowed_for_comments(
    context: &PackageSearchContext,
    module: &crate::parser::ParsedRustModule,
    options: &RustSearchOptions,
) -> bool {
    if let Some(owner) = options.owner.as_deref() {
        return display_project_path(&context.package_root, &module.report.path) == owner;
    }
    module_is_scope(
        &context.scope,
        module,
        options.scope.as_deref().unwrap_or("all"),
    )
}

fn comment_line_claim(line: &str) -> Option<(&'static str, &str)> {
    if let Some(text) = line.strip_prefix("//!") {
        return Some(("module-doc-comment", text.trim()));
    }
    if let Some(text) = line.strip_prefix("///") {
        return Some(("doc-comment", text.trim()));
    }
    if let Some(text) = line.strip_prefix("//") {
        return Some(("line-comment", text.trim()));
    }
    if let Some(text) = line.strip_prefix("/*") {
        return Some(("block-comment", text.trim().trim_end_matches("*/").trim()));
    }
    None
}

struct ManifestEnvFacts {
    edition: String,
    resolver: String,
    feature_count: usize,
    lock_path: Option<PathBuf>,
}

fn manifest_env_facts(package_root: &Path) -> ManifestEnvFacts {
    let manifest_table = fs::read_to_string(package_root.join("Cargo.toml"))
        .ok()
        .and_then(|content| content.parse::<toml::Table>().ok());
    let edition = manifest_table
        .as_ref()
        .and_then(|table| table.get("package"))
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("edition"))
        .and_then(toml::Value::as_str)
        .map(field_token)
        .unwrap_or_else(|| "-".to_string());
    let resolver = manifest_table
        .as_ref()
        .and_then(|table| table.get("workspace"))
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("resolver"))
        .and_then(toml::Value::as_str)
        .map(field_token)
        .unwrap_or_else(|| "-".to_string());
    let feature_count = manifest_table
        .as_ref()
        .and_then(|table| table.get("features"))
        .and_then(toml::Value::as_table)
        .map_or(0, toml::map::Map::len);
    let lock_path = cargo_lock_path(package_root);
    ManifestEnvFacts {
        edition,
        resolver,
        feature_count,
        lock_path,
    }
}

fn cargo_lock_path(package_root: &Path) -> Option<PathBuf> {
    let direct = package_root.join("Cargo.lock");
    if direct.is_file() {
        return Some(direct);
    }
    package_root
        .ancestors()
        .map(|ancestor| ancestor.join("Cargo.lock"))
        .find(|candidate| candidate.is_file())
}

fn nearest_toolchain_file(project_root: &Path, package_root: &Path) -> Option<PathBuf> {
    ["rust-toolchain.toml", "rust-toolchain"]
        .into_iter()
        .flat_map(|file_name| [package_root.join(file_name), project_root.join(file_name)])
        .find(|candidate| candidate.is_file())
}

fn command_first_line(cwd: &Path, program: &str, args: &[&str]) -> Option<String> {
    command_lines(cwd, program, args).into_iter().next()
}

fn command_lines(cwd: &Path, program: &str, args: &[&str]) -> Vec<String> {
    let Ok(output) = Command::new(program).args(args).current_dir(cwd).output() else {
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

fn cfg_key_value(cfg: &str) -> (String, Option<String>) {
    cfg.split_once('=')
        .map(|(key, value)| {
            (
                key.to_string(),
                Some(value.trim_matches('"').trim().to_string()),
            )
        })
        .unwrap_or_else(|| (cfg.to_string(), None))
}

fn status_label(ok: bool) -> &'static str {
    if ok { "ok" } else { "missing" }
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
        token.chars().take(120).collect()
    }
}
