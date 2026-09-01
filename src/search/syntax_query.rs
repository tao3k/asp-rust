use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::model::AspRustConfig;

use super::api::RustSearchOptions;
use super::context::{PackageSearchContext, search_contexts};
use super::format::{append_block, compact_locations, display_project_path, package_label};
use super::limits::SEARCH_HIT_LIMIT;

#[derive(Debug, Clone, PartialEq, Eq)]
enum RustSyntaxQuery {
    Use { public_only: bool, term: String },
}

#[derive(Debug, Clone)]
struct SyntaxFactHit {
    path: PathBuf,
    line: usize,
    fact_kind: &'static str,
    visibility: String,
    source_path: String,
    exposed_name: Option<String>,
}

impl SyntaxFactHit {
    fn relation_kind(&self) -> &'static str {
        match self.fact_kind {
            "reexport" => "reexports",
            _ => "imports",
        }
    }
}

pub(super) fn render_search_query(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let Some(syntax_query) = parse_rust_syntax_query(query) else {
        return render_unsupported_query(project_root, config, query, options);
    };
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        let hits = syntax_hits(&context, &syntax_query);
        let owners = owner_locations(&context, &hits);
        let mut block = format!(
            "[search-query] q={} pkg={} intent={} own={} fact={} routed=native-syntax\n",
            query,
            package_label(project_root, &context.package_root),
            syntax_query.kind(),
            owners.len(),
            hits.len()
        );
        let _ = writeln!(
            block,
            "|query intent={} term={} status={} routed=native-syntax hit={} selected={}",
            syntax_query.kind(),
            syntax_query.term(),
            if hits.is_empty() { "miss" } else { "hit" },
            hits.len(),
            owners.len()
        );
        for hit in hits.iter().take(SEARCH_HIT_LIMIT) {
            let owner = display_project_path(&context.package_root, &hit.path);
            let fact_name = hit.exposed_name.as_deref().unwrap_or_else(|| {
                hit.source_path
                    .rsplit("::")
                    .next()
                    .unwrap_or(hit.source_path.as_str())
            });
            let fact_id = format!(
                "rust:{}:{}:{}:{}",
                owner, hit.line, hit.fact_kind, fact_name
            );
            let _ = writeln!(
                block,
                "|fact {} kind={} source=native-parser owner={} line={} visibility={} name={} qualifiedName={} languageKind=use exported={} relation={} relationTarget={} query={}",
                compact_field(&fact_id),
                hit.fact_kind,
                owner,
                hit.line,
                hit.visibility,
                compact_field(fact_name),
                compact_field(&hit.source_path),
                hit.visibility == "public",
                hit.relation_kind(),
                compact_field(&hit.source_path),
                compact_field(syntax_query.term()),
            );
        }
        if options.output_view.as_deref() != Some("seeds") {
            for (owner, locations) in &owners {
                let _ = writeln!(
                    block,
                    "|owner {} hit_kind=syntax-query locations={} next=owner:{}",
                    owner,
                    compact_locations(locations),
                    owner
                );
            }
        }
        if !owners.is_empty() {
            let seed_limit = options.seeds.unwrap_or(8);
            let seed_owners = owners.keys().take(seed_limit).cloned().collect::<Vec<_>>();
            let _ = writeln!(block, "|seed owner:{}", seed_owners.join(","));
        }
        let _ = writeln!(
            block,
            "|synthesis algorithm=native-syntax-query scope=query summary=parser-owned-code-shaped-query selected_owners={} fact_count={}",
            owners.len(),
            hits.len()
        );
        append_block(&mut rendered, &block);
    }
    Ok(rendered)
}

fn render_unsupported_query(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<String, String> {
    let contexts = search_contexts(project_root, config, options)?;
    let mut rendered = String::new();
    for context in contexts {
        append_block(
            &mut rendered,
            &format!(
                "[search-query] q={} pkg={} kind=unknown own=0 handle=0 routed=none\n\
                 |query syntax=unknown term={} status=miss hit=0 selected=0 next=text:{}\n",
                query,
                package_label(project_root, &context.package_root),
                compact_field(query),
                compact_field(query)
            ),
        );
    }
    Ok(rendered)
}

fn parse_rust_syntax_query(query: &str) -> Option<RustSyntaxQuery> {
    let trimmed = query.trim();
    let (public_only, rest) = trimmed
        .strip_prefix("pub use ")
        .map(|rest| (true, rest))
        .or_else(|| {
            trimmed
                .strip_prefix("pub(crate) use ")
                .map(|rest| (false, rest))
        })
        .or_else(|| {
            trimmed
                .strip_prefix("pub(super) use ")
                .map(|rest| (false, rest))
        })
        .or_else(|| {
            trimmed
                .strip_prefix("pub(self) use ")
                .map(|rest| (false, rest))
        })
        .or_else(|| trimmed.strip_prefix("use ").map(|rest| (false, rest)))?;
    let term = rest
        .trim()
        .trim_end_matches(';')
        .split(" as ")
        .next()
        .unwrap_or(rest)
        .rsplit("::")
        .next()
        .unwrap_or(rest)
        .trim_matches(|ch: char| ch == '{' || ch == '}' || ch == ',' || ch.is_whitespace())
        .to_string();
    if term.is_empty() {
        return None;
    }
    Some(RustSyntaxQuery::Use { public_only, term })
}

fn syntax_hits(
    context: &PackageSearchContext,
    syntax_query: &RustSyntaxQuery,
) -> Vec<SyntaxFactHit> {
    match syntax_query {
        RustSyntaxQuery::Use { public_only, term } => context
            .parsed_modules
            .iter()
            .flat_map(|module| {
                module
                    .syntax_facts
                    .use_statements
                    .iter()
                    .filter(move |statement| {
                        !*public_only || visibility_label(&statement.visibility) == "public"
                    })
                    .flat_map(move |statement| {
                        let import_hits = statement.imports.iter().filter_map(move |import| {
                            if segments_match(
                                &import.segments,
                                import.exposed_name.as_deref(),
                                term,
                            ) {
                                Some(SyntaxFactHit {
                                    path: module.report.path.clone(),
                                    line: statement.line,
                                    fact_kind: if visibility_label(&statement.visibility)
                                        == "public"
                                    {
                                        "reexport"
                                    } else {
                                        "import"
                                    },
                                    visibility: visibility_label(&statement.visibility).to_string(),
                                    source_path: import.segments.join("::"),
                                    exposed_name: import.exposed_name.clone(),
                                })
                            } else {
                                None
                            }
                        });
                        let reexport_hits =
                            statement.reexports.iter().filter_map(move |reexport| {
                                if segments_match(
                                    &reexport.source_segments,
                                    Some(reexport.exposed_name.as_str()),
                                    term,
                                ) {
                                    Some(SyntaxFactHit {
                                        path: module.report.path.clone(),
                                        line: reexport.line,
                                        fact_kind: "reexport",
                                        visibility: visibility_label(&reexport.visibility)
                                            .to_string(),
                                        source_path: reexport.source_segments.join("::"),
                                        exposed_name: Some(reexport.exposed_name.clone()),
                                    })
                                } else {
                                    None
                                }
                            });
                        import_hits.chain(reexport_hits)
                    })
            })
            .collect(),
    }
}

fn owner_locations(
    context: &PackageSearchContext,
    hits: &[SyntaxFactHit],
) -> BTreeMap<String, Vec<String>> {
    hits.iter()
        .fold(BTreeMap::<String, Vec<String>>::new(), |mut owners, hit| {
            owners
                .entry(display_project_path(&context.package_root, &hit.path))
                .or_default()
                .push(format!("{}:1", hit.line));
            owners
        })
}

fn segments_match(segments: &[String], exposed_name: Option<&str>, term: &str) -> bool {
    segments.iter().any(|segment| segment == term)
        || exposed_name == Some(term)
        || segments.join("::").contains(term)
}

fn visibility_label(visibility: &impl std::fmt::Debug) -> &'static str {
    match format!("{visibility:?}").as_str() {
        "Public" => "public",
        "Crate" => "crate",
        "Super" => "super",
        "SelfScope" => "self",
        "Private" => "private",
        _ => "restricted",
    }
}

fn compact_field(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_whitespace() { '-' } else { ch })
        .collect()
}

impl RustSyntaxQuery {
    fn kind(&self) -> &'static str {
        match self {
            RustSyntaxQuery::Use { .. } => "rust-use",
        }
    }

    fn term(&self) -> &str {
        match self {
            RustSyntaxQuery::Use { term, .. } => term,
        }
    }
}
