//! Shared semantic-search JSON envelope for CLI search output.

use std::path::Path;

use serde_json::{Map, Value, json};

use crate::cli::{
    attach_syntax_refs_to_search_items, bool_field, canonical_owner_path,
    canonical_query_set_terms, canonicalize_read_field, display_path, graph_seed_fragment,
    header_package, input_detection_from_header, insert_if_some, insert_if_usize, location,
    location_from_node, merge_seed_fragment_search_synthesis, next_field, parse_edge_kind,
    parse_fields, parse_next_actions, push_synthesis, query_set_search_synthesis, string_field,
};

const SCHEMA_ID: &str = "agent.semantic-protocols.semantic-search-packet";
const SCHEMA_VERSION: &str = "1";

pub(in crate::cli) struct SemanticSearchJsonOptions {
    pub(in crate::cli) view: String,
    pub(in crate::cli) query: Option<String>,
    pub(in crate::cli) command_label: String,
    pub(in crate::cli) trace: bool,
    pub(in crate::cli) explain: bool,
    pub(in crate::cli) output_view: Option<String>,
    pub(in crate::cli) depth: Option<usize>,
    pub(in crate::cli) dir: Option<String>,
    pub(in crate::cli) edges: Vec<String>,
    pub(in crate::cli) per_owner: Option<usize>,
    pub(in crate::cli) seeds: Option<usize>,
    pub(in crate::cli) owners: Option<usize>,
    pub(in crate::cli) hits: Option<usize>,
    pub(in crate::cli) package: Option<String>,
    pub(in crate::cli) owner: Option<String>,
    pub(in crate::cli) dependency: Option<String>,
    pub(in crate::cli) scope: Option<String>,
    pub(in crate::cli) lines: bool,
    pub(in crate::cli) pipes: Vec<String>,
    pub(in crate::cli) query_set: Vec<String>,
}

pub(in crate::cli) fn render_search_json(
    project_root: &Path,
    options: &SemanticSearchJsonOptions,
    rendered: &str,
) -> Result<String, String> {
    let packet = build_search_packet_value(project_root, options, rendered);
    serde_json::to_string(&packet).map_err(|error| format!("failed to render search JSON: {error}"))
}

fn build_search_packet_value(
    project_root: &Path,
    options: &SemanticSearchJsonOptions,
    rendered: &str,
) -> Value {
    let (header_kind, mut header_fields) = parse_header(rendered, options);
    enrich_header_fields(&mut header_fields, options);

    let mut packages = Vec::new();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut owners = Vec::new();
    let mut items = Vec::new();
    let mut hits = Vec::new();
    let mut type_surfaces = Vec::new();
    let mut semantic_handles = Vec::new();
    let mut native_syntax_facts = Vec::new();
    let mut findings = Vec::new();
    let mut next_actions = Vec::new();
    let mut notes = Vec::new();
    let mut search_synthesis = None;
    let mut seed_fragments = Vec::<String>::new();
    let mut current_owner = None::<String>;
    if options.view == "policy" {
        semantic_handles = crate::search::policy::policy_semantic_handles_for_query(
            options.query.as_deref().unwrap_or_default(),
        );
    }

    for rendered_line in rendered.lines() {
        if let Some(seed_fragment) = graph_seed_fragment(rendered_line) {
            seed_fragments.push(seed_fragment);
            continue;
        }
        let Some(line) = rendered_line.strip_prefix('|') else {
            continue;
        };
        let mut tokens = line.split_whitespace();
        let Some(tag) = tokens.next() else {
            continue;
        };
        let remaining = tokens.collect::<Vec<_>>();
        match tag {
            "node" => push_node(&remaining, &mut nodes, &mut next_actions),
            "package" => push_package(
                &remaining,
                &mut packages,
                &mut nodes,
                &mut next_actions,
                &mut seed_fragments,
            ),
            "dep" => push_dependency_node(&remaining, &mut nodes),
            "owner" => {
                if let Some(owner_path) =
                    push_owner(options, &remaining, &mut owners, &mut next_actions)
                {
                    current_owner = Some(owner_path);
                }
            }
            "item" => push_item(
                &remaining,
                current_owner.as_deref(),
                &mut items,
                &mut next_actions,
            ),
            "hot" => push_hot(&remaining, current_owner.as_deref(), &mut next_actions),
            "edge" => push_edge(&remaining, &mut edges),
            "find" => push_finding(&remaining, &mut findings),
            "next" => push_next_actions(remaining.join(" "), None, &mut next_actions),
            "def" | "call" | "api" => push_hit(tag, &remaining, &mut hits),
            "fact" => push_native_syntax_fact(&remaining, &mut native_syntax_facts),
            "handle" if options.view != "policy" => {
                push_handle(&remaining, &mut semantic_handles);
            }
            "handle" => {}
            "external-type" => push_external_type_hit(&remaining, &mut hits, &mut type_surfaces),
            "api-candidate" => push_api_candidate(&remaining, &mut hits),
            "test" => push_test_node(&remaining, &mut nodes, &mut next_actions),
            "synthesis" => push_synthesis(&remaining, &mut search_synthesis),
            "seed" => seed_fragments.push(remaining.join(" ")),
            "note" => push_note(&remaining, &mut notes),
            "feat" | "feature" | "cfg" | "target" | "pat" => {
                push_fact_note(tag, &remaining, &mut notes)
            }
            _ => push_raw_note(tag, &remaining, &mut notes),
        }
    }
    if search_synthesis.is_none() && !options.query_set.is_empty() {
        search_synthesis = query_set_search_synthesis(&owners, &nodes, &seed_fragments);
    }
    merge_seed_fragment_search_synthesis(
        &mut search_synthesis,
        schema_view(options),
        &seed_fragments,
    );
    let input_detection = if options.view == "ingest" {
        input_detection_from_header(&header_fields)
    } else {
        None
    };
    if let Some(input_detection) = input_detection {
        return packet_with_input_detection(
            project_root,
            options,
            header_kind,
            header_fields,
            input_detection,
            PacketCollections {
                packages,
                nodes,
                edges,
                owners,
                items,
                hits,
                type_surfaces,
                semantic_handles,
                native_syntax_facts,
                findings,
                next_actions,
                notes,
                search_synthesis,
            },
        );
    }

    base_packet(
        project_root,
        options,
        header_kind,
        header_fields,
        PacketCollections {
            packages,
            nodes,
            edges,
            owners,
            items,
            hits,
            type_surfaces,
            semantic_handles,
            native_syntax_facts,
            findings,
            next_actions,
            notes,
            search_synthesis,
        },
    )
}

struct PacketCollections {
    packages: Vec<Value>,
    nodes: Vec<Value>,
    edges: Vec<Value>,
    owners: Vec<Value>,
    items: Vec<Value>,
    hits: Vec<Value>,
    type_surfaces: Vec<Value>,
    semantic_handles: Vec<Value>,
    native_syntax_facts: Vec<Value>,
    findings: Vec<Value>,
    next_actions: Vec<Value>,
    notes: Vec<Value>,
    search_synthesis: Option<Value>,
}

fn base_packet(
    project_root: &Path,
    options: &SemanticSearchJsonOptions,
    header_kind: String,
    header_fields: Map<String, Value>,
    mut collections: PacketCollections,
) -> Value {
    let query_set_terms = canonical_query_set_terms(
        &options.view,
        options.query.as_deref(),
        &options.query_set,
        &header_fields,
    );
    let syntax_refs = attach_syntax_refs_to_search_items(&mut collections.items);
    append_native_syntax_relation_edges(&collections.native_syntax_facts, &mut collections.edges);
    let mut packet = json!({
        "schemaId": SCHEMA_ID,
        "schemaVersion": SCHEMA_VERSION,
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "languageId": "rust",
        "providerId": "rs-harness",
        "binary": "rs-harness",
        "namespace": "agent.semantic-protocols.semantic-language",
        "method": format!("search/{}", schema_view(options)),
        "projectRoot": display_path(project_root),
        "view": schema_view(options),
        "renderMode": render_mode(options),
        "header": {
            "kind": header_kind,
            "fields": header_fields,
        },
        "packages": collections.packages,
        "nodes": collections.nodes,
        "edges": collections.edges,
        "owners": collections.owners,
        "items": collections.items,
        "hits": collections.hits,
        "typeSurfaces": collections.type_surfaces,
        "semanticHandles": collections.semantic_handles,
        "nativeSyntaxFacts": collections.native_syntax_facts,
        "findings": collections.findings,
        "nextActions": collections.next_actions,
        "notes": collections.notes,
    });
    if options.view == "reasoning" {
        packet["avoidNextActions"] = if options.dependency.is_some() {
            json!([
                {
                    "kind": "web-search",
                    "target": "external-docs",
                    "reason": "dependency-profile-local-first"
                },
                {
                    "kind": "docs.rs-search",
                    "target": "external-docs",
                    "reason": "dependency-profile-local-first"
                },
                {
                    "kind": "raw-read",
                    "target": "source",
                    "reason": "reasoning-profile"
                }
            ])
        } else {
            json!([
                {
                    "kind": "raw-read",
                    "target": "source",
                    "reason": "reasoning-profile"
                }
            ])
        };
    }
    if let Some(search_synthesis) = collections.search_synthesis {
        packet["searchSynthesis"] = search_synthesis;
    }
    if reasoning_profiles_enabled(options) {
        packet["reasoningProfiles"] = rust_reasoning_profiles();
    }
    if let Some(query) = options.query.as_deref() {
        packet["query"] = json!(query);
    }
    if let Some(syntax_refs) = syntax_refs {
        packet["syntaxQueryRef"] = json!(syntax_refs.query_ref);
        packet["syntaxMatchRefs"] = json!(syntax_refs.match_refs);
        packet["syntaxCaptureRefs"] = json!(syntax_refs.capture_refs);
        if let Some(anchor) = syntax_refs.anchor {
            packet["syntaxAnchor"] = anchor;
        }
    }
    if !query_set_terms.is_empty() {
        packet["querySet"] = json!(
            query_set_terms
                .iter()
                .map(|term| {
                    json!({
                        "value": term,
                        "kind": query_set_kind(options),
                        "selector": "exact"
                    })
                })
                .collect::<Vec<_>>()
        );
        packet["queryComposition"] = json!({
            "mode": "query-set",
            "view": schema_view(options),
            "selector": "exact-set",
            "merge": [
                "packages",
                "nodes",
                "edges",
        "owners",
        "items",
        "hits",
        "typeSurfaces",
        "nativeSyntaxFacts",
        "findings",
        "nextActions",
                "notes"
            ],
            "fields": {
                "count": query_set_terms.len()
            }
        });
    }
    let package_name = options
        .package
        .clone()
        .or_else(|| header_package(&packet).map(ToOwned::to_owned));
    if let Some(package_name) = package_name {
        packet["packageName"] = json!(package_name);
    }
    packet
}

#[path = "native_syntax.rs"]
mod native_syntax;
use native_syntax::append_native_syntax_relation_edges;

fn reasoning_profiles_enabled(options: &SemanticSearchJsonOptions) -> bool {
    matches!(render_mode(options), "graph" | "seeds" | "both" | "facts")
}

fn rust_reasoning_profiles() -> Value {
    json!([
        {
            "profile": "owner-query",
            "description": "Return owner-local matching items, read locators, tests, and optional dependency usage for a concrete owner plus query.",
            "selectors": [
                { "kind": "owner", "alias": "O", "targetRole": "path", "required": true },
                { "kind": "query", "alias": "Q", "targetRole": "term", "required": true }
            ],
            "returns": ["items", "tests", "dependency-usage"],
            "frontier": ["O.items", "Q.owner", "Q.tests"],
            "fields": { "source": "search-guide" }
        },
        {
            "profile": "query-deps",
            "description": "Return owners, imports, local dependency API/docs usage, crate-source frontier, and usage tests for a concrete query plus dependency.",
            "selectors": [
                { "kind": "query", "alias": "Q", "targetRole": "term", "required": true },
                { "kind": "dependency", "alias": "D", "targetRole": "pkg", "required": true }
            ],
            "returns": ["owners", "imports", "local-docs", "crate-source", "usage-tests"],
            "frontier": ["Q.owner", "D.public-api", "D.crate-source", "D.tests"],
            "avoid": ["web-search", "docs.rs-search"],
            "fields": { "source": "search-guide", "docs": "local-first" }
        },
        {
            "profile": "owner-tests",
            "description": "Return covering tests, test entrypoints, and fixture handles for a concrete owner.",
            "selectors": [
                { "kind": "owner", "alias": "O", "targetRole": "path", "required": true }
            ],
            "returns": ["covering-tests", "test-entrypoints", "fixtures"],
            "frontier": ["O.tests", "T.owner"],
            "fields": { "source": "search-guide" }
        },
        {
            "profile": "feature-cfg",
            "description": "Return cfg gates, related owners, and verification surfaces for a concrete Cargo feature.",
            "selectors": [
                { "kind": "feature", "alias": "F", "targetRole": "feature", "required": true }
            ],
            "returns": ["cfg-gates", "owners", "verification-surfaces"],
            "frontier": ["F.cfg", "F.owner", "F.tests"],
            "fields": { "source": "search-guide" }
        },
        {
            "profile": "finding-frontier",
            "description": "Return affected owners, tests, and verification actions for a concrete finding and optional owner.",
            "selectors": [
                { "kind": "finding", "alias": "F", "targetRole": "finding", "required": true },
                { "kind": "owner", "alias": "O", "targetRole": "path", "required": false }
            ],
            "returns": ["affected-owners", "tests", "verification-actions"],
            "frontier": ["F.owner", "F.tests", "O.policy"],
            "fields": { "source": "search-guide" }
        }
    ])
}

fn query_set_kind(options: &SemanticSearchJsonOptions) -> &'static str {
    match options.view.as_str() {
        "dependency" | "deps" => "dependency",
        "owner" | "tests" => "owner",
        "features" => "feature",
        "cfg" => "cfg",
        "api" | "docs" | "docs-use" | "public-external-types" => "api",
        "symbol" | "callsite" | "import" => "symbol",
        "lexical" => "text",
        _ => "custom",
    }
}

fn packet_with_input_detection(
    project_root: &Path,
    options: &SemanticSearchJsonOptions,
    header_kind: String,
    header_fields: Map<String, Value>,
    input_detection: Value,
    collections: PacketCollections,
) -> Value {
    let mut packet = base_packet(
        project_root,
        options,
        header_kind,
        header_fields,
        collections,
    );
    packet["inputDetection"] = input_detection;
    packet
}

fn parse_header(
    rendered: &str,
    options: &SemanticSearchJsonOptions,
) -> (String, Map<String, Value>) {
    let fallback = format!("search-{}", options.command_label.replace(' ', "-"));
    let Some(line) = rendered.lines().find(|line| line.starts_with('[')) else {
        return (fallback, Map::new());
    };
    let Some((kind, tail)) = line.strip_prefix('[').and_then(|line| line.split_once(']')) else {
        return (fallback, Map::new());
    };
    (kind.to_string(), parse_fields(tail.split_whitespace()))
}

fn enrich_header_fields(fields: &mut Map<String, Value>, options: &SemanticSearchJsonOptions) {
    insert_if_some(fields, "packageSelector", options.package.as_deref());
    insert_if_some(fields, "ownerSelector", options.owner.as_deref());
    insert_if_some(fields, "dependencySelector", options.dependency.as_deref());
    insert_if_some(fields, "scope", options.scope.as_deref());
    if !options.pipes.is_empty() {
        fields.insert("pipes".to_string(), json!(options.pipes));
    }
    if options.trace {
        fields.insert("trace".to_string(), Value::Bool(true));
    }
    if options.explain {
        fields.insert("explain".to_string(), Value::Bool(true));
    }
    if options.lines {
        fields.insert("lines".to_string(), Value::Bool(true));
    }
    insert_if_some(fields, "dir", options.dir.as_deref());
    insert_if_usize(fields, "depth", options.depth);
    insert_if_usize(fields, "perOwner", options.per_owner);
    insert_if_usize(fields, "seeds", options.seeds);
    insert_if_usize(fields, "owners", options.owners);
    insert_if_usize(fields, "hits", options.hits);
    if !options.edges.is_empty() {
        fields.insert("edge".to_string(), json!(options.edges));
    }
}

fn push_package(
    tokens: &[&str],
    packages: &mut Vec<Value>,
    nodes: &mut Vec<Value>,
    next_actions: &mut Vec<Value>,
    seed_fragments: &mut Vec<String>,
) {
    let Some(id) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    if let Some(next) = next_field(&fields) {
        seed_fragments.push(next.clone());
        next_actions.extend(parse_next_actions(next, None));
    }
    packages.push(json!({
        "id": id,
        "fields": fields.clone(),
    }));
    nodes.push(json!({
        "id": format!("P:{id}"),
        "kind": "package",
        "path": id,
        "fields": fields,
    }));
}

fn push_node(tokens: &[&str], nodes: &mut Vec<Value>, next_actions: &mut Vec<Value>) {
    let Some(id) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    next_actions.extend(
        next_field(&fields)
            .map(|next| parse_next_actions(next, None))
            .unwrap_or_default(),
    );
    nodes.push(json!({
        "id": id,
        "kind": string_field(&fields, "kind").unwrap_or_else(|| "node".to_string()),
        "fields": fields,
    }));
}

fn push_dependency_node(tokens: &[&str], nodes: &mut Vec<Value>) {
    let Some(id) = tokens.first() else {
        return;
    };
    nodes.push(json!({
        "id": format!("D:{id}"),
        "kind": "dependency",
        "fields": parse_fields(tokens.iter().skip(1).copied()),
    }));
}

fn push_owner(
    options: &SemanticSearchJsonOptions,
    tokens: &[&str],
    owners: &mut Vec<Value>,
    next_actions: &mut Vec<Value>,
) -> Option<String> {
    let path = canonical_owner_path(
        tokens.first()?,
        options.owner.as_deref(),
        options.query.as_deref(),
    );
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let role = string_field(&fields, "role").unwrap_or_else(|| "source".to_string());
    let public = bool_field(&fields, "public")
        .or_else(|| bool_field(&fields, "pub"))
        .unwrap_or_else(|| role.contains("public"));
    let owner_next = next_field(&fields)
        .map(|next| parse_next_actions(next, Some(path.as_str())))
        .unwrap_or_default();
    next_actions.extend(owner_next.iter().cloned());
    owners.push(json!({
        "path": path,
        "role": role,
        "public": public,
        "nextActions": owner_next,
        "fields": fields,
    }));
    Some(path)
}

fn push_item(
    tokens: &[&str],
    current_owner: Option<&str>,
    items: &mut Vec<Value>,
    next_actions: &mut Vec<Value>,
) {
    let Some(name) = tokens.first() else {
        return;
    };
    let mut fields = parse_fields(tokens.iter().skip(1).copied());
    let owner_path = current_owner.unwrap_or("-");
    canonicalize_read_field(&mut fields, owner_path);
    next_actions.extend(
        next_field(&fields)
            .map(|next| parse_next_actions(next, Some(owner_path)))
            .unwrap_or_default(),
    );
    items.push(json!({
        "name": name,
        "kind": string_field(&fields, "kind").unwrap_or_else(|| "item".to_string()),
        "ownerPath": owner_path,
        "location": location(owner_path),
        "fields": fields,
    }));
}

fn push_hot(tokens: &[&str], current_owner: Option<&str>, next_actions: &mut Vec<Value>) {
    let Some(target) = tokens.first() else {
        return;
    };
    let mut fields = parse_fields(tokens.iter().skip(1).copied());
    let owner_path = current_owner.unwrap_or("-");
    canonicalize_read_field(&mut fields, owner_path);
    let mut action = json!({
        "kind": "hot",
        "target": target,
        "targetRole": "symbol",
        "ownerPath": owner_path,
    });
    if let Some(read) = string_field(&fields, "read") {
        action["read"] = json!(read);
    }
    action["fields"] = json!(fields);
    next_actions.push(action);
}

fn push_edge(tokens: &[&str], edges: &mut Vec<Value>) {
    if tokens.len() < 3 {
        return;
    }
    let (kind, label) = parse_edge_kind(tokens[1]);
    let mut edge = json!({
        "from": tokens[0],
        "kind": kind,
        "to": tokens[2],
    });
    if let Some(label) = label {
        edge["label"] = json!(label);
    }
    edges.push(edge);
}

fn push_finding(tokens: &[&str], findings: &mut Vec<Value>) {
    let Some(rule_id) = tokens.first() else {
        return;
    };
    let count = tokens
        .iter()
        .find_map(|token| token.strip_prefix('x'))
        .and_then(|count| count.parse::<usize>().ok())
        .unwrap_or(1);
    let fields = parse_fields(tokens.iter().skip(1).copied().filter(|token| {
        token
            .strip_prefix('x')
            .is_none_or(|count| count.parse::<usize>().is_err())
    }));
    findings.push(json!({
        "ruleId": rule_id,
        "severity": string_field(&fields, "severity").unwrap_or_else(|| "warning".to_string()),
        "count": count,
        "location": location_from_node(string_field(&fields, "at").as_deref().unwrap_or("memory")),
        "fields": fields,
    }));
}

fn push_hit(kind: &str, tokens: &[&str], hits: &mut Vec<Value>) {
    let Some(path) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let mut hit = json!({
        "kind": kind,
        "ownerPath": path,
        "location": location(path),
        "score": 1.0,
        "reason": kind,
        "fields": fields,
    });
    if let Some(symbol) = string_field(&fields, "name") {
        hit["symbol"] = json!(symbol);
    }
    hits.push(hit);
}

fn push_handle(tokens: &[&str], semantic_handles: &mut Vec<Value>) {
    let Some(id) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let kind = string_field(&fields, "kind").unwrap_or_else(|| "custom".to_string());
    let source = string_field(&fields, "source").unwrap_or_else(|| "custom".to_string());
    let title = string_field(&fields, "title").unwrap_or_else(|| (*id).to_string());
    let mut handle = json!({
    "id": id,
    "kind": kind,
    "source": source,
    "title": title,
    "fields": fields.clone(),
    });
    if let Some(owner_path) = string_field(&fields, "owner") {
        handle["ownerPath"] = json!(owner_path);
    }
    if let Some(query) = string_field(&fields, "query") {
        handle["queryTerms"] = json!([query]);
    }
    if let Some(owner_path) = string_field(&fields, "owner")
        && let Some(line) = fields.get("line").and_then(Value::as_u64)
    {
        handle["locations"] = json!([{
        "path": owner_path,
        "line": line,
        }]);
    }
    semantic_handles.push(handle);
}

fn push_native_syntax_fact(tokens: &[&str], native_syntax_facts: &mut Vec<Value>) {
    let Some(id) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let owner_path = string_field(&fields, "owner").unwrap_or_else(|| ".".to_string());
    let kind = string_field(&fields, "kind").unwrap_or_else(|| "custom".to_string());
    let source = string_field(&fields, "source").unwrap_or_else(|| "native-parser".to_string());
    let mut fact = json!({
    "id": id,
    "kind": kind,
    "source": source,
    "ownerPath": owner_path,
    "fields": fields.clone(),
    });
    if let Some(language_kind) = string_field(&fields, "languageKind") {
        fact["languageKind"] = json!(language_kind);
    }
    if let Some(name) = string_field(&fields, "name") {
        fact["name"] = json!(name);
    }
    if let Some(qualified_name) = string_field(&fields, "qualifiedName") {
        fact["qualifiedName"] = json!(qualified_name);
    }
    if let Some(visibility) = string_field(&fields, "visibility") {
        fact["visibility"] = json!(visibility);
    }
    if let Some(exported) = fields.get("exported").and_then(Value::as_bool) {
        fact["exported"] = json!(exported);
    }
    if let Some(query) = string_field(&fields, "query") {
        fact["queryKeys"] = json!([query]);
    }
    if let (Some(relation), Some(target)) = (
        string_field(&fields, "relation"),
        string_field(&fields, "relationTarget"),
    ) {
        fact["relations"] = json!([{
            "kind": relation,
            "target": target,
        }]);
    }
    if let Some(owner_path) = string_field(&fields, "owner")
        && let Some(line) = fields.get("line").and_then(Value::as_u64)
    {
        fact["location"] = json!({
        "path": owner_path,
        "lineRange": format!("{line}:{line}"),
        });
    }
    native_syntax_facts.push(fact);
}

fn push_external_type_hit(tokens: &[&str], hits: &mut Vec<Value>, type_surfaces: &mut Vec<Value>) {
    let Some(path_token) = tokens.first() else {
        return;
    };
    let (owner_path, line) = split_path_line(path_token);
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let location = location_with_line(&owner_path, line);
    let mut hit = json!({
        "kind": "external-type",
        "ownerPath": owner_path,
        "location": location.clone(),
        "score": 1.0,
        "reason": "external-type",
        "fields": fields.clone(),
    });
    if let Some(symbol) = string_field(&fields, "item") {
        hit["symbol"] = json!(symbol);
    }
    type_surfaces.push(external_type_surface(&owner_path, line, location, &fields));
    hits.push(hit);
}

fn external_type_surface(
    owner_path: &str,
    line: Option<u64>,
    location: Value,
    fields: &Map<String, Value>,
) -> Value {
    let dependency = string_field(fields, "dep").unwrap_or_else(|| "unknown".to_string());
    let surface = string_field(fields, "surface");
    let type_text = string_field(fields, "type").unwrap_or_else(|| "unknown".to_string());
    let name = string_field(fields, "item").unwrap_or_else(|| type_text.clone());
    let surface_label = surface.as_deref().unwrap_or("external-type");
    let mut surface_fields = fields.clone();
    surface_fields.insert("dependency".to_string(), json!(dependency));
    json!({
        "id": format!(
            "RS:{owner_path}:{}:{name}:{surface_label}",
            line.map(|line| line.to_string()).unwrap_or_else(|| "-".to_string())
        ),
        "name": name,
        "languageName": name,
        "qualifiedName": type_text,
        "kind": type_surface_kind(surface.as_deref()),
        "role": type_surface_role(surface.as_deref()),
        "ownerPath": owner_path,
        "location": location,
        "visibility": "public",
        "external": true,
        "source": string_field(fields, "source").unwrap_or_else(|| "native-parser".to_string()),
        "package": dependency,
        "module": dependency,
        "symbol": name,
        "versionScope": "external",
        "carrier": {
            "name": type_text,
            "languageName": type_text,
            "qualifiedName": type_text,
            "carrier": "external",
            "package": dependency,
            "module": dependency,
            "versionScope": "external",
            "external": true,
        },
        "fields": surface_fields,
    })
}

fn split_path_line(value: &str) -> (String, Option<u64>) {
    if let Some((path, line)) = value.rsplit_once(':')
        && !path.is_empty()
        && let Ok(line) = line.parse::<u64>()
    {
        return (path.to_string(), Some(line));
    }
    (value.to_string(), None)
}

fn location_with_line(path: &str, line: Option<u64>) -> Value {
    let mut location = json!({ "path": path });
    if let Some(line) = line {
        location["line"] = json!(line);
    }
    location
}

fn type_surface_kind(surface: Option<&str>) -> &'static str {
    match surface {
        Some("alias") => "alias",
        Some(surface) if surface.starts_with("field:") => "object",
        Some(surface) if surface.starts_with("tuple-field:") => "tuple",
        _ => "unknown",
    }
}

fn type_surface_role(surface: Option<&str>) -> &'static str {
    match surface {
        Some(surface) if surface.starts_with("param:") => "api-input",
        Some("return") => "api-output",
        Some(surface) if surface.starts_with("field:") => "api-field",
        Some(surface) if surface.starts_with("tuple-field:") => "api-field",
        Some("alias") => "public-type-alias",
        _ => "external-dependency",
    }
}

fn push_api_candidate(tokens: &[&str], hits: &mut Vec<Value>) {
    let Some(symbol) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    let owner_path = string_field(&fields, "owner").unwrap_or_else(|| "-".to_string());
    hits.push(json!({
        "kind": "api-candidate",
        "ownerPath": owner_path,
        "symbol": symbol,
        "location": location(string_field(&fields, "owner").as_deref().unwrap_or("-")),
        "score": 1.0,
        "reason": string_field(&fields, "reason").unwrap_or_else(|| "public-item".to_string()),
        "fields": fields,
    }));
}

fn push_test_node(tokens: &[&str], nodes: &mut Vec<Value>, next_actions: &mut Vec<Value>) {
    let Some(path) = tokens.first() else {
        return;
    };
    let fields = parse_fields(tokens.iter().skip(1).copied());
    next_actions.extend(
        next_field(&fields)
            .map(|next| parse_next_actions(next, Some(path)))
            .unwrap_or_default(),
    );
    nodes.push(json!({
        "id": format!("T:{path}"),
        "kind": "test",
        "path": path,
        "fields": fields,
    }));
}

fn push_note(tokens: &[&str], notes: &mut Vec<Value>) {
    let fields = parse_fields(tokens.iter().copied());
    notes.push(json!({
        "kind": string_field(&fields, "kind").unwrap_or_else(|| "note".to_string()),
        "message": string_field(&fields, "message").unwrap_or_else(|| tokens.join(" ")),
        "fields": fields,
    }));
}

fn push_fact_note(tag: &str, tokens: &[&str], notes: &mut Vec<Value>) {
    notes.push(json!({
        "kind": tag,
        "message": tokens.join(" "),
        "fields": parse_fields(tokens.iter().skip(1).copied()),
    }));
}

fn push_raw_note(tag: &str, tokens: &[&str], notes: &mut Vec<Value>) {
    notes.push(json!({
        "kind": "line",
        "message": format!("{tag} {}", tokens.join(" ")).trim().to_string(),
    }));
}

fn push_next_actions(fragment: String, owner_path: Option<&str>, next_actions: &mut Vec<Value>) {
    next_actions.extend(parse_next_actions(fragment, owner_path));
}

fn schema_view(options: &SemanticSearchJsonOptions) -> &str {
    options.view.as_str()
}

fn render_mode(options: &SemanticSearchJsonOptions) -> &str {
    if let Some(output_view) = options.output_view.as_deref() {
        return output_view;
    }
    match options.view.as_str() {
        "symbol"
        | "callsite"
        | "lexical"
        | "pattern"
        | "docs"
        | "docs-use"
        | "api"
        | "public-external-types"
        | "deps"
        | "cfg" => "hits",
        _ => "graph",
    }
}
