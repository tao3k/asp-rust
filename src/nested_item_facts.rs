//! Provider-owned Rust item facts for nested declarations and exact source projection.

use std::ops::Range;

use proc_macro2::Span;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::canonical_item_identity::{
    CanonicalItemKindV1, CanonicalItemScopeKindV1, CanonicalItemScopeSymbolV1,
    CanonicalItemSymbolV1,
};

/// Schema identifier for provider-native nested Rust item facts.
pub const RUST_NESTED_ITEM_FACT_SCHEMA_ID: &str = "agent.semantic-protocols.rust-nested-item-fact";
/// Schema version for provider-native nested Rust item facts.
pub const RUST_NESTED_ITEM_FACT_SCHEMA_VERSION: &str = "1";

macro_rules! nested_text_identity {
    ($name:ident, $documentation:literal) => {
        #[doc = $documentation]
        #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = "Returns the provider-owned text value."]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}

nested_text_identity!(
    RustNestedItemSchemaIdV1,
    "Typed schema identifier for a nested-item fact."
);
nested_text_identity!(
    RustNestedItemSchemaVersionV1,
    "Typed schema version for a nested-item fact."
);
nested_text_identity!(
    RustNestedItemOwnerPathV1,
    "Typed workspace-relative owner path for a nested-item fact."
);
nested_text_identity!(
    RustNestedItemDigestV1,
    "Typed digest carried by a nested-item fact."
);

/// Lexical or ownership scope attached to one nested Rust item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustItemScopeFactV1 {
    /// Provider-native scope kind.
    pub kind: CanonicalItemScopeKindV1,
    /// Scope symbol as emitted by the Rust parser.
    pub symbol: CanonicalItemScopeSymbolV1,
}

/// Provider-native fact for one top-level or nested Rust item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustNestedItemFactV1 {
    /// Wire schema identifier.
    pub schema_id: RustNestedItemSchemaIdV1,
    /// Wire schema version.
    pub schema_version: RustNestedItemSchemaVersionV1,
    /// Workspace-relative source owner.
    pub owner_path: RustNestedItemOwnerPathV1,
    /// Provider-native Rust item kind.
    pub item_kind: CanonicalItemKindV1,
    /// Item symbol.
    pub symbol: CanonicalItemSymbolV1,
    /// Ordered lexical and ownership scopes.
    pub scopes: Vec<RustItemScopeFactV1>,
    /// Implementation owner when the item is nested in an `impl`.
    pub impl_owner: Option<CanonicalItemScopeSymbolV1>,
    /// Trait owner when the item is declared in a trait or trait implementation.
    pub trait_owner: Option<CanonicalItemScopeSymbolV1>,
    /// Inclusive source byte start.
    pub source_byte_start: usize,
    /// Exclusive source byte end.
    pub source_byte_end: usize,
    /// Stable identity digest.
    pub identity_digest: RustNestedItemDigestV1,
    /// Digest of the exact projected source slice.
    pub source_slice_digest: RustNestedItemDigestV1,
}

/// Parses one Rust owner and emits deterministically ordered nested-item facts.
pub fn rust_nested_item_facts_v1(
    owner_path: &str,
    source: &str,
) -> Result<Vec<RustNestedItemFactV1>, String> {
    if owner_path.is_empty() {
        return Err("Rust nested-item facts require an owner path".to_string());
    }
    let file = crate::parser::parse_rust_source_syntax(source)
        .map_err(|error| format!("failed to parse Rust owner: {error}"))?;
    let mut visitor = RustNestedItemFactVisitorV1 {
        owner_path,
        source,
        scopes: Vec::new(),
        impl_owner: None,
        trait_owner: None,
        facts: Vec::new(),
    };
    visitor.visit_file(&file);
    visitor.facts.sort_unstable_by(|left, right| {
        left.source_byte_start
            .cmp(&right.source_byte_start)
            .then_with(|| left.source_byte_end.cmp(&right.source_byte_end))
            .then_with(|| left.item_kind.cmp(&right.item_kind))
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    Ok(visitor.facts)
}

/// Projects and verifies the exact source slice referenced by a nested-item fact.
pub fn project_rust_nested_item_code_v1<'source>(
    source: &'source str,
    fact: &RustNestedItemFactV1,
) -> Result<&'source str, String> {
    if fact.schema_id.as_str() != RUST_NESTED_ITEM_FACT_SCHEMA_ID
        || fact.schema_version.as_str() != RUST_NESTED_ITEM_FACT_SCHEMA_VERSION
    {
        return Err("Rust nested-item fact contract mismatch".to_string());
    }
    let projection = source
        .get(fact.source_byte_start..fact.source_byte_end)
        .ok_or_else(|| {
            "Rust nested-item projection range is outside the owner source".to_string()
        })?;
    if sha256_hex(projection.as_bytes()) != fact.source_slice_digest.as_str() {
        return Err("Rust nested-item projection digest mismatch".to_string());
    }
    Ok(projection)
}

struct RustNestedItemFactVisitorV1<'source> {
    owner_path: &'source str,
    source: &'source str,
    scopes: Vec<RustItemScopeFactV1>,
    impl_owner: Option<String>,
    trait_owner: Option<String>,
    facts: Vec<RustNestedItemFactV1>,
}

impl RustNestedItemFactVisitorV1<'_> {
    fn record(
        &mut self,
        item_kind: &str,
        symbol: String,
        span: Span,
        attributes: &[syn::Attribute],
    ) {
        if symbol.is_empty() {
            return;
        }
        let range = source_range(span, attributes);
        let Some(source_slice) = self.source.get(range.clone()) else {
            return;
        };
        let identity_digest = nested_item_identity_digest(
            self.owner_path,
            item_kind,
            &symbol,
            &self.scopes,
            self.impl_owner.as_deref(),
            self.trait_owner.as_deref(),
        );
        self.facts.push(RustNestedItemFactV1 {
            schema_id: RUST_NESTED_ITEM_FACT_SCHEMA_ID.into(),
            schema_version: RUST_NESTED_ITEM_FACT_SCHEMA_VERSION.into(),
            owner_path: self.owner_path.into(),
            item_kind: item_kind.into(),
            symbol: symbol.into(),
            scopes: self.scopes.clone(),
            impl_owner: self.impl_owner.clone().map(Into::into),
            trait_owner: self.trait_owner.clone().map(Into::into),
            source_byte_start: range.start,
            source_byte_end: range.end,
            identity_digest: identity_digest.into(),
            source_slice_digest: sha256_hex(source_slice.as_bytes()).into(),
        });
    }

    fn enter_scope(&mut self, kind: &str, symbol: String) {
        self.scopes.push(RustItemScopeFactV1 {
            kind: kind.into(),
            symbol: symbol.into(),
        });
    }
}

impl<'ast> Visit<'ast> for RustNestedItemFactVisitorV1<'_> {
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.record("const", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_item_const(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        let symbol = item.ident.to_string();
        self.record("enum", symbol.clone(), item.span(), &item.attrs);
        self.enter_scope("enum", symbol);
        visit::visit_item_enum(self, item);
        self.scopes.pop();
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        self.record(
            "extern-crate",
            item.ident.to_string(),
            item.span(),
            &item.attrs,
        );
        visit::visit_item_extern_crate(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let symbol = item.sig.ident.to_string();
        self.record("function", symbol.clone(), item.span(), &item.attrs);
        self.enter_scope("function", symbol);
        visit::visit_item_fn(self, item);
        self.scopes.pop();
    }

    fn visit_item_foreign_mod(&mut self, item: &'ast syn::ItemForeignMod) {
        let symbol = item.abi.to_token_stream().to_string();
        self.record("foreign-module", symbol.clone(), item.span(), &item.attrs);
        self.enter_scope("foreign-module", symbol);
        visit::visit_item_foreign_mod(self, item);
        self.scopes.pop();
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let impl_owner = item.self_ty.to_token_stream().to_string();
        let trait_owner = item
            .trait_
            .as_ref()
            .map(|(_, path, _)| path.to_token_stream().to_string());
        let previous_impl_owner = self.impl_owner.replace(impl_owner.clone());
        let previous_trait_owner = std::mem::replace(&mut self.trait_owner, trait_owner);
        self.record("impl", impl_owner.clone(), item.span(), &item.attrs);
        self.enter_scope("impl", impl_owner);
        visit::visit_item_impl(self, item);
        self.scopes.pop();
        self.impl_owner = previous_impl_owner;
        self.trait_owner = previous_trait_owner;
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        let symbol = item
            .ident
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| item.mac.path.to_token_stream().to_string());
        self.record("macro", symbol, item.span(), &item.attrs);
        visit::visit_item_macro(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let symbol = item.ident.to_string();
        self.record("module", symbol.clone(), item.span(), &item.attrs);
        self.enter_scope("module", symbol);
        visit::visit_item_mod(self, item);
        self.scopes.pop();
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.record("static", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_item_static(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.record("struct", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_item_struct(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        let symbol = item.ident.to_string();
        self.record("trait", symbol.clone(), item.span(), &item.attrs);
        let previous_trait_owner = self.trait_owner.replace(symbol.clone());
        self.enter_scope("trait", symbol);
        visit::visit_item_trait(self, item);
        self.scopes.pop();
        self.trait_owner = previous_trait_owner;
    }

    fn visit_item_trait_alias(&mut self, item: &'ast syn::ItemTraitAlias) {
        self.record(
            "trait-alias",
            item.ident.to_string(),
            item.span(),
            &item.attrs,
        );
        visit::visit_item_trait_alias(self, item);
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.record("type", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_item_type(self, item);
    }

    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.record("union", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_item_union(self, item);
    }

    fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
        self.record("const", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_impl_item_const(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let symbol = item.sig.ident.to_string();
        self.record("method", symbol.clone(), item.span(), &item.attrs);
        self.enter_scope("method", symbol);
        visit::visit_impl_item_fn(self, item);
        self.scopes.pop();
    }

    fn visit_impl_item_macro(&mut self, item: &'ast syn::ImplItemMacro) {
        self.record(
            "macro",
            item.mac.path.to_token_stream().to_string(),
            item.span(),
            &item.attrs,
        );
        visit::visit_impl_item_macro(self, item);
    }

    fn visit_impl_item_type(&mut self, item: &'ast syn::ImplItemType) {
        self.record("type", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_impl_item_type(self, item);
    }

    fn visit_trait_item_const(&mut self, item: &'ast syn::TraitItemConst) {
        self.record("const", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_trait_item_const(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.record(
            "method",
            item.sig.ident.to_string(),
            item.span(),
            &item.attrs,
        );
        visit::visit_trait_item_fn(self, item);
    }

    fn visit_trait_item_macro(&mut self, item: &'ast syn::TraitItemMacro) {
        self.record(
            "macro",
            item.mac.path.to_token_stream().to_string(),
            item.span(),
            &item.attrs,
        );
        visit::visit_trait_item_macro(self, item);
    }

    fn visit_trait_item_type(&mut self, item: &'ast syn::TraitItemType) {
        self.record("type", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_trait_item_type(self, item);
    }

    fn visit_foreign_item_fn(&mut self, item: &'ast syn::ForeignItemFn) {
        self.record(
            "function",
            item.sig.ident.to_string(),
            item.span(),
            &item.attrs,
        );
        visit::visit_foreign_item_fn(self, item);
    }

    fn visit_foreign_item_macro(&mut self, item: &'ast syn::ForeignItemMacro) {
        self.record(
            "macro",
            item.mac.path.to_token_stream().to_string(),
            item.span(),
            &item.attrs,
        );
        visit::visit_foreign_item_macro(self, item);
    }

    fn visit_foreign_item_static(&mut self, item: &'ast syn::ForeignItemStatic) {
        self.record("static", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_foreign_item_static(self, item);
    }

    fn visit_foreign_item_type(&mut self, item: &'ast syn::ForeignItemType) {
        self.record("type", item.ident.to_string(), item.span(), &item.attrs);
        visit::visit_foreign_item_type(self, item);
    }
}

fn source_range(span: Span, attributes: &[syn::Attribute]) -> Range<usize> {
    let mut range = span.byte_range();
    for attribute in attributes {
        let attribute_range = attribute.span().byte_range();
        range.start = range.start.min(attribute_range.start);
        range.end = range.end.max(attribute_range.end);
    }
    range
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn nested_item_identity_digest(
    owner_path: &str,
    item_kind: &str,
    symbol: &str,
    scopes: &[RustItemScopeFactV1],
    impl_owner: Option<&str>,
    trait_owner: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    update_identity_field(&mut hasher, owner_path.as_bytes());
    update_identity_field(&mut hasher, item_kind.as_bytes());
    update_identity_field(&mut hasher, symbol.as_bytes());
    for scope in scopes {
        update_identity_field(&mut hasher, scope.kind.as_str().as_bytes());
        update_identity_field(&mut hasher, scope.symbol.as_str().as_bytes());
    }
    update_identity_field(&mut hasher, impl_owner.unwrap_or_default().as_bytes());
    update_identity_field(&mut hasher, trait_owner.unwrap_or_default().as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn update_identity_field(hasher: &mut Sha256, field: &[u8]) {
    hasher.update((field.len() as u64).to_le_bytes());
    hasher.update(field);
}
