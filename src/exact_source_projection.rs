#[derive(Clone, Debug)]
pub(crate) struct ExactProjectionAuthority {
    pub(crate) generation_identity_digest: String,
    pub(crate) parser_identity_digest: String,
    pub(crate) query_pack_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedExactItem {
    pub(crate) canonical_selector: crate::content_identity::CanonicalItemSelector,
    pub(crate) owner_path: String,
    pub(crate) identity: crate::content_identity::CanonicalItemIdentity,
    pub(crate) code: String,
    pub(crate) source_byte_start: usize,
    pub(crate) source_byte_end: usize,
    pub(crate) owner_blob_digest: String,
    pub(crate) parser_artifact_digest: Option<String>,
}

pub(crate) fn callable_skeleton_projection(
    resolved: &ResolvedExactItem,
    authority: &ExactProjectionAuthority,
) -> Result<serde_json::Value, String> {
    let (signature, block) = match syn::parse_str::<syn::ItemFn>(&resolved.code) {
        Ok(item) => (item.sig, Some(*item.block)),
        Err(function_error) => match syn::parse_str::<syn::ImplItemFn>(&resolved.code) {
            Ok(item) => (item.sig, Some(item.block)),
            Err(method_error) => match syn::parse_str::<syn::TraitItemFn>(&resolved.code) {
                Ok(item) => (item.sig, item.default),
                Err(trait_method_error) => {
                    return Err(format!(
                        "parse callable skeleton root: function={function_error} method={method_error} traitMethod={trait_method_error}"
                    ));
                }
            },
        },
    };
    let root_selector = exact_selector_json(resolved, authority, Vec::new());
    let root_node_selector = root_selector
        .get("selector")
        .cloned()
        .ok_or_else(|| "callable root exact selector omitted selector".to_owned())?;
    let root_node_id = "callable:root";
    let mut collector = SkeletonCollector {
        resolved,
        authority: Some(authority),
        nodes: vec![serde_json::json!({
            "nodeId": root_node_id,
            "kind": "callable",
            "label": signature.ident.to_string(),
            "order": 0,
            "queryable": true,
            "selector": root_node_selector,
            "languageFacts": {
                "async": signature.asyncness.is_some(),
                "const": signature.constness.is_some(),
                "unsafe": matches!(&signature.safety, syn::Safety::Unsafe(_)),
                "inputCount": signature.inputs.len(),
                "genericParameterCount": signature.generics.params.len(),
            },
        })],
        relations: Vec::new(),
        next_order: 1,
    };
    if let Some(block) = block.as_ref() {
        syn::visit::Visit::visit_block(&mut collector, block);
    }
    let source_bytes = resolved.code.len() as u64;
    let callable_kind = match resolved.identity.kind.as_str() {
        "method" | "trait-function" => "method",
        kind => kind,
    };
    let mut payload = serde_json::json!({
        "rootNodeId": root_node_id,
        "callable": {
            "kind": callable_kind,
            "displayName": signature.ident.to_string(),
            "signature": signature.ident.to_string(),
        },
        "nodes": collector.nodes,
        "relations": collector.relations,
        "cost": {
            "sourceBytes": source_bytes,
            "projectedBytes": 0,
            "omittedBytes": 0,
        },
        "languageFacts": {
            "parser": "syn",
            "syntax": "rust",
        },
    });
    // `projectedBytes` owns the complete serialized packet, not only the
    // structural node fragment. Iterate to a stable value because the byte
    // count's own decimal width is part of the serialized representation.
    for _ in 0..3 {
        let projected_bytes = serde_json::to_vec(&payload)
            .map_err(|error| format!("measure callable skeleton projection: {error}"))?
            .len() as u64;
        payload["cost"]["projectedBytes"] = projected_bytes.into();
        payload["cost"]["omittedBytes"] = source_bytes.saturating_sub(projected_bytes).into();
    }
    Ok(payload)
}

// Parser-owned projection engine used by the Runtime-managed provider server.
struct SkeletonCollector<'a> {
    resolved: &'a ResolvedExactItem,
    authority: Option<&'a ExactProjectionAuthority>,
    nodes: Vec<serde_json::Value>,
    relations: Vec<serde_json::Value>,
    next_order: u64,
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
            let exact_selector = exact_selector_json(self.resolved, authority, vec![segment]);
            self.nodes.push(serde_json::json!({
                "nodeId": node_id,
                "kind": kind,
                "label": label,
                "order": order,
                "queryable": true,
            "selector": exact_selector["selector"].clone(),
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

fn exact_selector_json(
    resolved: &ResolvedExactItem,
    authority: &ExactProjectionAuthority,
    segments: Vec<serde_json::Value>,
) -> serde_json::Value {
    let root_item_scopes = resolved
        .canonical_selector
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
            "languageId": resolved.canonical_selector.language_id.as_str(),
            "kind": resolved.canonical_selector.kind.as_str(),
            "symbol": resolved.canonical_selector.symbol.as_str(),
            "scopes": root_item_scopes,
            "structuralSelector": resolved.canonical_selector.structural_selector,
        },
        "segments": segments,
    })
}
