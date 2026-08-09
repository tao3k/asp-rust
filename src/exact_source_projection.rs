#[derive(Clone, Debug)]
pub(crate) struct ExactProjectionAuthority {
    pub(crate) projection_kind: String,
    pub(crate) generation_identity_digest: String,
    pub(crate) parser_identity_digest: String,
    pub(crate) query_pack_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExactSelectorSegment {
    pub(crate) kind: String,
    pub(crate) ordinal: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedExactItem {
    pub(crate) canonical_selector: crate::canonical_item_identity::CanonicalItemSelectorV1,
    pub(crate) owner_path: String,
    pub(crate) identity: crate::canonical_item_identity::CanonicalItemIdentityV1,
    pub(crate) code: String,
    pub(crate) source_byte_start: usize,
    pub(crate) source_byte_end: usize,
    pub(crate) owner_blob_digest: String,
    pub(crate) parser_artifact_digest: Option<String>,
}

pub(super) fn provider_native_exact_projection_packet(
    provider_id: &str,
    requested_structural_selector: &str,
    resolved: &ResolvedExactItem,
    resolution_state: &str,
    authority: Option<&ExactProjectionAuthority>,
) -> Result<serde_json::Value, String> {
    let authority = authority.ok_or_else(|| {
        "provider-native exact projection requires typed v1 projection authority".to_string()
    })?;
    let normalized_parser_facts = serde_json::json!({
        "itemKind": resolved.canonical_selector.identity().kind,
        "itemName": resolved.canonical_selector.identity().symbol,
        "ownerPath": resolved.owner_path,
        "resolvedSelector": resolved.canonical_selector.structural_selector,
        "resolutionState": resolution_state,
    });
    let mut packet = serde_json::json!({
        "schemaId": "agent.semantic-protocols.provider-native-exact-projection",
        "schemaVersion": "1",
        "languageId": "rust",
        "providerId": provider_id,
        "ownerPath": resolved.owner_path,
        "requestedStructuralSelector": requested_structural_selector,
        "structuralSelector": resolved.canonical_selector.structural_selector,
        "projectionMode": authority.projection_kind,
        "normalizedParserFacts": normalized_parser_facts,
        "sourceContentDigest": resolved.owner_blob_digest,
        "sourceByteStart": resolved.source_byte_start,
        "sourceByteEnd": resolved.source_byte_end,
    });
    match authority.projection_kind.as_str() {
        "source" => {
            packet["projectionText"] = serde_json::Value::String(resolved.code.clone());
        }
        "callable-skeleton" => {
            packet["projectionPayload"] =
                callable_skeleton_projection(provider_id, resolved, authority)?;
        }
        projection_kind => {
            return Err(format!(
                "unsupported provider-native exact projection kind `{projection_kind}`"
            ));
        }
    }
    Ok(packet)
}

pub(crate) fn callable_skeleton_projection(
    provider_id: &str,
    resolved: &ResolvedExactItem,
    authority: &ExactProjectionAuthority,
) -> Result<serde_json::Value, String> {
    let item: syn::ItemFn = syn::parse_str(&resolved.code)
        .map_err(|error| format!("parse callable skeleton root: {error}"))?;
    let root_selector = exact_selector_json(resolved, authority, Vec::new());
    let root_node_id = "callable:root";
    let mut collector = SkeletonCollector {
        resolved,
        authority: Some(authority),
        nodes: vec![serde_json::json!({
            "nodeId": root_node_id,
            "kind": "callable",
            "label": item.sig.ident.to_string(),
            "order": 0,
            "queryable": true,
            "exactSelector": root_selector,
            "languageFacts": {
                "async": item.sig.asyncness.is_some(),
                "const": item.sig.constness.is_some(),
                "unsafe": item.sig.unsafety.is_some(),
                "inputCount": item.sig.inputs.len(),
                "genericParameterCount": item.sig.generics.params.len(),
            },
        })],
        relations: Vec::new(),
        segments: Vec::new(),
        next_order: 1,
    };
    syn::visit::Visit::visit_block(&mut collector, &item.block);
    let source_bytes = resolved.code.len() as u64;
    let structural_bytes = serde_json::to_vec(&serde_json::json!({
        "nodes": collector.nodes,
        "relations": collector.relations,
    }))
    .map_err(|error| format!("measure callable skeleton projection: {error}"))?
    .len() as u64;
    let projected_bytes = structural_bytes.min(source_bytes);
    Ok(serde_json::json!({
        "schemaId": "agent.semantic-protocols.callable-skeleton-projection",
        "schemaVersion": "1",
        "projectionKind": "callable-skeleton",
        "languageId": "rust",
        "providerId": provider_id,
        "rootSelector": exact_selector_json(resolved, authority, Vec::new()),
        "rootNodeId": root_node_id,
        "callable": {
            "kind": "function",
            "displayName": item.sig.ident.to_string(),
            "signature": item.sig.ident.to_string(),
        },
        "nodes": collector.nodes,
        "relations": collector.relations,
        "cost": {
            "sourceBytes": source_bytes,
            "projectedBytes": projected_bytes,
            "omittedBytes": source_bytes - projected_bytes,
        },
        "languageFacts": {
            "parser": "syn",
            "syntax": "rust",
        },
    }))
}

// Parser-owned projection engine shared by CLI and non-CLI provider callers.
struct SkeletonCollector<'a> {
    resolved: &'a ResolvedExactItem,
    authority: Option<&'a ExactProjectionAuthority>,
    nodes: Vec<serde_json::Value>,
    relations: Vec<serde_json::Value>,
    segments: Vec<ParserSkeletonSegment>,
    next_order: u64,
}

#[derive(Clone, Debug)]
struct ParserSkeletonSegment {
    kind: String,
    ordinal: u64,
    source_byte_start: usize,
    source_byte_end: usize,
}

impl SkeletonCollector<'_> {
    fn push(&mut self, kind: &str, label: &str, span: proc_macro2::Span) {
        let order = self.next_order;
        self.next_order += 1;
        let node_id = format!("{kind}:{order}");
        let source_byte_start =
            line_column_offset(&self.resolved.code, span.start()).unwrap_or_default();
        let source_byte_end =
            line_column_offset(&self.resolved.code, span.end()).unwrap_or(source_byte_start);
        let segment = serde_json::json!({
            "relation": "contains",
            "kind": kind,
            "identity": format!("ordinal-{order}"),
            "label": label,
        });
        if let Some(authority) = self.authority {
            self.nodes.push(serde_json::json!({
                "nodeId": node_id,
                "kind": kind,
                "label": label,
                "order": order,
                "queryable": true,
                "exactSelector": exact_selector_json(
                    self.resolved,
                    authority,
                    vec![segment],
                ),
                "sourceLocatorHint": {
                    "sourceByteStart": self.resolved.source_byte_start + source_byte_start,
                    "sourceByteEnd": self.resolved.source_byte_start + source_byte_end,
                },
            }));
            self.relations.push(serde_json::json!({
                "fromNodeId": "callable:root",
                "toNodeId": node_id,
                "kind": "contains",
            }));
        }
        self.segments.push(ParserSkeletonSegment {
            kind: kind.to_string(),
            ordinal: order,
            source_byte_start,
            source_byte_end,
        });
    }
}

impl<'ast> syn::visit::Visit<'ast> for SkeletonCollector<'_> {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        self.push("binding", "let", syn::spanned::Spanned::span(node));
        syn::visit::visit_local(self, node);
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        let span = syn::spanned::Spanned::span(node);
        match node {
            syn::Expr::If(_) => self.push("branch", "if", span),
            syn::Expr::Match(_) => self.push("branch", "match", span),
            syn::Expr::Loop(_) => self.push("loop", "loop", span),
            syn::Expr::While(_) => self.push("loop", "while", span),
            syn::Expr::ForLoop(_) => self.push("loop", "for", span),
            syn::Expr::Return(_) => self.push("exit", "return", span),
            _ => {}
        }
        syn::visit::visit_expr(self, node);
    }
}

fn line_column_offset(source: &str, location: proc_macro2::LineColumn) -> Option<usize> {
    if location.line == 0 {
        return None;
    }
    let line_index = location.line - 1;
    if let Some(line) = source.split_inclusive('\n').nth(line_index) {
        let offset = source
            .split_inclusive('\n')
            .take(line_index)
            .map(str::len)
            .sum::<usize>();
        return (location.column <= line.len()).then_some(offset + location.column);
    }
    (location.line == source.lines().count() + 1 && location.column == 0).then_some(source.len())
}

pub(super) fn resolve_callable_segment(
    resolved: &ResolvedExactItem,
    requested: &ExactSelectorSegment,
) -> Result<ResolvedExactItem, String> {
    let item: syn::ItemFn = syn::parse_str(&resolved.code)
        .map_err(|error| format!("parse exact callable segment root: {error}"))?;
    let mut collector = SkeletonCollector {
        resolved,
        authority: None,
        nodes: Vec::new(),
        relations: Vec::new(),
        segments: Vec::new(),
        next_order: 1,
    };
    syn::visit::Visit::visit_block(&mut collector, &item.block);
    let segment = collector
        .segments
        .into_iter()
        .find(|segment| {
            segment.kind == requested.kind && segment.ordinal == requested.ordinal
        })
        .ok_or_else(|| {
            format!(
                "exact source query state=item-missing reasonKind=callable-segment-not-found kind={} ordinal={}",
                requested.kind, requested.ordinal
            )
        })?;
    let code = resolved
        .code
        .get(segment.source_byte_start..segment.source_byte_end)
        .ok_or_else(|| {
            "exact source query state=parser-failed reasonKind=callable-segment-span-invalid"
                .to_string()
        })?
        .to_string();
    let mut canonical_selector = resolved.canonical_selector.clone();
    canonical_selector.structural_selector = format!(
        "{}/segment/{}/ordinal-{}",
        canonical_selector.structural_selector, segment.kind, segment.ordinal
    );
    Ok(ResolvedExactItem {
        canonical_selector,
        owner_path: resolved.owner_path.clone(),
        identity: resolved.identity.clone(),
        code,
        source_byte_start: resolved.source_byte_start + segment.source_byte_start,
        source_byte_end: resolved.source_byte_start + segment.source_byte_end,
        owner_blob_digest: resolved.owner_blob_digest.clone(),
        parser_artifact_digest: resolved.parser_artifact_digest.clone(),
    })
}

fn exact_selector_json(
    resolved: &ResolvedExactItem,
    authority: &ExactProjectionAuthority,
    segments: Vec<serde_json::Value>,
) -> serde_json::Value {
    let root_item_scopes = resolved
        .canonical_selector
        .identity()
        .scopes
        .iter()
        .map(|scope| {
            serde_json::json!({
                "relation": scope.relation.as_str(),
                "kind": scope.kind.as_str(),
                "symbol": scope.symbol.as_str(),
            })
        })
        .collect::<Vec<_>>();
    let selector = if segments.is_empty() {
        resolved.canonical_selector.structural_selector.to_string()
    } else {
        let segment = &segments[0];
        format!(
            "{}/segment/{}/{}",
            resolved.canonical_selector.structural_selector,
            segment["kind"].as_str().unwrap_or("node"),
            segment["identity"].as_str().unwrap_or("ordinal")
        )
    };
    serde_json::json!({
        "schemaId": "asp.exact-structural-selector.v1",
        "schemaVersion": "1",
        "languageId": "rust",
        "ownerPath": resolved.owner_path,
        "selector": selector,
        "generationIdentityDigest": authority.generation_identity_digest,
        "parserIdentityDigest": authority.parser_identity_digest,
        "queryPackDigest": authority.query_pack_digest,
        "rootItemSelector": {
            "schemaId": "asp.canonical-item-selector.v1",
            "schemaVersion": "1",
            "languageId": resolved.canonical_selector.identity().language_id.as_str(),
            "kind": resolved.canonical_selector.identity().kind.as_str(),
            "symbol": resolved.canonical_selector.identity().symbol.as_str(),
            "scopes": root_item_scopes,
            "structuralSelector": resolved.canonical_selector.structural_selector,
        },
        "segments": segments,
    })
}
