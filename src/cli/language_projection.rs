//! Query-free, parser-owned language projection for one Rust owner.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Value, json};

use crate::parser::parse_rust_file;

pub(super) fn run_language_projection(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<ExitCode, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args.len() == 1 && args[0] == "--batch-stdin" {
        return run_language_projection_batch_stdin();
    }
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
    transport: String,
    owners: Vec<LanguageProjectionBatchOwner>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LanguageProjectionBatchOwner {
    owner_path: PathBuf,
    source_leaf_digest: String,
    byte_length: usize,
}

fn run_language_projection_batch_stdin() -> Result<ExitCode, String> {
    use std::io::Read as _;

    let mut input = std::io::stdin().lock();
    let mut header_length_bytes = [0_u8; 4];
    input
        .read_exact(&mut header_length_bytes)
        .map_err(|error| format!("failed to read projection batch header length: {error}"))?;
    let header_length = u32::from_be_bytes(header_length_bytes) as usize;
    let mut header_bytes = Vec::new();
    header_bytes
        .try_reserve_exact(header_length)
        .map_err(|error| format!("projection batch header allocation failed: {error}"))?;
    header_bytes.resize(header_length, 0);
    input
        .read_exact(&mut header_bytes)
        .map_err(|error| format!("failed to read projection batch header: {error}"))?;
    let header: LanguageProjectionBatchHeader = serde_json::from_slice(&header_bytes)
        .map_err(|error| format!("invalid projection batch header: {error}"))?;
    validate_projection_batch_header(&header)?;
    let projection_authority = crate::exact_source_projection::ExactProjectionAuthority {
        projection_kind: "callable-skeleton".to_owned(),
        generation_identity_digest: header.generation_root_digest.clone(),
        parser_identity_digest: header.parser_identity_digest.clone(),
        query_pack_digest: header.query_pack_digest.clone(),
    };

    let mut projected_owners = Vec::new();
    projected_owners
        .try_reserve_exact(header.owners.len())
        .map_err(|error| format!("projection batch owner allocation failed: {error}"))?;
    for owner in header.owners {
        validate_relative_owner(&owner.owner_path)?;
        if owner
            .owner_path
            .extension()
            .and_then(|value| value.to_str())
            != Some("rs")
        {
            return Err(format!(
                "projection batch owner must be a Rust source: {}",
                owner.owner_path.display()
            ));
        }
        let mut source_bytes = Vec::new();
        source_bytes
            .try_reserve_exact(owner.byte_length)
            .map_err(|error| format!("projection owner allocation failed: {error}"))?;
        source_bytes.resize(owner.byte_length, 0);
        input.read_exact(&mut source_bytes).map_err(|error| {
            format!(
                "failed to read projection owner frame {}: {error}",
                owner.owner_path.display()
            )
        })?;
        let source = std::str::from_utf8(&source_bytes).map_err(|error| {
            format!(
                "projection owner is not UTF-8 {}: {error}",
                owner.owner_path.display()
            )
        })?;
        projected_owners.push(render_batch_owner_projection(
            &owner.owner_path,
            &owner.source_leaf_digest,
            source,
            &projection_authority,
        )?);
    }
    let mut trailing = [0_u8; 1];
    if input
        .read(&mut trailing)
        .map_err(|error| format!("failed to finish projection batch input: {error}"))?
        != 0
    {
        return Err("projection batch input contains trailing bytes".to_string());
    }

    println!(
        "{}",
        json!({
            "schemaId": "asp.provider-language-projection-batch-response.v1",
            "schemaVersion": "1",
            "languageId": header.language_id,
            "providerId": header.provider_id,
            "generationRootDigest": header.generation_root_digest,
            "owners": projected_owners,
        })
    );
    Ok(ExitCode::SUCCESS)
}

fn validate_projection_batch_header(header: &LanguageProjectionBatchHeader) -> Result<(), String> {
    if header.schema_id != "asp.provider-language-projection-batch-request.v1"
        || header.schema_version != "1"
        || header.language_id != "rust"
        || header.provider_id != "rs-harness"
        || header.transport != "framed-stdin-v1"
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
                let projections = if artifact.identity.kind.as_str() == "function" {
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
                                    crate::canonical_item_identity::CanonicalItemSelectorV1::new(
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
                                "rs-harness",
                                &resolved,
                                authority,
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

fn projection_identity(
    identity: &crate::canonical_item_identity::CanonicalItemIdentityV1,
) -> Value {
    json!({
        "schemaId": "asp.canonical-language-item-identity.v1",
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
    "usage: rs-harness projection <relative-owner> --workspace <root> --json".to_string()
}
