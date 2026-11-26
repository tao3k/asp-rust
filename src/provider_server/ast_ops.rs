//! Native Rust `syn` AST inspection (`ast-inspect`) and batched compiler
//! oracle verification (`ast-verify`) for ASP's `agent-semantic-ast` protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use syn::spanned::Spanned as _;

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
struct WireAstInspectResponse {
    accepted: bool,
    nodes: Vec<WireAstNodeSpanRecord>,
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
    match syn::parse_file(source) {
        Ok(file) => {
            let mut nodes = Vec::new();
            collect_items(path, source, None, &file.items, &mut nodes);
            WireAstInspectResponse {
                accepted: true,
                nodes,
                diagnostics: Vec::new(),
                syntax_surface: rust_syntax_surface(),
            }
        }
        Err(err) => WireAstInspectResponse {
            accepted: false,
            nodes: Vec::new(),
            diagnostics: vec![syn_error_to_diagnostic(&err)],
            syntax_surface: rust_syntax_surface(),
        },
    }
}

fn verify_rust_candidate(source: &str) -> Vec<WireAstSyntaxDiagnostic> {
    match syn::parse_file(source) {
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
    if matches!(vis, syn::Visibility::Public(_)) {
        vec![WireAstNodeAttr {
            key: "visibility".to_owned(),
            value: "pub".to_owned(),
        }]
    } else {
        Vec::new()
    }
}

fn collect_items(
    path: &str,
    source: &str,
    parent: Option<&str>,
    items: &[syn::Item],
    out: &mut Vec<WireAstNodeSpanRecord>,
) {
    for item in items {
        match item {
            syn::Item::Fn(item_fn) => push_fn_item(path, source, parent, item_fn, out),
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
            syn::Item::Impl(item_impl) => push_impl_item(path, source, parent, item_impl, out),
            syn::Item::Use(item_use) => push_use_item(path, source, parent, item_use, out),
            syn::Item::Mod(item_mod) => push_mod_item(path, source, parent, item_mod, out),
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
) {
    let outer = item_fn.span().byte_range();
    let vis_start = match &item_fn.vis {
        syn::Visibility::Inherited => item_fn.sig.span().byte_range().start,
        vis => vis.span().byte_range().start,
    };
    let inner_start = item_inner_start(outer.start, &item_fn.attrs, vis_start);
    let name = item_fn.sig.ident.to_string();
    out.push(WireAstNodeSpanRecord {
        locator: build_locator(path, parent, "fn", &name),
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
        attrs: vis_attrs(&item_fn.vis),
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
        attrs: vis_attrs(vis),
    });
}

fn push_impl_item(
    path: &str,
    source: &str,
    parent: Option<&str>,
    item_impl: &syn::ItemImpl,
    out: &mut Vec<WireAstNodeSpanRecord>,
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
    out.push(WireAstNodeSpanRecord {
        locator: locator.clone(),
        kind: "item",
        language_kind: "impl_item",
        name: ty_name,
        line: line_of_offset(source, inner_start),
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
        attrs: Vec::new(),
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
            out.push(WireAstNodeSpanRecord {
                locator: format!("{locator}#fn:{m_name}"),
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
                attrs: vis_attrs(&method.vis),
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
) {
    let outer = item_use.span().byte_range();
    let line = line_of_offset(source, outer.start);
    let label = format!("use_line_{line}");
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
        attrs: vis_attrs(&item_use.vis),
    });
}

fn push_mod_item(
    path: &str,
    source: &str,
    parent: Option<&str>,
    item_mod: &syn::ItemMod,
    out: &mut Vec<WireAstNodeSpanRecord>,
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
        collect_items(path, source, Some(&locator), nested, out);
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
