use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::token_compact::{compact_limited, compact_tokens};
use syn::parse::Parser;

pub(super) fn item_projection_header(item: &syn::Item) -> String {
    match item {
        syn::Item::Fn(item_fn) => fn_signature_projection_label(&item_fn.sig),
        syn::Item::Const(item) => format!("const {}", item.ident),
        syn::Item::Enum(item) => format!("{}enum {} {{", visibility_text(&item.vis), item.ident),
        syn::Item::ExternCrate(item) => format!("extern crate {};", item.ident),
        syn::Item::Macro(item) => macro_call_projection_label(&item.mac),
        syn::Item::Mod(item) => format!("mod {} {{", item.ident),
        syn::Item::Static(item) => format!("static {}", item.ident),
        syn::Item::Struct(item) => {
            format!("{}struct {} {{", visibility_text(&item.vis), item.ident)
        }
        syn::Item::Impl(item) => impl_projection_header(item),
        syn::Item::Trait(item) => format!("{}trait {} {{", visibility_text(&item.vis), item.ident),
        syn::Item::TraitAlias(item) => {
            format!("{}trait {}", visibility_text(&item.vis), item.ident)
        }
        syn::Item::Type(item) => format!("type {};", item.ident),
        syn::Item::Union(item) => format!("{}union {} {{", visibility_text(&item.vis), item.ident),
        _ => compact_tokens(item),
    }
}

pub(super) fn impl_projection_header(item: &syn::ItemImpl) -> String {
    if let Some((trait_path, _)) = &item.trait_ {
        format!(
            "impl {} for {} {{",
            compact_tokens(trait_path),
            compact_tokens(&item.self_ty)
        )
    } else {
        format!("impl {} {{", compact_tokens(&item.self_ty))
    }
}

pub(super) fn fn_signature_projection_label(signature: &syn::Signature) -> String {
    format!("{} {{", compact_tokens(signature))
}

pub(super) fn struct_field_projection_label(field: &syn::Field, index: usize) -> String {
    let name = field
        .ident
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("#{index}"));
    format!(
        "{}{}: {},",
        visibility_text(&field.vis),
        name,
        compact_tokens(&field.ty)
    )
}

pub(super) fn local_projection_label(local: &syn::Local) -> String {
    match &local.init {
        Some(init) => format!(
            "let {} = {};",
            local_pattern_projection_label(&local.pat),
            value_expression_projection_summary(&init.expr)
        ),
        None => format!("let {};", local_pattern_projection_label(&local.pat)),
    }
}

pub(super) fn local_pattern_projection_label(pattern: &syn::Pat) -> String {
    compact_tokens(pattern)
}

pub(super) fn expression_projection_summary(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Call(call) => compact_tokens(call),
        syn::Expr::MethodCall(method_call) => method_call.method.to_string(),
        syn::Expr::Macro(macro_expr) => macro_expression_projection_summary(&macro_expr.mac),
        syn::Expr::Struct(struct_expr) => compact_tokens(struct_expr),
        syn::Expr::Array(array) => array_expression_projection_summary(array),
        syn::Expr::Tuple(tuple) => tuple_expression_projection_summary(tuple),
        syn::Expr::Try(try_expr) => {
            format!("{}?", expression_projection_summary(&try_expr.expr))
        }
        syn::Expr::Await(await_expr) => {
            format!("{}.await", expression_projection_summary(&await_expr.base))
        }
        syn::Expr::Reference(reference) => expression_projection_summary(&reference.expr),
        syn::Expr::Paren(paren) => expression_projection_summary(&paren.expr),
        syn::Expr::Cast(cast) => expression_projection_summary(&cast.expr),
        syn::Expr::Binary(binary) => binary_expression_projection_summary(binary),
        _ => compact_tokens(expression),
    }
}

pub(super) fn condition_projection_summary(expression: &syn::Expr) -> String {
    value_expression_projection_summary(expression)
}

pub(super) fn value_expression_projection_summary(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Array(array) => array_expression_projection_summary(array),
        syn::Expr::Binary(binary) => binary_expression_projection_summary(binary),
        syn::Expr::Lit(lit) => literal_projection_summary(&lit.lit),
        syn::Expr::Macro(macro_expr) => macro_expression_projection_summary(&macro_expr.mac),
        syn::Expr::Reference(reference) => value_expression_projection_summary(&reference.expr),
        syn::Expr::Struct(struct_expr) => struct_expression_projection_summary(struct_expr),
        syn::Expr::Tuple(tuple) => tuple_expression_projection_summary(tuple),
        syn::Expr::Paren(paren) => value_expression_projection_summary(&paren.expr),
        syn::Expr::Cast(cast) => value_expression_projection_summary(&cast.expr),
        _ => compact_tokens(expression),
    }
}

pub(super) fn array_expression_projection_summary(array: &syn::ExprArray) -> String {
    sequence_expression_projection_summary("array", array.elems.iter())
}

pub(super) fn tuple_expression_projection_summary(tuple: &syn::ExprTuple) -> String {
    sequence_expression_projection_summary("tuple", tuple.elems.iter())
}

pub(super) fn sequence_expression_projection_summary<'a>(
    kind: &str,
    elements: impl ExactSizeIterator<Item = &'a syn::Expr>,
) -> String {
    let total = elements.len();
    let labels = elements
        .take(3)
        .map(collection_item_projection_summary)
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>();
    if labels.is_empty() {
        format!("{kind}[{total}]")
    } else {
        format!("{kind}[{total}] items={}", labels.join(","))
    }
}

pub(super) fn collection_item_projection_summary(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Array(array) => compact_limited(&array_expression_projection_summary(array), 72),
        syn::Expr::Lit(lit) => literal_projection_summary(&lit.lit),
        syn::Expr::Macro(macro_expr) => {
            compact_limited(&macro_expression_projection_summary(&macro_expr.mac), 72)
        }
        syn::Expr::Struct(struct_expr) => {
            compact_limited(&struct_expression_projection_summary(struct_expr), 72)
        }
        syn::Expr::Tuple(tuple) => compact_limited(&tuple_expression_projection_summary(tuple), 72),
        _ => value_expression_projection_summary(expression),
    }
}

pub(super) fn struct_expression_projection_summary(struct_expr: &syn::ExprStruct) -> String {
    let mut entries = struct_expr
        .fields
        .iter()
        .take(4)
        .map(|field| {
            format!(
                "{}={}",
                compact_tokens(&field.member),
                value_expression_projection_summary(&field.expr)
            )
        })
        .collect::<Vec<_>>();
    let remaining = struct_expr
        .fields
        .iter()
        .skip(entries.len())
        .map(|field| compact_tokens(&field.member))
        .take(6)
        .collect::<Vec<_>>();
    if !remaining.is_empty() {
        entries.push(format!("keys={}", remaining.join(",")));
    }
    if entries.is_empty() {
        compact_tokens(&struct_expr.path)
    } else {
        format!(
            "{} {}",
            compact_tokens(&struct_expr.path),
            entries.join(" ")
        )
    }
}

pub(super) fn macro_expression_projection_summary(mac: &syn::Macro) -> String {
    if !mac.path.is_ident("vec") {
        return compact_tokens(mac);
    }
    let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
    parser
        .parse2(mac.tokens.clone())
        .map(|arguments| sequence_expression_projection_summary("vec", arguments.iter()))
        .unwrap_or_else(|_| compact_tokens(mac))
}

pub(super) fn literal_projection_summary(literal: &syn::Lit) -> String {
    match literal {
        syn::Lit::Bool(value) => value.value.to_string(),
        syn::Lit::ByteStr(_) => "bytes".to_string(),
        syn::Lit::Char(value) => value.value().to_string(),
        syn::Lit::Float(value) => value.base10_digits().to_string(),
        syn::Lit::Int(value) => value.base10_digits().to_string(),
        syn::Lit::Str(value) => string_literal_projection_summary(&value.value()),
        _ => compact_tokens(literal),
    }
}

fn string_literal_projection_summary(value: &str) -> String {
    if !string_literal_needs_summary(value) {
        return value.to_string();
    }
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    let hash = hasher.finish() as u32;
    format!(
        "string[lines={},bytes={},hash={hash:08x}]",
        value.bytes().filter(|byte| *byte == b'\n').count() + 1,
        value.len()
    )
}

fn string_literal_needs_summary(value: &str) -> bool {
    value.len() > 48
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('\t')
        || value.contains("  ")
}

pub(super) fn binary_expression_projection_summary(binary: &syn::ExprBinary) -> String {
    format!(
        "{} {} {}",
        compact_tokens(&binary.left),
        compact_tokens(&binary.op),
        compact_tokens(&binary.right)
    )
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
}

pub(super) fn return_projection_label(return_expr: &syn::ExprReturn) -> String {
    return_expr.expr.as_deref().map_or_else(
        || "return;".to_string(),
        |expr| format!("return {};", value_expression_projection_summary(expr)),
    )
}

pub(super) fn break_projection_label(break_expr: &syn::ExprBreak) -> String {
    break_expr.expr.as_deref().map_or_else(
        || "break;".to_string(),
        |expr| format!("break {};", value_expression_projection_summary(expr)),
    )
}

pub(super) fn binary_assignment_projection_label(binary: &syn::ExprBinary) -> Option<String> {
    let operator = compact_tokens(&binary.op);
    matches!(
        operator.as_str(),
        "+=" | "-=" | "*=" | "/=" | "%=" | "^=" | "&=" | "|=" | "<<=" | ">>="
    )
    .then(|| {
        format!(
            "{} {} {};",
            compact_tokens(&binary.left),
            operator,
            expression_projection_summary(&binary.right)
        )
    })
}

pub(super) fn macro_call_projection_label(mac: &syn::Macro) -> String {
    let arguments = compact_limited(&macro_arguments_projection(mac), 96);
    if arguments.is_empty() {
        format!("{}!();", compact_tokens(&mac.path))
    } else {
        format!("{}!({});", compact_tokens(&mac.path), arguments)
    }
}

pub(super) fn macro_arguments_projection(mac: &syn::Macro) -> String {
    let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
    parser
        .parse2(mac.tokens.clone())
        .map(|arguments| {
            arguments
                .iter()
                .map(macro_argument_projection)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|_| compact_tokens(&mac.tokens))
}

pub(super) fn macro_argument_projection(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Lit(_) => compact_tokens(expression),
        _ => value_expression_projection_summary(expression),
    }
}

pub(super) fn tail_expression_projection_label(expression: &syn::Expr) -> String {
    value_expression_projection_summary(expression)
}

pub(super) fn visibility_text(visibility: &syn::Visibility) -> String {
    let value = compact_tokens(visibility);
    if value.is_empty() {
        String::new()
    } else {
        format!("{value} ")
    }
}

pub(super) fn item_kind(item: &syn::Item) -> &'static str {
    match item {
        syn::Item::Const(_) => "const",
        syn::Item::Enum(_) => "enum",
        syn::Item::Fn(_) => "fn",
        syn::Item::Impl(_) => "impl",
        syn::Item::Macro(_) => "macro",
        syn::Item::Mod(_) => "mod",
        syn::Item::Static(_) => "static",
        syn::Item::Struct(_) => "struct",
        syn::Item::Trait(_) => "trait",
        syn::Item::TraitAlias(_) => "trait_alias",
        syn::Item::Type(_) => "type",
        syn::Item::Union(_) => "union",
        syn::Item::Use(_) => "use",
        _ => "item",
    }
}
