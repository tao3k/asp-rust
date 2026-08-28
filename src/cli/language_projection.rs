//! Query-free, parser-owned language projection for one Rust owner.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use serde_json::{Value, json};

use crate::parser::parse_rust_file;

pub(super) fn run_language_projection(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<ExitCode, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    let options = LanguageProjectionOptions::parse(args)?;
    if options.help {
        println!("{}", language_projection_usage());
        return Ok(ExitCode::SUCCESS);
    }
    if !options.json {
        return Err("projection requires --json".to_string());
    }
    println!("{}", render_language_projection(&options)?);
    Ok(ExitCode::SUCCESS)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LanguageProjectionBatchHeader {
    schema_id: String,
    schema_version: String,
    language_id: String,
    provider_id: String,
    workspace_identity: String,
    generation_root_digest: String,
    parser_identity_digest: String,
    query_pack_digest: String,
    #[serde(default)]
    base_generation_root_digest: Option<String>,
    owners: Vec<LanguageProjectionBatchOwner>,
    #[serde(default, rename = "auxiliaryOwners")]
    auxiliary_owners: Vec<LanguageProjectionBatchOwner>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LanguageProjectionBatchOwner {
    owner_path: PathBuf,
    source_leaf_digest: String,
    source_encoding: String,
    #[serde(default)]
    source_text: Option<String>,
    #[serde(default)]
    source_bytes_base64: Option<String>,
}

impl LanguageProjectionBatchOwner {
    fn decode_source_bytes(&self) -> Result<Vec<u8>, String> {
        match (
            self.source_encoding.as_str(),
            self.source_text.as_deref(),
            self.source_bytes_base64.as_deref(),
        ) {
            ("utf8", Some(source_text), None) => Ok(source_text.as_bytes().to_vec()),
            ("base64", None, Some(source_bytes_base64)) => BASE64_STANDARD
                .decode(source_bytes_base64)
                .map_err(|error| format!("decode projection owner base64 source: {error}")),
            _ => Err("projection owner source encoding payload mismatch".to_string()),
        }
    }

    fn decode_source_text(&self) -> Result<String, String> {
        String::from_utf8(self.decode_source_bytes()?)
            .map_err(|error| format!("Rust projection owner is not UTF-8: {error}"))
    }
}

pub(crate) fn handle_language_projection_batch_value(request: &Value) -> Result<Vec<u8>, String> {
    let header: LanguageProjectionBatchHeader = serde_json::from_value(request.clone())
        .map_err(|error| format!("decode structured projection request: {error}"))?;
    validate_projection_batch_header(&header)?;
    let projection_authority = crate::exact_source_projection::ExactProjectionAuthority {
        generation_identity_digest: header.generation_root_digest.clone(),
        parser_identity_digest: header.parser_identity_digest.clone(),
        query_pack_digest: header.query_pack_digest.clone(),
    };
    let projected_owners = header
        .owners
        .into_iter()
        .map(|owner| {
            validate_relative_owner(&owner.owner_path)?;
            let source_text = owner.decode_source_text()?;
            render_batch_owner_projection(
                &owner.owner_path,
                &owner.source_leaf_digest,
                &source_text,
                &projection_authority,
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    serde_json::to_vec(&json!({
        "schemaId": "agent.semantic-protocols.provider-language-projection-batch-response",
        "schemaVersion": "1",
        "languageId": header.language_id,
        "providerId": header.provider_id,
        "generationRootDigest": header.generation_root_digest,
        "owners": projected_owners,
    }))
    .map_err(|error| format!("encode structured projection response: {error}"))
}

fn validate_projection_batch_header(header: &LanguageProjectionBatchHeader) -> Result<(), String> {
    if header.schema_id != "agent.semantic-protocols.provider-language-projection-batch-request"
        || header.schema_version != "1"
        || header.language_id != "rust"
        || header.provider_id != "asp-rust"
    {
        return Err("projection batch header contract mismatch".to_string());
    }
    if header.workspace_identity.trim().is_empty()
        || header.generation_root_digest.trim().is_empty()
        || header.parser_identity_digest.trim().is_empty()
        || header.query_pack_digest.trim().is_empty()
        || header
            .base_generation_root_digest
            .as_deref()
            .is_some_and(str::is_empty)
        || header.owners.is_empty()
    {
        return Err("projection batch header requires generation identity and owners".to_string());
    }
    if header.owners.iter().any(|owner| {
        owner.source_leaf_digest.trim().is_empty()
            || owner
                .owner_path
                .extension()
                .and_then(|value| value.to_str())
                != Some("rs")
    }) {
        return Err("projection batch owners must be identified Rust sources".to_string());
    }
    let mut paths = BTreeSet::new();
    for owner in header.owners.iter().chain(&header.auxiliary_owners) {
        validate_relative_owner(&owner.owner_path)?;
        owner.decode_source_bytes()?;
        if owner.source_leaf_digest.trim().is_empty() || !paths.insert(&owner.owner_path) {
            return Err(
                "projection batch owner identities must be non-empty and unique".to_string(),
            );
        }
    }
    Ok(())
}

fn render_batch_owner_projection(
    owner_path: &Path,
    source_leaf_digest: &str,
    source: &str,
    projection_authority: &crate::exact_source_projection::ExactProjectionAuthority,
) -> Result<Value, String> {
    if source_leaf_digest.trim().is_empty() {
        return Err("projection owner sourceLeafDigest must be non-empty".to_string());
    }
    let relative_path = project_path(owner_path);
    let owner_id = format!("owner:{relative_path}");
    let items = projection_items(
        source,
        &relative_path,
        &owner_id,
        source_leaf_digest,
        Some(projection_authority),
    )?;
    let relations = items
        .iter()
        .map(|item| {
            json!({
                "from": {"kind": "owner", "id": owner_id},
                "kind": "contains",
                "to": {"kind": "item", "id": item["itemId"]},
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "ownerPath": relative_path,
        "sourceLeafDigest": source_leaf_digest,
        "items": items,
        "relations": relations,
    }))
}

struct LanguageProjectionOptions {
    owner: PathBuf,
    workspace: PathBuf,
    json: bool,
    help: bool,
}

impl LanguageProjectionOptions {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let args = args
            .into_iter()
            .map(|arg| {
                arg.into_string()
                    .map_err(|_| "projection arguments must be UTF-8")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut owner = None;
        let mut workspace = None;
        let mut json = false;
        let mut help = false;
        let mut index = 0;
        while let Some(argument) = args.get(index) {
            match argument.as_str() {
                "--workspace" => {
                    let value = args.get(index + 1).ok_or_else(language_projection_usage)?;
                    workspace = Some(PathBuf::from(value));
                    index += 2;
                }
                "--json" => {
                    json = true;
                    index += 1;
                }
                "--help" | "-h" => {
                    help = true;
                    index += 1;
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown projection option: {value}"));
                }
                value => {
                    if owner.replace(PathBuf::from(value)).is_some() {
                        return Err("projection accepts exactly one owner".to_string());
                    }
                    index += 1;
                }
            }
        }
        let workspace =
            workspace.unwrap_or(std::env::current_dir().map_err(|error| error.to_string())?);
        if help {
            return Ok(Self {
                owner: owner.unwrap_or_default(),
                workspace,
                json,
                help,
            });
        }
        let owner = owner.ok_or_else(language_projection_usage)?;
        validate_relative_owner(&owner)?;
        Ok(Self {
            owner,
            workspace,
            json,
            help,
        })
    }
}

fn render_language_projection(options: &LanguageProjectionOptions) -> Result<Value, String> {
    let workspace = options
        .workspace
        .canonicalize()
        .map_err(|error| format!("failed to resolve projection workspace: {error}"))?;
    let source_path = workspace.join(&options.owner);
    let source_path = source_path
        .canonicalize()
        .map_err(|error| format!("failed to resolve projection owner: {error}"))?;
    if !source_path.starts_with(&workspace)
        || source_path.extension().and_then(|value| value.to_str()) != Some("rs")
    {
        return Err("projection owner must be a Rust source inside the workspace".to_string());
    }
    let relative_path = source_path
        .strip_prefix(&workspace)
        .map_err(|error| error.to_string())?;
    let relative_path = project_path(relative_path);
    let module = parse_rust_file(&source_path);
    let source_id = format!("source:{relative_path}");
    let owner_id = format!("owner:{relative_path}");
    let owner_name = source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("module");
    let items = projection_items(&module.source, &relative_path, &owner_id, "", None)?;
    let mut relations = vec![json!({
        "from": {"kind": "source", "id": source_id},
        "kind": "contains",
        "to": {"kind": "owner", "id": owner_id},
    })];
    relations.extend(items.iter().map(|item| {
        json!({
            "from": {"kind": "owner", "id": owner_id},
            "kind": "contains",
            "to": {"kind": "item", "id": item["itemId"]},
        })
    }));
    Ok(json!({
        "schemaId": "agent.semantic-protocols.semantic-language-projection",
        "schemaVersion": "1",
        "protocolId": "agent.semantic-protocols.language-projection",
        "protocolVersion": "1",
        "languageId": "rust",
        "harness": {
            "harnessId": "rust-lang-project-harness",
            "parserAbi": "syn-v2-full-v1",
            "selectorDialect": "rust",
        },
        "sources": [{
            "sourceId": source_id,
            "path": relative_path,
            "sourceKind": source_kind(&options.owner),
        }],
        "owners": [{
            "ownerId": owner_id,
            "sourceId": source_id,
            "kind": "module",
            "name": owner_name,
        }],
        "items": items,
        "relations": relations,
    }))
}

fn projection_items(
    source: &str,
    relative_path: &str,
    owner_id: &str,
    source_leaf_digest: &str,
    projection_authority: Option<&crate::exact_source_projection::ExactProjectionAuthority>,
) -> Result<Vec<Value>, String> {
    let artifacts = crate::exact_source_parse_artifact::parse_owner_items_v1(source)?;
    let mut seen_selectors = BTreeSet::new();
    Ok(artifacts
        .into_iter()
        .filter_map(|artifact| {
            let encoded_identity =
                crate::structural_selector::encode_canonical_item_identity_path(&artifact.identity);
            let selector = format!("rust://{relative_path}#{encoded_identity}");
            seen_selectors.insert(selector.clone()).then(|| {
                let projections = if matches!(
                    artifact.identity.kind.as_str(),
                    "function" | "method" | "trait-function"
                ) {
                    projection_authority
                        .map(|authority| {
                            let code = source
                                .get(artifact.source_byte_start..artifact.source_byte_end)
                                .ok_or_else(|| {
                                    format!(
                                        "callable projection range is outside owner: {selector}"
                                    )
                                })?
                                .to_owned();
                            let resolved = crate::exact_source_projection::ResolvedExactItem {
                                canonical_selector:
                                    agent_semantic_content_identity::CanonicalItemSelector::new(
                                        artifact.identity.clone(),
                                        selector.clone(),
                                    ),
                                owner_path: relative_path.to_owned(),
                                identity: artifact.identity.clone(),
                                code,
                                source_byte_start: artifact.source_byte_start,
                                source_byte_end: artifact.source_byte_end,
                                owner_blob_digest: source_leaf_digest.to_owned(),
                                parser_artifact_digest: None,
                            };
                            crate::exact_source_projection::callable_skeleton_projection(
                                &resolved, authority,
                            )
                            .map(|payload| {
                                vec![json!({
                                    "projectionKind": "callable-skeleton",
                                    "payload": payload,
                                })]
                            })
                        })
                        .transpose()?
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                Ok(json!({
                    "itemId": encoded_identity.replace('/', ":"),
                    "ownerId": owner_id,
                    "kind": artifact.identity.kind.as_str(),
                    "name": artifact.identity.symbol.as_str(),
                    "selector": selector,
                    "sourceByteStart": artifact.source_byte_start,
                    "sourceByteEnd": artifact.source_byte_end,
                    "identity": projection_identity(&artifact.identity),
                    "projections": projections,
                }))
            })
        })
        .collect::<Result<Vec<_>, String>>()?)
}

fn projection_identity(identity: &agent_semantic_content_identity::CanonicalItemIdentity) -> Value {
    json!({
        "schemaId": "agent.semantic-protocols.canonical-language-item-identity",
        "schemaVersion": "1",
        "languageId": identity.language_id.as_str(),
        "kind": identity.kind.as_str(),
        "symbol": identity.symbol.as_str(),
        "scopes": identity.scopes.iter().map(|scope| json!({
            "relation": scope.relation.as_str(),
            "kind": scope.kind.as_str(),
            "symbol": scope.symbol.as_str(),
        })).collect::<Vec<_>>(),
    })
}

fn validate_relative_owner(owner: &Path) -> Result<(), String> {
    if owner.is_absolute()
        || owner.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("projection owner must be a relative workspace path".to_string());
    }
    Ok(())
}

fn project_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn source_kind(owner: &Path) -> &'static str {
    if owner
        .components()
        .any(|component| component.as_os_str() == "tests")
    {
        "test"
    } else {
        "source"
    }
}

fn language_projection_usage() -> String {
    "usage: asp-rust projection <relative-owner> --workspace <root> --json".to_string()
}
