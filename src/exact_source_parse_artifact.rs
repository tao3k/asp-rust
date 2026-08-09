#[derive(Clone, Debug)]
pub(crate) struct ParseArtifactItem {
    pub(crate) identity: crate::canonical_item_identity::CanonicalItemIdentityV1,
    pub(crate) source_byte_start: usize,
    pub(crate) source_byte_end: usize,
}

pub(crate) fn collect_parse_artifact_items(
    source: &str,
    items: &[syn::Item],
    output: &mut Vec<ParseArtifactItem>,
) {
    for item in items {
        match item {
            syn::Item::Const(item) => {
                push_parse_artifact_item("const", item.ident.to_string(), &item.attrs, item, output)
            }
            syn::Item::Enum(item) => {
                push_parse_artifact_item("enum", item.ident.to_string(), &item.attrs, item, output)
            }
            syn::Item::Fn(item) => push_parse_artifact_item(
                "function",
                item.sig.ident.to_string(),
                &item.attrs,
                item,
                output,
            ),
            syn::Item::Macro(item) => collect_macro_parse_artifact_item(item, output),
            syn::Item::Mod(item) => collect_module_parse_artifact_items(source, item, output),
            syn::Item::Static(item) => push_parse_artifact_item(
                "static",
                item.ident.to_string(),
                &item.attrs,
                item,
                output,
            ),
            syn::Item::Struct(item) => push_parse_artifact_item(
                "struct",
                item.ident.to_string(),
                &item.attrs,
                item,
                output,
            ),
            syn::Item::Trait(item) => collect_trait_parse_artifact_items(item, output),
            syn::Item::TraitAlias(item) => push_parse_artifact_item(
                "trait-alias",
                item.ident.to_string(),
                &item.attrs,
                item,
                output,
            ),
            syn::Item::Type(item) => {
                push_parse_artifact_item("type", item.ident.to_string(), &item.attrs, item, output)
            }
            syn::Item::Union(item) => {
                push_parse_artifact_item("union", item.ident.to_string(), &item.attrs, item, output)
            }
            syn::Item::Impl(item) => collect_impl_parse_artifact_items(item, output),
            syn::Item::Use(item) if !matches!(item.vis, syn::Visibility::Inherited) => {
                let byte_range = syn::spanned::Spanned::span(item).byte_range();
                collect_reexport_items(&item.tree, byte_range.start, byte_range.end, output);
            }
            _ => {}
        }
    }
}

fn collect_macro_parse_artifact_item(item: &syn::ItemMacro, output: &mut Vec<ParseArtifactItem>) {
    if let Some(ident) = item.ident.as_ref() {
        push_parse_artifact_item("macro", ident.to_string(), &item.attrs, item, output);
    }
}

fn collect_module_parse_artifact_items(
    source: &str,
    item: &syn::ItemMod,
    output: &mut Vec<ParseArtifactItem>,
) {
    push_parse_artifact_item("module", item.ident.to_string(), &item.attrs, item, output);
    if let Some((_, nested)) = item.content.as_ref() {
        collect_parse_artifact_items(source, nested, output);
    }
}

fn collect_impl_parse_artifact_items(item: &syn::ItemImpl, output: &mut Vec<ParseArtifactItem>) {
    let impl_owner = quote::ToTokens::to_token_stream(item.self_ty.as_ref())
        .to_string()
        .replace(' ', "");
    let trait_owner = item.trait_.as_ref().map(|(_, path, _)| {
        quote::ToTokens::to_token_stream(path)
            .to_string()
            .replace(' ', "")
    });
    let mut impl_identity = crate::canonical_item_identity::CanonicalItemIdentityV1::new(
        "rust",
        "impl",
        impl_owner.clone(),
    )
    .with_scope("implementation-owner", "type", impl_owner.clone());
    if let Some(trait_owner) = trait_owner.as_deref() {
        impl_identity = impl_identity.with_scope("trait-owner", "trait", trait_owner);
    }
    push_canonical_parse_artifact_item(with_cfg_scopes(impl_identity, &item.attrs), item, output);
    for method in item.items.iter().filter_map(|item| match item {
        syn::ImplItem::Fn(method) => Some(method),
        _ => None,
    }) {
        let mut identity = crate::canonical_item_identity::CanonicalItemIdentityV1::new(
            "rust",
            "method",
            method.sig.ident.to_string(),
        )
        .with_scope("implementation-owner", "type", impl_owner.clone());
        if let Some(trait_owner) = trait_owner.as_deref() {
            identity = identity.with_scope("trait-owner", "trait", trait_owner);
        }
        push_canonical_parse_artifact_item(
            with_cfg_scopes(identity, &method.attrs),
            method,
            output,
        );
    }
}

fn collect_trait_parse_artifact_items(item: &syn::ItemTrait, output: &mut Vec<ParseArtifactItem>) {
    let trait_owner = item.ident.to_string();
    push_parse_artifact_item("trait", trait_owner.clone(), &item.attrs, item, output);
    for method in item.items.iter().filter_map(|item| match item {
        syn::TraitItem::Fn(method) => Some(method),
        _ => None,
    }) {
        let identity = crate::canonical_item_identity::CanonicalItemIdentityV1::new(
            "rust",
            "trait-function",
            method.sig.ident.to_string(),
        )
        .with_scope("trait-owner", "trait", trait_owner.clone());
        push_canonical_parse_artifact_item(
            with_cfg_scopes(identity, &method.attrs),
            method,
            output,
        );
    }
}

fn collect_reexport_items(
    tree: &syn::UseTree,
    source_byte_start: usize,
    source_byte_end: usize,
    output: &mut Vec<ParseArtifactItem>,
) {
    let identity = match tree {
        syn::UseTree::Name(name) => Some(("reexport", name.ident.to_string())),
        syn::UseTree::Rename(rename) => Some(("reexport", rename.rename.to_string())),
        syn::UseTree::Path(path) => {
            return collect_reexport_items(&path.tree, source_byte_start, source_byte_end, output);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_reexport_items(item, source_byte_start, source_byte_end, output);
            }
            None
        }
        syn::UseTree::Glob(_) => None,
    };
    if let Some((kind, name)) = identity {
        output.push(ParseArtifactItem {
            identity: crate::canonical_item_identity::CanonicalItemIdentityV1::new(
                "rust", kind, name,
            ),
            source_byte_start,
            source_byte_end,
        });
    }
}

pub(crate) fn with_cfg_scopes(
    mut identity: crate::canonical_item_identity::CanonicalItemIdentityV1,
    attrs: &[syn::Attribute],
) -> crate::canonical_item_identity::CanonicalItemIdentityV1 {
    for attribute in attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
    {
        if let syn::Meta::List(meta) = &attribute.meta {
            let predicate = quote::ToTokens::to_token_stream(&meta.tokens).to_string();
            identity = identity.with_scope("conditional-compilation", "cfg", predicate);
        }
    }
    identity
}

pub(crate) fn parse_owner_items_v1(source: &str) -> Result<Vec<ParseArtifactItem>, String> {
    let file = crate::parser::parse_rust_source_syntax(source)
        .map_err(|error| format!("failed to parse native owner-search source: {error}"))?;
    let mut items = Vec::new();
    collect_parse_artifact_items(source, &file.items, &mut items);
    Ok(items)
}

fn push_parse_artifact_item<T: syn::spanned::Spanned>(
    kind: &str,
    name: String,
    attrs: &[syn::Attribute],
    item: &T,
    output: &mut Vec<ParseArtifactItem>,
) {
    let identity = crate::canonical_item_identity::CanonicalItemIdentityV1::new("rust", kind, name);
    push_canonical_parse_artifact_item(with_cfg_scopes(identity, attrs), item, output);
}

fn push_canonical_parse_artifact_item<T: syn::spanned::Spanned>(
    identity: crate::canonical_item_identity::CanonicalItemIdentityV1,
    item: &T,
    output: &mut Vec<ParseArtifactItem>,
) {
    let byte_range = item.span().byte_range();
    output.push(ParseArtifactItem {
        identity,
        source_byte_start: byte_range.start,
        source_byte_end: byte_range.end,
    });
}
