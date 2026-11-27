//! Native Rust `syn` AST inspection (`ast-inspect`) and batched compiler
//! oracle verification (`ast-verify`) for ASP's `agent-semantic-ast` protocol.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use syn::spanned::Spanned as _;
use syn::visit::{self, Visit};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireAstInspectRequest {
    path: String,
    source_utf8: String,
    #[serde(default)]
    _source_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireByteSpan {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireAstNodeAttr {
    key: String,
    value: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireAstNodeSpanRecord {
    locator: String,
    kind: &'static str,
    language_kind: &'static str,
    name: String,
    line: usize,
    outer_span: WireByteSpan,
    inner_span: WireByteSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    body_span: Option<WireByteSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<String>,
    attrs: Vec<WireAstNodeAttr>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WireAstSyntaxDiagnostic {
    pub(super) byte_start: usize,
    pub(super) byte_end: usize,
    pub(super) message: String,
    pub(super) expected_tokens: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireSyntaxSurface {
    line_comment: &'static str,
    block_comment: [&'static str; 2],
    delimiter_pairs: [[&'static str; 2]; 3],
    statement_terminator: &'static str,
    string_delimiters: [&'static str; 1],
}

fn rust_syntax_surface() -> WireSyntaxSurface {
    WireSyntaxSurface {
        line_comment: "//",
        block_comment: ["/*", "*/"],
        delimiter_pairs: [["(", ")"], ["[", "]"], ["{", "}"]],
        statement_terminator: ";",
        string_delimiters: ["\""],
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireAstSemanticEdge {
    src: String,
    dst: String,
    relation: &'static str,
    line: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireAstInspectResponse {
    accepted: bool,
    nodes: Vec<WireAstNodeSpanRecord>,
    edges: Vec<WireAstSemanticEdge>,
    diagnostics: Vec<WireAstSyntaxDiagnostic>,
    syntax_surface: WireSyntaxSurface,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireAstVerifyCandidate {
    candidate_id: String,
    source_utf8: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireAstVerifyRequest {
    #[serde(default)]
    _path: Option<String>,
    candidates: Vec<WireAstVerifyCandidate>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireAstVerifyCandidateResult {
    candidate_id: String,
    accepted: bool,
    diagnostics: Vec<WireAstSyntaxDiagnostic>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireAstVerifyResponse {
    results: Vec<WireAstVerifyCandidateResult>,
}

pub(super) fn handle_ast_inspect_value(payload: &Value) -> Result<Vec<u8>, String> {
    let req: WireAstInspectRequest = serde_json::from_value(payload.clone())
        .map_err(|err| format!("decode ast-inspect request: {err}"))?;
    let response = inspect_rust_source(&req.path, &req.source_utf8);
    serde_json::to_vec(&response).map_err(|err| format!("encode ast-inspect response: {err}"))
}

pub(super) fn handle_ast_verify_value(payload: &Value) -> Result<Vec<u8>, String> {
    let req: WireAstVerifyRequest = serde_json::from_value(payload.clone())
        .map_err(|err| format!("decode ast-verify request: {err}"))?;
    let results = req
        .candidates
        .into_iter()
        .map(|cand| {
            let diagnostics = verify_rust_candidate(&cand.source_utf8);
            WireAstVerifyCandidateResult {
                candidate_id: cand.candidate_id,
                accepted: diagnostics.is_empty(),
                diagnostics,
            }
        })
        .collect();
    serde_json::to_vec(&WireAstVerifyResponse { results })
        .map_err(|err| format!("encode ast-verify response: {err}"))
}

fn inspect_rust_source(path: &str, source: &str) -> WireAstInspectResponse {
    match crate::parser::parse_rust_source_syntax(source) {
        Ok(file) => {
            let mut nodes = Vec::new();
            let mut edges = Vec::new();
            collect_items(path, source, None, &file.items, &mut nodes, &mut edges);
            WireAstInspectResponse {
                accepted: true,
                nodes,
                edges,
                diagnostics: Vec::new(),
                syntax_surface: rust_syntax_surface(),
            }
        }
        Err(err) => WireAstInspectResponse {
            accepted: false,
            nodes: Vec::new(),
            edges: Vec::new(),
            diagnostics: vec![syn_error_to_diagnostic(&err)],
            syntax_surface: rust_syntax_surface(),
        },
    }
}

fn verify_rust_candidate(source: &str) -> Vec<WireAstSyntaxDiagnostic> {
    match crate::parser::parse_rust_source_syntax(source) {
        Ok(_) => Vec::new(),
        Err(err) => vec![syn_error_to_diagnostic(&err)],
    }
}

fn syn_error_to_diagnostic(err: &syn::Error) -> WireAstSyntaxDiagnostic {
    let range = err.span().byte_range();
    let msg = err.to_string();
    let expected_tokens = extract_expected_tokens(&msg);
    WireAstSyntaxDiagnostic {
        byte_start: range.start,
        byte_end: range.end.max(range.start),
        message: msg,
        expected_tokens,
    }
}

fn extract_expected_tokens(message: &str) -> Vec<String> {
    [";", "}", ")", "]", ","]
        .into_iter()
        .filter(|tok| message.contains(&format!("expected `{tok}`")))
        .map(str::to_owned)
        .collect()
}

fn line_of_offset(source: &str, offset: usize) -> usize {
    1 + source
        .as_bytes()
        .get(..offset.min(source.len()))
        .map_or(0, |slice| slice.iter().filter(|&&b| b == b'\n').count())
}

fn item_inner_start(outer_start: usize, attrs: &[syn::Attribute], keyword_start: usize) -> usize {
    if attrs.is_empty() {
        outer_start
    } else {
        keyword_start.max(outer_start)
    }
}

fn block_interior_span(block: &syn::Block) -> Option<WireByteSpan> {
    let range = block.brace_token.span.span().byte_range();
    (range.end >= range.start + 2).then_some(WireByteSpan {
        start: range.start + 1,
        end: range.end - 1,
    })
}

fn vis_attrs(vis: &syn::Visibility) -> Vec<WireAstNodeAttr> {
    let label = match vis {
        syn::Visibility::Public(_) => "pub",
        syn::Visibility::Restricted(r) => {
            if r.path.is_ident("crate") {
                "pub(crate)"
            } else if r.path.is_ident("super") {
                "pub(super)"
            } else {
                "pub(restricted)"
            }
        }
        syn::Visibility::Inherited => "private",
    };
    vec![WireAstNodeAttr {
        key: "visibility".to_owned(),
        value: label.to_owned(),
    }]
}

fn push_attr(attrs: &mut Vec<WireAstNodeAttr>, key: &str, value: impl Into<String>) {
    let v = value.into();
    if !v.is_empty() && !attrs.iter().any(|a| a.key == key && a.value == v) {
        attrs.push(WireAstNodeAttr {
            key: key.to_owned(),
            value: v,
        });
    }
}

fn collect_syn_item_meta_attrs(syn_attrs: &[syn::Attribute], out: &mut Vec<WireAstNodeAttr>) {
    for attr in syn_attrs {
        let Some(last) = attr.path().segments.last() else {
            continue;
        };
        let ident = last.ident.to_string();
        if ident == "test" {
            push_attr(out, "is_test", "true");
        } else if ident == "derive" {
            let _ = attr.parse_nested_meta(|meta| {
                if let Some(seg) = meta.path.segments.last() {
                    push_attr(out, "derive", seg.ident.to_string());
                }
                Ok(())
            });
        } else if ident == "error"
            && let Ok(lit) = attr.parse_args::<syn::LitStr>()
        {
            let msg = lit.value();
            if !msg.is_empty() && msg.len() <= 120 {
                push_attr(out, "error_literal", msg);
            }
        }
    }
}

fn fn_sig_attrs(
    vis: &syn::Visibility,
    sig: &syn::Signature,
    syn_attrs: &[syn::Attribute],
) -> Vec<WireAstNodeAttr> {
    let mut attrs = vis_attrs(vis);
    if sig.asyncness.is_some() {
        push_attr(&mut attrs, "async", "true");
    }
    if matches!(sig.safety, syn::Safety::Unsafe(_)) {
        push_attr(&mut attrs, "unsafe", "true");
    }
    if sig.constness.is_some() {
        push_attr(&mut attrs, "const", "true");
    }
    collect_syn_item_meta_attrs(syn_attrs, &mut attrs);
    attrs
}

fn extract_first_string_literal_from_tokens(tokens: &proc_macro2::TokenStream) -> Option<String> {
    for tt in tokens.clone() {
        if let proc_macro2::TokenTree::Literal(lit) = tt
            && let Ok(lit_str) = syn::parse2::<syn::LitStr>(proc_macro2::TokenTree::Literal(lit).into())
        {
            let value = lit_str.value();
            if value.len() >= 3 && value.len() <= 120 {
                return Some(value);
            }
        }
    }
    None
}

struct CallEdgeCollector<'a> {
    caller_locator: &'a str,
    source: &'a str,
    seen: BTreeSet<(String, usize)>,
    edges: &'a mut Vec<WireAstSemanticEdge>,
    error_literals: Vec<String>,
}

impl CallEdgeCollector<'_> {
    fn record_call(&mut self, callee: String, offset: usize) {
        if callee.is_empty() {
            return;
        }
        let line = line_of_offset(self.source, offset);
        if self.seen.insert((callee.clone(), line)) {
            self.edges.push(WireAstSemanticEdge {
                src: self.caller_locator.to_owned(),
                dst: callee,
                relation: "calls",
                line,
            });
        }
    }

    fn record_error_literal(&mut self, value: String) {
        if value.len() >= 3
            && value.len() <= 120
            && self.error_literals.len() < 8
            && !self.error_literals.contains(&value)
        {
            self.error_literals.push(value);
        }
    }
}

impl<'ast> Visit<'ast> for CallEdgeCollector<'_> {
    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        if let syn::Expr::Path(ep) = expr.func.as_ref()
            && let Some(seg) = ep.path.segments.last()
        {
            let name = seg.ident.to_string();
            if name == "Err"
                && let Some(syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                })) = expr.args.first()
            {
                self.record_error_literal(s.value());
            }
            self.record_call(name, expr.span().byte_range().start);
        }
        visit::visit_expr_call(self, expr);
    }

    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        self.record_call(
            expr.method.to_string(),
            expr.method.span().byte_range().start,
        );
        visit::visit_expr_method_call(self, expr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(seg) = mac.path.segments.last() {
            let name = seg.ident.to_string();
            if matches!(
                name.as_str(),
                "panic" | "bail" | "anyhow" | "format_err" | "unreachable"
            ) && let Some(msg) = extract_first_string_literal_from_tokens(&mac.tokens)
            {
                self.record_error_literal(msg);
            }
        }
        visit::visit_macro(self, mac);
    }
}

fn collect_block_call_edges(
    caller_locator: &str,
    source: &str,
    block: &syn::Block,
    edges: &mut Vec<WireAstSemanticEdge>,
) -> Vec<String> {
    let mut visitor = CallEdgeCollector {
        caller_locator,
        source,
        seen: BTreeSet::new(),
        edges,
        error_literals: Vec::new(),
    };
    visitor.visit_block(block);
    visitor.error_literals
}

fn collect_use_tree_targets(prefix: &str, tree: &syn::UseTree, out: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(p) => {
            let next = if prefix.is_empty() {
                p.ident.to_string()
            } else {
                format!("{prefix}::{}", p.ident)
            };
            collect_use_tree_targets(&next, &p.tree, out);
        }
        syn::UseTree::Name(n) => {
            let full = if prefix.is_empty() {
                n.ident.to_string()
            } else {
                format!("{prefix}::{}", n.ident)
            };
            out.push(full);
        }
        syn::UseTree::Rename(r) => {
            let full = if prefix.is_empty() {
                r.ident.to_string()
            } else {
                format!("{prefix}::{}", r.ident)
            };
            out.push(full);
        }
        syn::UseTree::Glob(_) => {
            if !prefix.is_empty() {
                out.push(format!("{prefix}::*"));
            }
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                collect_use_tree_targets(prefix, item, out);
            }
        }
    }
}

fn collect_items(
    path: &str,
    source: &str,
    parent: Option<&str>,
    items: &[syn::Item],
    out: &mut Vec<WireAstNodeSpanRecord>,
    edges: &mut Vec<WireAstSemanticEdge>,
) {
    for item in items {
        match item {
            syn::Item::Fn(item_fn) => push_fn_item(path, source, parent, item_fn, out, edges),
            syn::Item::Struct(item_st) => push_simple_item(
                path,
                source,
                parent,
                "struct",
                "struct_item",
                "struct",
                &item_st.ident.to_string(),
                &item_st.attrs,
                &item_st.vis,
                item_st.span().byte_range(),
                item_st.struct_token.span.byte_range().start,
                out,
            ),
            syn::Item::Enum(item_en) => push_simple_item(
                path,
                source,
                parent,
                "enum",
                "enum_item",
                "enum",
                &item_en.ident.to_string(),
                &item_en.attrs,
                &item_en.vis,
                item_en.span().byte_range(),
                item_en.enum_token.span.byte_range().start,
                out,
            ),
            syn::Item::Trait(item_tr) => push_simple_item(
                path,
                source,
                parent,
                "trait",
                "trait_item",
                "trait",
                &item_tr.ident.to_string(),
                &item_tr.attrs,
                &item_tr.vis,
                item_tr.span().byte_range(),
                item_tr.trait_token.span.byte_range().start,
                out,
            ),
            syn::Item::Type(item_ty) => push_simple_item(
                path,
                source,
                parent,
                "type",
                "type_item",
                "type",
                &item_ty.ident.to_string(),
                &item_ty.attrs,
                &item_ty.vis,
                item_ty.span().byte_range(),
                item_ty.type_token.span.byte_range().start,
                out,
            ),
            syn::Item::Impl(item_impl) => {
                push_impl_item(path, source, parent, item_impl, out, edges)
            }
            syn::Item::Use(item_use) => push_use_item(path, source, parent, item_use, out, edges),
            syn::Item::Mod(item_mod) => push_mod_item(path, source, parent, item_mod, out, edges),
            syn::Item::Macro(item_mac) => push_macro_item(path, source, parent, item_mac, out),
            _ => {}
        }
    }
}

fn build_locator(path: &str, parent: Option<&str>, tag: &str, name: &str) -> String {
    match parent {
        Some(prefix) => format!("{prefix}#{tag}:{name}"),
        None => format!("{path}#{tag}:{name}"),
    }
}

fn push_fn_item(
    path: &str,
    source: &str,
    parent: Option<&str>,
    item_fn: &syn::ItemFn,
    out: &mut Vec<WireAstNodeSpanRecord>,
    edges: &mut Vec<WireAstSemanticEdge>,
) {
    let outer = item_fn.span().byte_range();
    let vis_start = match &item_fn.vis {
        syn::Visibility::Inherited => item_fn.sig.span().byte_range().start,
        vis => vis.span().byte_range().start,
    };
    let inner_start = item_inner_start(outer.start, &item_fn.attrs, vis_start);
    let name = item_fn.sig.ident.to_string();
    let locator = build_locator(path, parent, "fn", &name);
    let err_lits = collect_block_call_edges(&locator, source, &item_fn.block, edges);
    let mut node_attrs = fn_sig_attrs(&item_fn.vis, &item_fn.sig, &item_fn.attrs);
    for msg in err_lits {
        push_attr(&mut node_attrs, "error_literal", msg);
    }
    out.push(WireAstNodeSpanRecord {
        locator,
        kind: "function",
        language_kind: "fn_item",
        name,
        line: line_of_offset(source, inner_start),
        outer_span: WireByteSpan {
            start: outer.start,
            end: outer.end,
        },
        inner_span: WireByteSpan {
            start: inner_start,
            end: outer.end,
        },
        body_span: block_interior_span(&item_fn.block),
        parent: parent.map(str::to_owned),
        attrs: node_attrs,
    });
}

#[allow(clippy::too_many_arguments)]
fn push_simple_item(
    path: &str,
    source: &str,
    parent: Option<&str>,
    kind: &'static str,
    language_kind: &'static str,
    tag: &str,
    name: &str,
    attrs: &[syn::Attribute],
    vis: &syn::Visibility,
    outer: std::ops::Range<usize>,
    kw_start: usize,
    out: &mut Vec<WireAstNodeSpanRecord>,
) {
    let vis_start = match vis {
        syn::Visibility::Inherited => kw_start,
        v => v.span().byte_range().start,
    };
    let inner_start = item_inner_start(outer.start, attrs, vis_start);
    let mut node_attrs = vis_attrs(vis);
    collect_syn_item_meta_attrs(attrs, &mut node_attrs);
    out.push(WireAstNodeSpanRecord {
        locator: build_locator(path, parent, tag, name),
        kind,
        language_kind,
        name: name.to_owned(),
        line: line_of_offset(source, inner_start),
        outer_span: WireByteSpan {
            start: outer.start,
            end: outer.end,
        },
        inner_span: WireByteSpan {
            start: inner_start,
            end: outer.end,
        },
        body_span: None,
        parent: parent.map(str::to_owned),
        attrs: node_attrs,
    });
}

fn push_impl_item(
    path: &str,
    source: &str,
    parent: Option<&str>,
    item_impl: &syn::ItemImpl,
    out: &mut Vec<WireAstNodeSpanRecord>,
    edges: &mut Vec<WireAstSemanticEdge>,
) {
    let outer = item_impl.span().byte_range();
    let inner_start = item_inner_start(
        outer.start,
        &item_impl.attrs,
        item_impl.impl_token.span.byte_range().start,
    );
    let ty_name = quote::ToTokens::to_token_stream(&item_impl.self_ty)
        .to_string()
        .replace(' ', "");
    let locator = build_locator(path, parent, "impl", &ty_name);
    let brace = item_impl.brace_token.span.span().byte_range();
    let body_span = (brace.end >= brace.start + 2).then_some(WireByteSpan {
        start: brace.start + 1,
        end: brace.end - 1,
    });
    let line = line_of_offset(source, inner_start);
    let mut attrs = Vec::new();
    if item_impl.unsafety.is_some() {
        push_attr(&mut attrs, "unsafe", "true");
    }
    if let Some((trait_path, _)) = &item_impl.trait_
        && let Some(seg) = trait_path.segments.last()
    {
        let trait_name = seg.ident.to_string();
        attrs.push(WireAstNodeAttr {
            key: "implements".to_owned(),
            value: trait_name.clone(),
        });
        edges.push(WireAstSemanticEdge {
            src: locator.clone(),
            dst: trait_name,
            relation: "implements",
            line,
        });
    }
    out.push(WireAstNodeSpanRecord {
        locator: locator.clone(),
        kind: "item",
        language_kind: "impl_item",
        name: ty_name,
        line,
        outer_span: WireByteSpan {
            start: outer.start,
            end: outer.end,
        },
        inner_span: WireByteSpan {
            start: inner_start,
            end: outer.end,
        },
        body_span,
        parent: parent.map(str::to_owned),
        attrs,
    });
    for impl_item in &item_impl.items {
        if let syn::ImplItem::Fn(method) = impl_item {
            let m_outer = method.span().byte_range();
            let m_vis_start = match &method.vis {
                syn::Visibility::Inherited => method.sig.span().byte_range().start,
                v => v.span().byte_range().start,
            };
            let m_inner = item_inner_start(m_outer.start, &method.attrs, m_vis_start);
            let m_name = method.sig.ident.to_string();
            let m_locator = format!("{locator}#fn:{m_name}");
            let m_errs = collect_block_call_edges(&m_locator, source, &method.block, edges);
            let mut m_attrs = fn_sig_attrs(&method.vis, &method.sig, &method.attrs);
            for msg in m_errs {
                push_attr(&mut m_attrs, "error_literal", msg);
            }
            out.push(WireAstNodeSpanRecord {
                locator: m_locator,
                kind: "method",
                language_kind: "impl_fn",
                name: m_name,
                line: line_of_offset(source, m_inner),
                outer_span: WireByteSpan {
                    start: m_outer.start,
                    end: m_outer.end,
                },
                inner_span: WireByteSpan {
                    start: m_inner,
                    end: m_outer.end,
                },
                body_span: block_interior_span(&method.block),
                parent: Some(locator.clone()),
                attrs: m_attrs,
            });
        }
    }
}

fn push_use_item(
    path: &str,
    source: &str,
    parent: Option<&str>,
    item_use: &syn::ItemUse,
    out: &mut Vec<WireAstNodeSpanRecord>,
    edges: &mut Vec<WireAstSemanticEdge>,
) {
    let outer = item_use.span().byte_range();
    let line = line_of_offset(source, outer.start);
    let label = format!("use_line_{line}");
    let mut targets = Vec::new();
    collect_use_tree_targets("", &item_use.tree, &mut targets);
    let mut attrs = vis_attrs(&item_use.vis);
    if let Some(first) = targets.first() {
        attrs.push(WireAstNodeAttr {
            key: "import_target".to_owned(),
            value: first.clone(),
        });
    }
    for target in targets {
        edges.push(WireAstSemanticEdge {
            src: path.to_owned(),
            dst: target,
            relation: "imports",
            line,
        });
    }
    out.push(WireAstNodeSpanRecord {
        locator: build_locator(path, parent, "use", &label),
        kind: "import",
        language_kind: "use_item",
        name: label,
        line,
        outer_span: WireByteSpan {
            start: outer.start,
            end: outer.end,
        },
        inner_span: WireByteSpan {
            start: outer.start,
            end: outer.end,
        },
        body_span: None,
        parent: parent.map(str::to_owned),
        attrs,
    });
}

fn push_mod_item(
    path: &str,
    source: &str,
    parent: Option<&str>,
    item_mod: &syn::ItemMod,
    out: &mut Vec<WireAstNodeSpanRecord>,
    edges: &mut Vec<WireAstSemanticEdge>,
) {
    let outer = item_mod.span().byte_range();
    let name = item_mod.ident.to_string();
    let locator = build_locator(path, parent, "mod", &name);
    let vis_start = match &item_mod.vis {
        syn::Visibility::Inherited => item_mod.mod_token.span.byte_range().start,
        v => v.span().byte_range().start,
    };
    let inner_start = item_inner_start(outer.start, &item_mod.attrs, vis_start);
    out.push(WireAstNodeSpanRecord {
        locator: locator.clone(),
        kind: "module",
        language_kind: "mod_item",
        name,
        line: line_of_offset(source, inner_start),
        outer_span: WireByteSpan {
            start: outer.start,
            end: outer.end,
        },
        inner_span: WireByteSpan {
            start: inner_start,
            end: outer.end,
        },
        body_span: None,
        parent: parent.map(str::to_owned),
        attrs: vis_attrs(&item_mod.vis),
    });
    if let Some((_, nested)) = &item_mod.content {
        collect_items(path, source, Some(&locator), nested, out, edges);
    }
}

fn push_macro_item(
    path: &str,
    source: &str,
    parent: Option<&str>,
    item_mac: &syn::ItemMacro,
    out: &mut Vec<WireAstNodeSpanRecord>,
) {
    let Some(ident) = &item_mac.ident else {
        return;
    };
    let outer = item_mac.span().byte_range();
    let name = ident.to_string();
    out.push(WireAstNodeSpanRecord {
        locator: build_locator(path, parent, "macro", &name),
        kind: "macro",
        language_kind: "macro_rules",
        name,
        line: line_of_offset(source, outer.start),
        outer_span: WireByteSpan {
            start: outer.start,
            end: outer.end,
        },
        inner_span: WireByteSpan {
            start: outer.start,
            end: outer.end,
        },
        body_span: None,
        parent: parent.map(str::to_owned),
        attrs: Vec::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_emits_rich_visibility_async_unsafe_derive_test_and_error_literal_attrs() {
        let src = r#"
#[derive(Clone, Debug)]
pub struct Engine;

#[derive(Debug)]
pub(crate) enum GateErr {
    #[error("continuation blocked")]
    Blocked,
}

pub async unsafe fn dispatch(x: bool) -> Result<(), &'static str> {
    if !x {
        return Err("missing trace context");
    }
    helper();
    Ok(())
}

#[test]
fn smoke() {}
"#;
        let resp = inspect_rust_source("src/lib.rs", src);
        assert!(resp.accepted);
        let engine = resp.nodes.iter().find(|n| n.name == "Engine").expect("Engine");
        assert!(engine.attrs.iter().any(|a| a.key == "visibility" && a.value == "pub"));
        assert!(engine.attrs.iter().any(|a| a.key == "derive" && a.value == "Clone"));
        let gate_err = resp.nodes.iter().find(|n| n.name == "GateErr").expect("GateErr");
        assert!(gate_err.attrs.iter().any(|a| a.key == "visibility" && a.value == "pub(crate)"));
        let dispatch = resp.nodes.iter().find(|n| n.name == "dispatch").expect("dispatch");
        assert!(dispatch.attrs.iter().any(|a| a.key == "visibility" && a.value == "pub"));
        assert!(dispatch.attrs.iter().any(|a| a.key == "async" && a.value == "true"));
        assert!(dispatch.attrs.iter().any(|a| a.key == "unsafe" && a.value == "true"));
        assert!(dispatch.attrs.iter().any(|a| a.key == "error_literal" && a.value == "missing trace context"));
        let smoke = resp.nodes.iter().find(|n| n.name == "smoke").expect("smoke");
        assert!(smoke.attrs.iter().any(|a| a.key == "is_test" && a.value == "true"));
        assert!(smoke.attrs.iter().any(|a| a.key == "visibility" && a.value == "private"));
        assert!(resp.edges.iter().any(|e| e.relation == "calls" && e.dst == "helper"));
    }
}
