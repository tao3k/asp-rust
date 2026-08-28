//! Hook-oriented query command mapped onto provider-owned search views.

use std::ffi::OsString;

use super::query_options::QueryOptions;

pub(super) enum QueryCommand {
    Help,
    ExactSource(ExactSourceQuery),
    TreeSitter(Box<TreeSitterQuery>),
}

pub(crate) struct ExactSourceQuery {
    pub(crate) selector: String,
    pub(crate) projection: String,
    pub(crate) json: bool,
    pub(crate) provider_id: String,
    pub(crate) exact_request_stdin: bool,
    pub(crate) source_snapshot_envelope: Option<std::path::PathBuf>,
}

#[derive(Default)]
struct ExactQueryAuthority {
    exact_request_stdin: bool,
    from_hook: Option<String>,
    source_snapshot_envelope: Option<std::path::PathBuf>,
}

impl ExactQueryAuthority {
    fn is_empty(&self) -> bool {
        !self.exact_request_stdin
            && self.from_hook.is_none()
            && self.source_snapshot_envelope.is_none()
    }
}

pub(super) struct TreeSitterQuery {
    pub(crate) source: Option<String>,
    pub(crate) catalog_id: Option<String>,
    pub(crate) selector: Option<String>,
    pub(crate) captures: Vec<String>,
    pub(crate) node_types: Vec<String>,
    pub(crate) fields: Vec<String>,
    pub(crate) predicates_json: Option<String>,
    pub(crate) workspace_root: std::path::PathBuf,
    pub(crate) json: bool,
    pub(crate) provider_id: Option<String>,
}

pub(super) fn query_guide_kind(args: &[OsString]) -> bool {
    args.first().and_then(|arg| arg.to_str()) == Some("guide")
}

pub(super) fn parse_query(
    args: impl IntoIterator<Item = OsString>,
) -> Result<QueryCommand, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    if let Some(options) = parse_tree_sitter_query(&args)? {
        return Ok(QueryCommand::TreeSitter(Box::new(options)));
    }
    let (args, authority) = extract_exact_query_authority(args)?;
    let options = QueryOptions::parse(args)?;
    if options.help {
        return Ok(QueryCommand::Help);
    }
    let wants_direct_source_items = options
        .selector
        .as_deref()
        .is_some_and(is_exact_item_selector);
    if wants_direct_source_items {
        let selector = options
            .selector
            .clone()
            .ok_or_else(|| "exact source query requires a selector".to_string())?;
        let provider_id = options.provider_id.ok_or_else(|| {
            "exact source query requires --asp-provider-id v1 authority".to_string()
        })?;
        let projection = options
            .projection
            .clone()
            .ok_or_else(|| "exact source query requires --projection".to_string())?;
        return Ok(QueryCommand::ExactSource(ExactSourceQuery {
            selector,
            projection,
            json: options.json,
            provider_id,
            exact_request_stdin: authority.exact_request_stdin,
            source_snapshot_envelope: authority.source_snapshot_envelope,
        }));
    }
    if !authority.is_empty() {
        return Err(
            "source snapshot and typed projection identity options require an exact source selector"
                .to_string(),
        );
    }
    Err(
        "rust query requires an exact --selector; use `asp rust search owner <owner-path> items --query <symbol> --workspace . --view seeds` for owner or symbol discovery"
            .to_string(),
    )
}

fn parse_tree_sitter_query(args: &[OsString]) -> Result<Option<TreeSitterQuery>, String> {
    let is_tree_sitter_query = args
        .iter()
        .any(|argument| matches!(argument.to_str(), Some("--treesitter-query" | "--catalog")));
    if !is_tree_sitter_query {
        return Ok(None);
    }
    let mut options = TreeSitterQuery {
        source: None,
        catalog_id: None,
        selector: None,
        captures: Vec::new(),
        node_types: Vec::new(),
        fields: Vec::new(),
        predicates_json: None,
        workspace_root: std::path::PathBuf::from("."),
        json: false,
        provider_id: None,
    };
    let mut index = 0;
    while index < args.len() {
        let argument = args[index]
            .to_str()
            .ok_or_else(|| "tree-sitter query arguments must be UTF-8".to_string())?;
        match argument {
            "--treesitter-query" => {
                options.source = Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--catalog" => {
                options.catalog_id = Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--selector" => {
                options.selector = Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--workspace" => {
                options.workspace_root =
                    std::path::PathBuf::from(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--asp-syntax-query-captures" => {
                options.captures = tree_sitter_csv_option(args, &mut index, argument)?;
            }
            "--asp-syntax-query-node-types" => {
                options.node_types = tree_sitter_csv_option(args, &mut index, argument)?;
            }
            "--asp-syntax-query-fields" => {
                options.fields = tree_sitter_csv_option(args, &mut index, argument)?;
            }
            "--asp-syntax-query-predicates-json" => {
                options.predicates_json =
                    Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            "--json" => options.json = true,
            "--asp-provider-id" => {
                options.provider_id = Some(tree_sitter_option_value(args, &mut index, argument)?);
            }
            _ => return Err(format!("unknown tree-sitter query option: {argument}")),
        }
        index += 1;
    }
    if options.source.is_some() == options.catalog_id.is_some() {
        return Err(
            "tree-sitter query requires exactly one of --treesitter-query or --catalog".to_string(),
        );
    }
    Ok(Some(options))
}

fn tree_sitter_option_value(
    args: &[OsString],
    index: &mut usize,
    option: &str,
) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn tree_sitter_csv_option(
    args: &[OsString],
    index: &mut usize,
    option: &str,
) -> Result<Vec<String>, String> {
    Ok(tree_sitter_option_value(args, index, option)?
        .split(',')
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect())
}

fn extract_exact_query_authority(
    args: impl IntoIterator<Item = OsString>,
) -> Result<(Vec<OsString>, ExactQueryAuthority), String> {
    let mut filtered = Vec::new();
    let mut authority = ExactQueryAuthority::default();
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        if argument == "--asp-exact-request-stdin" {
            if authority.exact_request_stdin {
                return Err("--asp-exact-request-stdin may be supplied only once".to_string());
            }
            authority.exact_request_stdin = true;
        } else if argument == "--from-hook" {
            if authority.from_hook.is_some() {
                return Err("--from-hook may be supplied only once".to_string());
            }
            let provenance = args
                .next()
                .ok_or_else(|| "--from-hook requires a provenance kind".to_string())?;
            let provenance = provenance
                .into_string()
                .map_err(|_| "--from-hook provenance must be UTF-8".to_string())?;
            if provenance.is_empty() {
                return Err("--from-hook provenance must not be empty".to_string());
            }
            authority.from_hook = Some(provenance);
        } else if argument == "--source-snapshot-envelope" {
            if authority.source_snapshot_envelope.is_some() {
                return Err("--source-snapshot-envelope may be supplied only once".to_string());
            }
            let path = args.next().ok_or_else(|| {
                "--source-snapshot-envelope requires a JSON file path".to_string()
            })?;
            authority.source_snapshot_envelope = Some(std::path::PathBuf::from(path));
        } else {
            filtered.push(argument);
        }
    }
    Ok((filtered, authority))
}

fn is_exact_item_selector(selector: &str) -> bool {
    let Some(selector) = selector.strip_prefix("rust://") else {
        return false;
    };
    let Some((owner_path, item_path)) = selector.split_once("#item/") else {
        return false;
    };
    !owner_path.is_empty()
        && !item_path.is_empty()
        && item_path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

pub(super) fn print_query_guide() {
    println!(
        r#"[query-guide] lang=rust provider=asp-rust protocol=query-guide.v1 root=.
|contract discovery="search owner" materialization="query --selector"
|contract projectionRequired=true
|contract projections=source,callable-skeleton
|contract sourceOutput=pure-source header=false legend=false metadata=false
|contract jsonOutput=explicit-only
|mode source command="query --selector <exact-structural-selector> --projection source --workspace <WORKSPACE>" output=pure-source
|mode skeleton command="query --selector <exact-structural-selector> --workspace <WORKSPACE> --projection callable-skeleton" output=callable-skeleton
|action discover mapsTo="search owner <owner-path> items --query <symbol> --workspace <WORKSPACE> --view seeds"
|action materialize mapsTo="query --selector <structural-selector> --projection source --workspace <WORKSPACE>"
|avoid owner-path-as-query,line-range-selector,raw-read"#
    );
}

pub(super) fn print_query_help() {
    println!(
        "asp-rust query --selector 'rust://OWNER#item/KIND/NAME' --projection source|callable-skeleton [--workspace WORKSPACE] [--source-snapshot-envelope JSON-FILE]\n\
asp-rust query --treesitter-query QUERY [--workspace WORKSPACE]\n\
asp-rust query --catalog flow-lite --where 'source.call=NAME sink.constructs=TYPE scope.fn=FUNCTION' [<workspace-root>] [--json] [--workspace WORKSPACE]\n\
asp-rust query --from-hook KIND --selector SELECTOR --source-snapshot-envelope JSON-FILE --projection source --json --asp-provider-id ID --asp-parser-identity-digest DIGEST --asp-query-pack-digest DIGEST [--workspace WORKSPACE]\n\
asp-rust search dependency <crate-or-package> [items docs-use tests] [--view seeds] [--workspace WORKSPACE]\n\
asp-rust search guide [--workspace WORKSPACE]\n\n\
Maps hook-denied raw reads and broad searches into parser-owned search output.\n\
Owner and symbol discovery is owned by `search owner`; `query` accepts only exact structural selectors or exact Tree-sitter/relation contracts.\n\
Dependency search is manifest-first: inspect Cargo.toml/Cargo.lock facts, import owners, public API/docs-use, and tests before web or docs.rs search.\n\
Flow-lite native relation queries emit compact locator/provenance frontiers or semantic-flow-lite.v1 JSON without running CodeQL.\n\
Use `asp rust search owner OWNER items --query SYMBOL --workspace . --view seeds` to discover exact item selectors.\n\
Use --workspace WORKSPACE when the exact selector is workspace-relative; query never accepts an owner path as a positional discovery shortcut.\n\
Use --source-snapshot-envelope JSON-FILE with an exact selector to derive an editor-buffer Merkle root from asp.exact-source-snapshot-envelope.v1.\n\
Flow-lite query forms accept one positional workspace root for ABI corpus compatibility.\n\
Exact projection is required; use `--projection source` for pure source or `--projection callable-skeleton` for signatures without bodies."
    );
}

#[cfg(test)]
#[path = "../../tests/unit/cli/query/authority.rs"]
mod tests;
