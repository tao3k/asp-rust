use std::collections::BTreeMap;

use super::graph_dependency::render_dependency_graph;

pub(in crate::cli) fn render_search_graph_packet(
    rendered: &str,
    seed_limit: Option<usize>,
) -> Result<String, String> {
    let header = rendered
        .lines()
        .find(|line| line.starts_with("[search-"))
        .unwrap_or("[search] q=-");
    let limit = seed_limit.unwrap_or(8).max(1);
    let fields = header_fields(header);
    if header.starts_with("[search-prime]") {
        return Ok(render_prime_graph(rendered, header, &fields, limit));
    }
    if header.starts_with("[search-owner]") {
        return Ok(render_owner_graph(rendered, header, &fields, limit));
    }
    if header.starts_with("[search-query]") {
        return Ok(render_query_graph(rendered, header, limit));
    }
    if header.starts_with("[search-policy]") {
        return Ok(render_policy_graph(rendered, header, &fields, limit));
    }
    if header.starts_with("[search-reasoning]") {
        return Ok(render_reasoning_graph(rendered, header, &fields, limit));
    }
    if header.starts_with("[search-ingest]") {
        return Ok(render_ingest_graph(rendered, header, limit));
    }
    if header.starts_with("[search-workspace]") {
        return Ok(render_workspace_graph(rendered, header, limit));
    }
    if header.starts_with("[search-dependency]") || header.starts_with("[search-deps]") {
        return Ok(render_dependency_graph(rendered, &fields, limit));
    }
    Ok(render_generic_graph(rendered, header, limit))
}

fn render_prime_graph(
    rendered: &str,
    header: &str,
    fields: &BTreeMap<String, String>,
    limit: usize,
) -> String {
    let root = fields
        .get("root")
        .or_else(|| fields.get("package"))
        .or_else(|| fields.get("pkg"))
        .map(String::as_str)
        .unwrap_or(".");
    let mut out = format!(
        "[search-prime] root={root} alg=budgeted-prime-frontier-v1 budget=handles:{limit}\n"
    );
    if let Some(decision) = rendered.lines().find(|line| line.starts_with("|decision ")) {
        out.push_str(decision);
        out.push('\n');
    }
    out.push_str(
        "legend: ID=kind:role(value)!next; entries profile(selectors=>returns); frontier ID.next\n",
    );
    let mut features = seed_values(rendered, "features");
    if features.is_empty() {
        features = line_second_tokens(rendered, "|feature ");
    }
    if !features.is_empty() {
        let mut nodes = feature_nodes(features, limit);
        let remaining = limit.saturating_sub(nodes.len());
        nodes.extend(role_nodes(
            "C",
            "cfg",
            "cfg",
            seed_values(rendered, "cfg"),
            "cfg",
            remaining,
        ));
        append_graph_block(
            &mut out,
            "graph:{G=search,F=feature,C=cfg,O=owner}",
            &nodes,
            "gates",
        );
    } else {
        let nodes = role_nodes(
            "O",
            "owner",
            "path",
            package_relative_values(owner_values(rendered), root),
            "owner",
            limit,
        );
        append_node_block(&mut out, "owner:{O=owner}", &nodes);
        out.push_str("entries=owner-tests(O=>covering-tests+test-entrypoints+fixtures)\n");
    }
    out.push_str("omit=items,blocks,code,full-test-list\n");
    out.push_str("avoid=raw-read,full-json,broad-lexical\n");
    if !header.ends_with('\n') && out.is_empty() {
        out.push('\n');
    }
    out
}

fn render_owner_graph(
    rendered: &str,
    _header: &str,
    fields: &BTreeMap<String, String>,
    limit: usize,
) -> String {
    let query = header_query(_header)
        .or_else(|| fields.get("q").map(String::as_str))
        .unwrap_or("-");
    let items = item_seeds(rendered);
    if !items.is_empty() {
        if has_item_query(rendered, fields) {
            let matching_items = matching_item_seeds(&items, fields, query);
            return render_owner_item_query_graph(rendered, fields, query, &matching_items, limit);
        }
        return render_owner_items_graph(rendered, fields, query, &items, limit);
    }
    let algorithm = synthesis_field(rendered, "algorithm").unwrap_or("bounded-reachability-depth1");
    let mut out = format!("[search-owner] q={query} alg={algorithm}\n");
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    let owner = first_owner(rendered).unwrap_or_else(|| query.to_string());
    let mut nodes = vec![GraphNode::new("O", "owner", "path", owner, "owner")];
    let remaining = limit.saturating_sub(1);
    nodes.extend(role_nodes(
        "T",
        "test",
        "path",
        test_values(rendered),
        "tests",
        remaining,
    ));
    append_graph_block(
        &mut out,
        "graph:{G=search,O=owner,T=test}",
        &nodes,
        "covers",
    );
    out.push_str("entries=owner-tests(O=>covering-tests+test-entrypoints+fixtures)\n");
    out
}

fn render_generic_graph(rendered: &str, header: &str, limit: usize) -> String {
    let mut out = String::new();
    out.push_str(header);
    out.push('\n');
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    let nodes = role_nodes("O", "owner", "path", owner_values(rendered), "owner", limit);
    append_graph_block(&mut out, "graph:{G=search,O=owner}", &nodes, "selects");
    append_passthrough_lines(&mut out, rendered, &["entries=", "|next ", "avoid="]);
    out
}

fn render_query_graph(rendered: &str, header: &str, limit: usize) -> String {
    let query = header_query(header).unwrap_or("-");
    let mut out = format!("[search-query] q={query} alg=native-syntax-query\n");
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    let nodes = role_nodes("O", "owner", "path", owner_values(rendered), "owner", limit);
    append_graph_block(&mut out, "graph:{G=search,O=owner}", &nodes, "selects");
    out
}

fn render_workspace_graph(rendered: &str, header: &str, limit: usize) -> String {
    let mut out = String::new();
    let fields = header_fields(header);
    let root = fields.get("root").map(String::as_str).unwrap_or(".");
    out.push_str(&format!(
        "[search-workspace] root={root} alg=seed-frontier\n"
    ));
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    let nodes = role_nodes(
        "P",
        "package",
        "pkg",
        line_second_tokens(rendered, "|package "),
        "owner",
        limit,
    );
    append_graph_block(&mut out, "graph:{G=search,P=package}", &nodes, "contains");
    out
}

fn render_ingest_graph(rendered: &str, header: &str, limit: usize) -> String {
    let fields = header_fields(header);
    let mut out = "[search-ingest] root=. alg=seed-frontier".to_string();
    if let Some(scope) = fields.get("scope") {
        out.push_str(&format!(" scope={scope}"));
    }
    out.push('\n');
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    out.push_str("aliases: graph:{G=search,O=owner,T=test,S=symbol}\n");
    let mut nodes = role_nodes("O", "owner", "path", owner_values(rendered), "owner", limit);
    nodes.extend(role_nodes(
        "T",
        "test",
        "path",
        test_values(rendered),
        "tests",
        limit.saturating_sub(nodes.len()),
    ));
    nodes.extend(role_nodes(
        "S",
        "symbol",
        "symbol",
        seed_values(rendered, "symbol"),
        "symbol",
        limit.saturating_sub(nodes.len()),
    ));
    append_edges_rank_frontier(&mut out, &nodes, "selects", None);
    out
}

fn render_reasoning_graph(
    rendered: &str,
    header: &str,
    fields: &BTreeMap<String, String>,
    limit: usize,
) -> String {
    if fields.get("q").is_some_and(|query| query == "feature-cfg") {
        let feature = fields
            .get("query")
            .map(String::as_str)
            .or_else(|| header_query(header))
            .unwrap_or("-");
        let mut out = format!(
            "[search-reasoning] q=feature-cfg selector=feature={feature} alg=feature-cfg-frontier\n"
        );
        out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
        out.push_str("aliases: graph:{G=search,F=feature,O=owner}\n");
        let mut nodes = vec![GraphNode::new("F", "feature", "feature", feature, "cfg")];
        nodes.extend(role_nodes(
            "O",
            "owner",
            "path",
            owner_values(rendered),
            "owner",
            limit.saturating_sub(1),
        ));
        append_edges_rank_frontier(&mut out, &nodes, "selects", None);
        append_passthrough_lines(&mut out, rendered, &["entries=", "|next ", "avoid="]);
        return out;
    }
    if fields
        .get("q")
        .is_some_and(|query| query == "finding-frontier")
    {
        let finding = fields
            .get("query")
            .map(String::as_str)
            .or_else(|| header_query(header))
            .unwrap_or("-");
        let mut out = format!(
            "[search-reasoning] q=finding-frontier selector=finding={finding} alg=finding-frontier\n"
        );
        out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
        out.push_str("aliases: graph:{G=search,F=finding,O=owner}\n");
        let mut nodes = vec![GraphNode::new(
            "F", "finding", "finding", finding, "finding",
        )];
        nodes.extend(role_nodes(
            "O",
            "owner",
            "path",
            owner_values(rendered),
            "owner",
            limit.saturating_sub(1),
        ));
        append_edges_rank_frontier(&mut out, &nodes, "selects", None);
        append_passthrough_lines(&mut out, rendered, &["entries=", "|next ", "avoid="]);
        return out;
    }
    render_generic_graph(rendered, header, limit)
}

fn render_policy_graph(
    rendered: &str,
    _header: &str,
    fields: &BTreeMap<String, String>,
    limit: usize,
) -> String {
    let query = fields.get("q").map(String::as_str).unwrap_or("-");
    let mut out = format!("[search-policy] q={query} alg=policy-handle-catalog\n");
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    out.push_str("aliases: graph:{G=search,O=owner,T=test}\n");
    let mut nodes = role_nodes("O", "owner", "path", owner_values(rendered), "owner", 1);
    nodes.extend(role_nodes(
        "T",
        "test",
        "path",
        test_values(rendered),
        "tests",
        limit.saturating_sub(nodes.len()),
    ));
    append_edges_rank_frontier(&mut out, &nodes, "selects", None);
    out
}

fn render_owner_item_query_graph(
    rendered: &str,
    fields: &BTreeMap<String, String>,
    query: &str,
    items: &[&ItemSeed],
    limit: usize,
) -> String {
    let package = fields.get("pkg").map(String::as_str).unwrap_or(".");
    let item_query = fields.get("itemQuery").map(String::as_str).unwrap_or(query);
    let owner = if package == "." {
        first_owner(rendered).unwrap_or_else(|| query.to_string())
    } else {
        query.to_string()
    };
    let items = items
        .iter()
        .copied()
        .take(limit.saturating_sub(2).max(1))
        .collect::<Vec<_>>();
    let mut out =
        format!("[search-owner] q={query} pkg={package} selector=items alg=item-frontier\n");
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    out.push_str("aliases: graph:{G=search,O=owner,Q=query,I=item}\n");
    let item_nodes = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{}=item:symbol({})@{}!syntax",
                numbered_id("I", index),
                item.name,
                item_locator(item, package)
            )
        })
        .collect::<Vec<_>>();
    out.push_str(&format!(
        "O=owner:path({owner})!owner;Q=query:term({item_query})!query"
    ));
    if !item_nodes.is_empty() {
        out.push(';');
        out.push_str(&item_nodes.join(";"));
    }
    out.push('\n');
    for (index, item) in items.iter().enumerate() {
        let item_id = numbered_id("I", index);
        let locator = item_locator(item, package);
        out.push_str(&format!(
            "syntax {item_id} selector={locator} pattern='{}'\n",
            item_pattern(&item.syn, &item.name),
        ));
    }
    let item_ids = (0..items.len())
        .map(|index| numbered_id("I", index))
        .collect::<Vec<_>>();
    out.push_str("G>{O:selects,Q:matches}\n");
    out.push_str(&format!(
        "O>{{{}}}\n",
        item_ids
            .iter()
            .map(|id| format!("{id}:contains"))
            .collect::<Vec<_>>()
            .join(",")
    ));
    out.push_str(&format!(
        "Q>{{{}}}\n",
        item_ids
            .iter()
            .map(|id| format!("{id}:matches"))
            .collect::<Vec<_>>()
            .join(",")
    ));
    out.push_str("rank=");
    out.push_str(&item_ids.join(","));
    if !item_ids.is_empty() {
        out.push(',');
    }
    out.push_str("O frontier=");
    out.push_str(
        &item_ids
            .iter()
            .map(|id| format!("{id}.syntax"))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
    out.push_str("omit=code,projection-nodes,large-item-text\n");
    out.push_str("avoid=inline-code-in-search,raw-read,repeat-owner\n");
    out
}

fn render_owner_items_graph(
    rendered: &str,
    fields: &BTreeMap<String, String>,
    query: &str,
    items: &[ItemSeed],
    limit: usize,
) -> String {
    let package = fields.get("pkg").map(String::as_str).unwrap_or(".");
    let owner = if package == "." {
        first_owner(rendered).unwrap_or_else(|| query.to_string())
    } else {
        query.to_string()
    };
    let mut out =
        format!("[search-owner] q={query} pkg={package} selector=items alg=item-frontier\n");
    out.push_str("legend: ID=kind:role(value)!next; edge SRC>{DST:rel}; frontier ID.next\n");
    out.push_str("aliases: graph:{G=search,O=owner,I=item}\n");
    let item_nodes = items
        .iter()
        .take(limit.saturating_sub(1).max(1))
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{}=item:symbol({})@{}!syntax",
                numbered_id("I", index),
                item.name,
                item_locator(item, package)
            )
        })
        .collect::<Vec<_>>();
    out.push_str(&format!("O=owner:path({owner})!owner"));
    if !item_nodes.is_empty() {
        out.push(';');
        out.push_str(&item_nodes.join(";"));
    }
    out.push('\n');
    for (index, item) in items
        .iter()
        .take(limit.saturating_sub(1).max(1))
        .enumerate()
    {
        let item_id = numbered_id("I", index);
        let read = item_locator(item, package);
        out.push_str(&format!(
            "syntax {item_id} selector={read} pattern='{}'\n",
            item_pattern(&item.syn, &item.name),
        ));
    }
    let item_ids = (0..items.len().min(limit.saturating_sub(1).max(1)))
        .map(|index| numbered_id("I", index))
        .collect::<Vec<_>>();
    out.push_str("G>{O:selects");
    for item_id in &item_ids {
        out.push_str(&format!(",{item_id}:matches"));
    }
    out.push_str("}\n");
    out.push_str("O>{");
    out.push_str(
        &item_ids
            .iter()
            .map(|id| format!("{id}:contains"))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("}\n");
    out.push_str("rank=");
    out.push_str(&item_ids.join(","));
    if !item_ids.is_empty() {
        out.push_str(",O");
    } else {
        out.push('O');
    }
    out.push_str(" frontier=");
    out.push_str(
        &item_ids
            .iter()
            .map(|id| format!("{id}.syntax"))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
    out.push_str("omit=code,projection-nodes,large-item-text\n");
    out.push_str("avoid=inline-code-in-search,raw-read,repeat-owner\n");
    out
}

fn workspace_prefixed_selector(selector: &str, package: &str) -> String {
    if package == "." || selector.starts_with('/') || selector.contains("://") {
        return selector.to_string();
    }
    let prefix = format!("{package}/");
    if selector.starts_with(&prefix) {
        selector.to_string()
    } else {
        format!("{package}/{selector}")
    }
}

fn append_graph_block(out: &mut String, aliases: &str, nodes: &[GraphNode], relation: &str) {
    out.push_str("aliases: ");
    out.push_str(aliases);
    out.push('\n');
    append_edges_rank_frontier(out, nodes, relation, None);
}

fn append_node_block(out: &mut String, aliases: &str, nodes: &[GraphNode]) {
    out.push_str("aliases: ");
    out.push_str(aliases);
    out.push('\n');
    if nodes.is_empty() {
        return;
    }
    out.push_str(
        &nodes
            .iter()
            .map(GraphNode::render)
            .collect::<Vec<_>>()
            .join(";"),
    );
    out.push('\n');
}

fn append_edges_rank_frontier(
    out: &mut String,
    nodes: &[GraphNode],
    relation: &str,
    first_relation: Option<(&str, &str)>,
) {
    if nodes.is_empty() {
        out.push_str("G>{}\nrank= frontier=\n");
        return;
    }
    out.push_str(
        &nodes
            .iter()
            .map(GraphNode::render)
            .collect::<Vec<_>>()
            .join(";"),
    );
    out.push('\n');
    let edges = nodes
        .iter()
        .map(|node| {
            let relation = first_relation
                .filter(|(id, _)| *id == node.id)
                .map(|(_, relation)| relation)
                .unwrap_or(relation);
            format!("{}:{relation}", node.id)
        })
        .collect::<Vec<_>>();
    out.push_str("G>{");
    out.push_str(&edges.join(","));
    out.push_str("}\n");
    let ranks = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    out.push_str("rank=");
    out.push_str(&ranks);
    out.push_str(" frontier=");
    out.push_str(
        &nodes
            .iter()
            .map(|node| format!("{}.{}", node.id, node.next))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
}

fn feature_nodes(values: Vec<String>, limit: usize) -> Vec<GraphNode> {
    role_nodes("F", "feature", "feature", values, "features", limit)
}

fn role_nodes(
    prefix: &str,
    kind: &str,
    value_kind: &str,
    values: Vec<String>,
    next: &str,
    limit: usize,
) -> Vec<GraphNode> {
    values
        .into_iter()
        .filter(|value| !value.is_empty() && value != "-")
        .take(limit)
        .enumerate()
        .map(|(index, value)| {
            GraphNode::new(
                numbered_id(prefix, index),
                kind.to_string(),
                value_kind.to_string(),
                value,
                next.to_string(),
            )
        })
        .collect()
}

fn numbered_id(prefix: &str, index: usize) -> String {
    if index == 0 {
        prefix.to_string()
    } else {
        format!("{prefix}{}", index + 1)
    }
}

fn header_fields(header: &str) -> BTreeMap<String, String> {
    header
        .split_once(']')
        .map(|(_, rest)| rest)
        .unwrap_or(header)
        .split_whitespace()
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            Some((key.to_string(), value.trim_matches('"').to_string()))
        })
        .collect()
}

pub(super) fn seed_values(rendered: &str, seed_key: &str) -> Vec<String> {
    let prefix = format!("|seed {seed_key}:");
    rendered
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .flat_map(split_csv)
        .collect()
}

pub(super) fn owner_values(rendered: &str) -> Vec<String> {
    let mut values = seed_values(rendered, "owner");
    if values.is_empty() {
        values = line_second_tokens(rendered, "|owner ");
    }
    if values.is_empty() {
        values = next_values(rendered, "owner:");
    }
    if values.is_empty()
        && let Some(seeds) = synthesis_field(rendered, "seeds")
    {
        values = seeds
            .split(',')
            .filter_map(|seed| seed.strip_prefix("owner:"))
            .map(str::to_string)
            .collect();
    }
    values
}

pub(super) fn test_values(rendered: &str) -> Vec<String> {
    let mut values = seed_values(rendered, "tests");
    if values.is_empty() {
        values = line_second_tokens(rendered, "|test ");
    }
    if values.is_empty() {
        values = next_values(rendered, "tests:");
    }
    if values.is_empty()
        && let Some(seeds) = synthesis_field(rendered, "seeds")
    {
        values = seeds
            .split(',')
            .filter_map(|seed| seed.strip_prefix("tests:"))
            .map(str::to_string)
            .collect();
    }
    values
}

fn first_owner(rendered: &str) -> Option<String> {
    owner_values(rendered).into_iter().next()
}

pub(super) fn line_second_tokens(rendered: &str, prefix: &str) -> Vec<String> {
    rendered
        .lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .filter_map(|rest| rest.split_whitespace().next())
        .flat_map(split_csv)
        .collect()
}

fn next_values(rendered: &str, value_prefix: &str) -> Vec<String> {
    rendered
        .lines()
        .filter_map(|line| line.strip_prefix("|next "))
        .flat_map(|rest| rest.split(','))
        .filter_map(|part| part.trim().strip_prefix(value_prefix))
        .map(str::to_string)
        .collect()
}

fn package_relative_values(values: Vec<String>, package_root: &str) -> Vec<String> {
    if package_root == "." {
        return values;
    }
    let prefix = format!("{package_root}/");
    let mut values = values
        .into_iter()
        .map(|value| {
            value
                .strip_prefix(&prefix)
                .map(str::to_string)
                .unwrap_or(value)
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|value| (value.ends_with("src/lib.rs"), value.clone()));
    values
}

fn header_query(header: &str) -> Option<&str> {
    let rest = header.split_once(" q=")?.1;
    let end = [
        " querySet=",
        " selector=",
        " mode=",
        " backend=",
        " pkg=",
        " skipped=",
        " scope=",
        " alg=",
    ]
    .iter()
    .filter_map(|marker| rest.find(marker))
    .min()
    .unwrap_or(rest.len());
    Some(rest[..end].trim())
}

pub(super) fn append_passthrough_lines(out: &mut String, rendered: &str, prefixes: &[&str]) {
    for line in rendered.lines() {
        if has_passthrough_prefix(line, prefixes) {
            out.push_str(line);
            out.push('\n');
        }
    }
}

fn has_passthrough_prefix(line: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| line.starts_with(prefix))
}

pub(super) fn synthesis_field<'a>(rendered: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    rendered
        .lines()
        .filter_map(|line| line.strip_prefix("|synthesis "))
        .flat_map(str::split_whitespace)
        .find_map(|part| part.strip_prefix(&prefix))
}

struct ItemSeed {
    name: String,
    read: String,
    structural_selector: Option<String>,
    syn: String,
}

fn item_seeds(rendered: &str) -> Vec<ItemSeed> {
    rendered
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("|item ")?;
            let mut parts = rest.split_whitespace();
            let name = parts.next()?.to_string();
            let mut read = None;
            let mut structural_selector = None;
            let mut syn = None;
            for part in parts {
                if let Some(value) = part.strip_prefix("read=") {
                    read = Some(value.to_string());
                } else if let Some(value) = part.strip_prefix("structuralSelector=") {
                    structural_selector = Some(value.to_string());
                } else if let Some(value) = part.strip_prefix("syn=") {
                    syn = Some(value.to_string());
                }
            }
            Some(ItemSeed {
                name,
                read: read?,
                structural_selector,
                syn: syn.unwrap_or_else(|| "item".to_string()),
            })
        })
        .collect()
}

fn matching_item_seeds<'a>(
    items: &'a [ItemSeed],
    fields: &BTreeMap<String, String>,
    query: &str,
) -> Vec<&'a ItemSeed> {
    let terms = fields
        .get("itemQuery")
        .map(String::as_str)
        .unwrap_or(query)
        .split('|')
        .map(str::trim)
        .filter(|term| !term.is_empty() && *term != "-")
        .collect::<Vec<_>>();
    let matching = items
        .iter()
        .filter(|item| terms.iter().any(|term| item.name == *term))
        .collect::<Vec<_>>();
    if matching.is_empty() {
        items.first().into_iter().collect()
    } else {
        matching
    }
}

fn item_locator(item: &ItemSeed, package: &str) -> String {
    let selector = item.structural_selector.as_deref().unwrap_or(&item.read);
    workspace_prefixed_selector(selector, package)
}

fn has_item_query(rendered: &str, fields: &BTreeMap<String, String>) -> bool {
    fields.contains_key("itemQuery") || rendered.lines().any(|line| line.starts_with("|query "))
}

fn item_pattern(syntax_kind: &str, item_name: &str) -> String {
    let syntax_kind = syntax_kind.split('/').next().unwrap_or(syntax_kind);
    let capture = if syntax_kind.contains("struct")
        || syntax_kind.contains("enum")
        || syntax_kind.contains("type")
    {
        "type.name"
    } else {
        "function.name"
    };
    format!(
        "(({syntax_kind} name: (_) @{capture}) (#eq? @{capture} \"{}\"))",
        item_name.replace('"', "\\\""),
    )
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .map(str::to_string)
        .collect()
}

struct GraphNode {
    id: String,
    kind: String,
    value_kind: String,
    value: String,
    next: String,
}

impl GraphNode {
    fn new(
        id: impl Into<String>,
        kind: impl Into<String>,
        value_kind: impl Into<String>,
        value: impl Into<String>,
        next: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            value_kind: value_kind.into(),
            value: value.into(),
            next: next.into(),
        }
    }

    fn render(&self) -> String {
        format!(
            "{}={}:{}({})!{}",
            self.id, self.kind, self.value_kind, self.value, self.next
        )
    }
}
