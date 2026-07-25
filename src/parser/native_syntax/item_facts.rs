//! Native Rust top-level item facts.

use std::path::Path;

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;

use super::facts::RustTopLevelItemSyntax;
use super::invocation_facts::{include_literal_target, invocation_syntax};
use super::item_projection::item_projection_nodes;
use super::module_facts::{
    attribute_is_cfg, attribute_is_cfg_feature, attribute_is_proc_macro_export,
    module_declaration_syntax,
};

pub(super) fn top_level_item_syntax(
    item: &syn::Item,
    source_file: &Path,
) -> RustTopLevelItemSyntax {
    let start_line = item.span().start().line.max(1);
    let end_line = item.span().end().line.max(start_line);
    RustTopLevelItemSyntax {
        line: start_line,
        end_line,
        kind: item_kind(item),
        name: item_name(item),
        impl_target_name: impl_target_name_syntax(item),
        trait_owner_name: trait_owner_name_syntax(item),
        has_doc: item_attrs(item)
            .iter()
            .any(|attr| attr.path().is_ident("doc")),
        is_public: item_visibility(item).is_some_and(is_public_visibility),
        is_public_use: is_public_use(item),
        is_use: matches!(item, syn::Item::Use(_)),
        is_extern_crate: matches!(item, syn::Item::ExternCrate(_)),
        is_macro: matches!(item, syn::Item::Macro(_)),
        has_proc_macro_export_attr: item_attrs(item).iter().any(attribute_is_proc_macro_export),
        has_cfg_attr: item_attrs(item).iter().any(attribute_is_cfg),
        is_implementation_item: is_implementation_item(item),
        function_name: function_name_syntax(item),
        macro_name: macro_name_syntax(item),
        macro_declares_module: macro_declares_module_syntax(item),
        macro_body_is_facade_boundary: macro_body_is_facade_boundary_syntax(item),
        include_target: include_target_syntax(item),
        module: module_declaration_syntax(item, source_file),
        cfg_predicates: item_attrs(item)
            .iter()
            .filter_map(|attribute| {
                if !attribute.path().is_ident("cfg") {
                    return None;
                }
                let syn::Meta::List(meta) = &attribute.meta else {
                    return None;
                };
                Some(quote::ToTokens::to_token_stream(&meta.tokens).to_string())
            })
            .collect(),
        projection_responsibilities: item_projection_responsibilities(item),
        projection_nodes: item_projection_nodes(item),
    }
}

fn item_name(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Const(item) => Some(item.ident.to_string()),
        syn::Item::Enum(item) => Some(item.ident.to_string()),
        syn::Item::ExternCrate(item) => Some(item.ident.to_string()),
        syn::Item::Fn(item) => Some(item.sig.ident.to_string()),
        syn::Item::Mod(item) => Some(item.ident.to_string()),
        syn::Item::Static(item) => Some(item.ident.to_string()),
        syn::Item::Struct(item) => Some(item.ident.to_string()),
        syn::Item::Trait(item) => Some(item.ident.to_string()),
        syn::Item::TraitAlias(item) => Some(item.ident.to_string()),
        syn::Item::Type(item) => Some(item.ident.to_string()),
        syn::Item::Union(item) => Some(item.ident.to_string()),
        _ => None,
    }
}

fn impl_target_name_syntax(item: &syn::Item) -> Option<String> {
    let syn::Item::Impl(item_impl) = item else {
        return None;
    };
    type_terminal_name(&item_impl.self_ty)
}

fn trait_owner_name_syntax(item: &syn::Item) -> Option<String> {
    let syn::Item::Impl(item_impl) = item else {
        return None;
    };
    item_impl.trait_.as_ref().map(|(_, path, _)| {
        quote::ToTokens::to_token_stream(path)
            .to_string()
            .replace(' ', "")
    })
}

fn type_terminal_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Type::Reference(reference) => type_terminal_name(&reference.elem),
        syn::Type::Paren(paren) => type_terminal_name(&paren.elem),
        syn::Type::Group(group) => type_terminal_name(&group.elem),
        _ => None,
    }
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        _ => &[],
    }
}

fn item_projection_responsibilities(item: &syn::Item) -> Vec<&'static str> {
    let mut responsibilities = Vec::new();
    let attrs = item_attrs(item);
    if attrs.iter().any(attribute_is_cfg_feature) {
        responsibilities.push("cfg-feature-gate");
    } else if attrs.iter().any(attribute_is_cfg) {
        responsibilities.push("cfg-gated-item");
    }
    if item_is_async(item) {
        responsibilities.push("async-item");
    }
    if item_has_generic_bound(item) {
        responsibilities.push("generic-bound");
    }
    responsibilities
}

fn item_is_async(item: &syn::Item) -> bool {
    matches!(item, syn::Item::Fn(item_fn) if item_fn.sig.asyncness.is_some())
}

fn item_has_generic_bound(item: &syn::Item) -> bool {
    item_generics(item).is_some_and(generics_have_bounds)
}

fn item_generics(item: &syn::Item) -> Option<&syn::Generics> {
    match item {
        syn::Item::Const(item) => Some(&item.generics),
        syn::Item::Enum(item) => Some(&item.generics),
        syn::Item::Fn(item) => Some(&item.sig.generics),
        syn::Item::Impl(item) => Some(&item.generics),
        syn::Item::Struct(item) => Some(&item.generics),
        syn::Item::Trait(item) => Some(&item.generics),
        syn::Item::TraitAlias(item) => Some(&item.generics),
        syn::Item::Type(item) => Some(&item.generics),
        syn::Item::Union(item) => Some(&item.generics),
        _ => None,
    }
}

fn generics_have_bounds(generics: &syn::Generics) -> bool {
    generics
        .params
        .iter()
        .any(|param| matches!(param, syn::GenericParam::Type(param) if !param.bounds.is_empty()))
        || generics
            .where_clause
            .as_ref()
            .is_some_and(|where_clause| !where_clause.predicates.is_empty())
}

fn function_name_syntax(item: &syn::Item) -> Option<String> {
    let syn::Item::Fn(item_fn) = item else {
        return None;
    };
    Some(item_fn.sig.ident.to_string())
}

fn macro_name_syntax(item: &syn::Item) -> Option<String> {
    let syn::Item::Macro(item_macro) = item else {
        return None;
    };
    invocation_syntax(&item_macro.mac.path).map(|invocation| invocation.terminal_name)
}

fn macro_declares_module_syntax(item: &syn::Item) -> bool {
    let syn::Item::Macro(item_macro) = item else {
        return false;
    };
    token_stream_declares_module(&item_macro.mac.tokens)
}

fn macro_body_is_facade_boundary_syntax(item: &syn::Item) -> bool {
    let syn::Item::Macro(item_macro) = item else {
        return false;
    };
    token_stream_is_facade_boundary(&item_macro.mac.tokens)
}

fn token_stream_is_facade_boundary(tokens: &TokenStream) -> bool {
    let Ok(file) = syn::parse2::<syn::File>(tokens.clone()) else {
        return false;
    };
    !file.items.is_empty() && file.items.iter().all(item_is_facade_boundary)
}

fn item_is_facade_boundary(item: &syn::Item) -> bool {
    match item {
        syn::Item::ExternCrate(_) | syn::Item::Use(_) => true,
        syn::Item::Mod(item_mod) => item_mod.content.is_none(),
        syn::Item::Macro(item_macro) => {
            invocation_syntax(&item_macro.mac.path)
                .is_some_and(|invocation| invocation.terminal_name != "macro_rules")
                && token_stream_is_facade_boundary(&item_macro.mac.tokens)
        }
        _ => false,
    }
}

fn token_stream_declares_module(tokens: &TokenStream) -> bool {
    let mut iter = tokens.clone().into_iter().peekable();
    while let Some(token) = iter.next() {
        match token {
            TokenTree::Group(group) if token_stream_declares_module(&group.stream()) => {
                return true;
            }
            TokenTree::Ident(ident)
                if ident == "mod"
                    && iter
                        .peek()
                        .is_some_and(|next| matches!(next, TokenTree::Ident(_))) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn include_target_syntax(item: &syn::Item) -> Option<String> {
    let syn::Item::Macro(item_macro) = item else {
        return None;
    };
    include_literal_target(&item_macro.mac)
}

fn item_kind(item: &syn::Item) -> &'static str {
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

fn item_visibility(item: &syn::Item) -> Option<&syn::Visibility> {
    match item {
        syn::Item::Const(item) => Some(&item.vis),
        syn::Item::Enum(item) => Some(&item.vis),
        syn::Item::Fn(item) => Some(&item.vis),
        syn::Item::Mod(item) => Some(&item.vis),
        syn::Item::Static(item) => Some(&item.vis),
        syn::Item::Struct(item) => Some(&item.vis),
        syn::Item::Trait(item) => Some(&item.vis),
        syn::Item::TraitAlias(item) => Some(&item.vis),
        syn::Item::Type(item) => Some(&item.vis),
        syn::Item::Union(item) => Some(&item.vis),
        _ => None,
    }
}

fn is_public_visibility(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn is_public_use(item: &syn::Item) -> bool {
    matches!(item, syn::Item::Use(item_use) if is_public_visibility(&item_use.vis))
}

fn is_implementation_item(item: &syn::Item) -> bool {
    matches!(
        item,
        syn::Item::Const(_)
            | syn::Item::Enum(_)
            | syn::Item::Fn(_)
            | syn::Item::Impl(_)
            | syn::Item::Static(_)
            | syn::Item::Struct(_)
            | syn::Item::Trait(_)
            | syn::Item::TraitAlias(_)
            | syn::Item::Type(_)
            | syn::Item::Union(_)
    )
}
