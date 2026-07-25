use super::model::ParseArtifactItem;

pub(super) fn collect_parse_artifact_items(items: &[syn::Item]) -> Vec<ParseArtifactItem> {
    let mut output = Vec::new();
    collect_items(items, &mut output);
    output
}

fn collect_items(items: &[syn::Item], output: &mut Vec<ParseArtifactItem>) {
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
            syn::Item::Macro(item) => collect_macro_item(item, output),
            syn::Item::Mod(item) => collect_module_items(item, output),
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
            syn::Item::Trait(item) => collect_trait_items(item, output),
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
            syn::Item::Impl(item) => collect_impl_items(item, output),
            syn::Item::Use(item) if !matches!(item.vis, syn::Visibility::Inherited) => {
                let span = syn::spanned::Spanned::span(item);
                collect_reexport_items(
                    &item.tree,
                    span.start().line.max(1),
                    span.end().line.max(span.start().line.max(1)),
                    output,
                );
            }
            _ => {}
        }
    }
}

fn collect_macro_item(item: &syn::ItemMacro, output: &mut Vec<ParseArtifactItem>) {
    let Some(ident) = item.ident.as_ref() else {
        return;
    };
    push_parse_artifact_item("macro", ident.to_string(), &item.attrs, item, output);
}

fn collect_module_items(item: &syn::ItemMod, output: &mut Vec<ParseArtifactItem>) {
    push_parse_artifact_item("module", item.ident.to_string(), &item.attrs, item, output);
    let Some((_, nested)) = item.content.as_ref() else {
        return;
    };
    collect_items(nested, output);
}

fn collect_impl_items(item: &syn::ItemImpl, output: &mut Vec<ParseArtifactItem>) {
    let impl_owner = quote::ToTokens::to_token_stream(item.self_ty.as_ref())
        .to_string()
        .replace(' ', "");
    let trait_owner = item.trait_.as_ref().map(|(_, path, _)| {
        quote::ToTokens::to_token_stream(path)
            .to_string()
            .replace(' ', "")
    });
    let mut impl_identity =
        agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
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
        let mut method_identity =
            agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
                "rust",
                "method",
                method.sig.ident.to_string(),
            )
            .with_scope("implementation-owner", "type", impl_owner.clone());
        if let Some(trait_owner) = trait_owner.as_deref() {
            method_identity = method_identity.with_scope("trait-owner", "trait", trait_owner);
        }
        push_canonical_parse_artifact_item(
            with_cfg_scopes(method_identity, &method.attrs),
            method,
            output,
        );
    }
}

fn collect_trait_items(item: &syn::ItemTrait, output: &mut Vec<ParseArtifactItem>) {
    let trait_owner = item.ident.to_string();
    push_parse_artifact_item("trait", trait_owner.clone(), &item.attrs, item, output);
    for trait_item in &item.items {
        if let syn::TraitItem::Fn(method) = trait_item {
            let identity = with_cfg_scopes(
                agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
                    "rust",
                    "trait-function",
                    method.sig.ident.to_string(),
                )
                .with_scope("trait-owner", "trait", trait_owner.clone()),
                &method.attrs,
            );
            push_canonical_parse_artifact_item(identity, method, output);
        }
    }
}

fn collect_reexport_items(
    tree: &syn::UseTree,
    start_line: usize,
    end_line: usize,
    output: &mut Vec<ParseArtifactItem>,
) {
    match tree {
        syn::UseTree::Name(name) => output.push(ParseArtifactItem {
            identity: agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
                "rust",
                "reexport",
                name.ident.to_string(),
            ),
            start_line,
            end_line,
        }),
        syn::UseTree::Rename(rename) => output.push(ParseArtifactItem {
            identity: agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
                "rust",
                "reexport",
                rename.rename.to_string(),
            ),
            start_line,
            end_line,
        }),
        syn::UseTree::Path(path) => {
            collect_reexport_items(&path.tree, start_line, end_line, output);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_reexport_items(item, start_line, end_line, output);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn with_cfg_scopes(
    mut identity: agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1,
    attrs: &[syn::Attribute],
) -> agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1 {
    for attribute in attrs {
        if !attribute.path().is_ident("cfg") {
            continue;
        }
        let syn::Meta::List(meta) = &attribute.meta else {
            continue;
        };
        let predicate = quote::ToTokens::to_token_stream(&meta.tokens).to_string();
        identity = identity.with_scope("conditional-compilation", "cfg", predicate);
    }
    identity
}

fn push_parse_artifact_item<T: syn::spanned::Spanned>(
    kind: &str,
    name: String,
    attrs: &[syn::Attribute],
    item: &T,
    output: &mut Vec<ParseArtifactItem>,
) {
    let identity = with_cfg_scopes(
        agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1::new(
            "rust", kind, name,
        ),
        attrs,
    );
    push_canonical_parse_artifact_item(identity, item, output);
}

fn push_canonical_parse_artifact_item<T: syn::spanned::Spanned>(
    identity: agent_semantic_content_identity::canonical_item_identity::CanonicalItemIdentityV1,
    item: &T,
    output: &mut Vec<ParseArtifactItem>,
) {
    let span = item.span();
    let start_line = span.start().line.max(1);
    output.push(ParseArtifactItem {
        identity,
        start_line,
        end_line: span.end().line.max(start_line),
    });
}
