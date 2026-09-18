//! Stable V1 canonical item identity owned by the standalone Rust provider.

use serde::{Deserialize, Serialize};

/// Stable schema identity for canonical item selectors.
pub const CANONICAL_ITEM_SELECTOR_SCHEMA_ID: &str = "asp.canonical-item-selector.v1";
/// Stable schema version for canonical item selectors.
pub const CANONICAL_ITEM_SELECTOR_SCHEMA_VERSION: &str = "1";

macro_rules! canonical_item_text {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// Borrow the stable wire value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

canonical_item_text!(/// Relation between a scope frame and the selected item.
CanonicalItemScopeRelation);
canonical_item_text!(/// Language-neutral kind of a scope frame.
CanonicalItemScopeKind);
canonical_item_text!(/// Stable source symbol of a scope frame.
CanonicalItemScopeSymbol);
canonical_item_text!(/// Language identifier of a canonical item.
CanonicalItemLanguageId);
canonical_item_text!(/// Language-neutral kind of a canonical item.
CanonicalItemKind);
canonical_item_text!(/// Stable source symbol of a canonical item.
CanonicalItemSymbol);

/// One typed lexical or implementation scope surrounding an item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanonicalItemScope {
    /// Relation from the selected item to this scope.
    pub relation: CanonicalItemScopeRelation,
    /// Language-neutral kind of the scope.
    pub kind: CanonicalItemScopeKind,
    /// Stable source symbol of the scope.
    pub symbol: CanonicalItemScopeSymbol,
}

impl CanonicalItemScope {
    /// Construct one canonical scope frame.
    pub fn new(
        relation: impl Into<CanonicalItemScopeRelation>,
        kind: impl Into<CanonicalItemScopeKind>,
        symbol: impl Into<CanonicalItemScopeSymbol>,
    ) -> Self {
        Self {
            relation: relation.into(),
            kind: kind.into(),
            symbol: symbol.into(),
        }
    }
}

/// Typed identity of one item independent of its source owner path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanonicalItemIdentity {
    /// Language that owns the item.
    pub language_id: CanonicalItemLanguageId,
    /// Language-neutral item kind.
    pub kind: CanonicalItemKind,
    /// Stable source symbol.
    pub symbol: CanonicalItemSymbol,
    /// Ordered lexical and implementation scopes.
    pub scopes: Vec<CanonicalItemScope>,
}

impl CanonicalItemIdentity {
    /// Construct a root item identity without surrounding scopes.
    pub fn new(
        language_id: impl Into<CanonicalItemLanguageId>,
        kind: impl Into<CanonicalItemKind>,
        symbol: impl Into<CanonicalItemSymbol>,
    ) -> Self {
        Self {
            language_id: language_id.into(),
            kind: kind.into(),
            symbol: symbol.into(),
            scopes: Vec::new(),
        }
    }

    /// Append one surrounding scope in canonical order.
    pub fn with_scope(
        mut self,
        relation: impl Into<CanonicalItemScopeRelation>,
        kind: impl Into<CanonicalItemScopeKind>,
        symbol: impl Into<CanonicalItemScopeSymbol>,
    ) -> Self {
        self.scopes
            .push(CanonicalItemScope::new(relation, kind, symbol));
        self
    }

    /// Validate that every identity component carries a non-empty value.
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("languageId", self.language_id.as_str()),
            ("kind", self.kind.as_str()),
            ("symbol", self.symbol.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("canonical item identity {field} must not be empty"));
            }
        }
        for scope in &self.scopes {
            for (field, value) in [
                ("relation", scope.relation.as_str()),
                ("kind", scope.kind.as_str()),
                ("symbol", scope.symbol.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(format!(
                        "canonical item identity scope {field} must not be empty"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Stable V1 selector joining typed item identity to a structural owner path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanonicalItemSelector {
    /// Schema identity.
    pub schema_id: String,
    /// Schema version.
    pub schema_version: String,
    /// Language that owns the selected item.
    pub language_id: CanonicalItemLanguageId,
    /// Language-neutral selected item kind.
    pub kind: CanonicalItemKind,
    /// Stable selected item symbol.
    pub symbol: CanonicalItemSymbol,
    /// Ordered lexical and implementation scopes.
    pub scopes: Vec<CanonicalItemScope>,
    /// Canonical structural selector rendered by the provider.
    pub structural_selector: String,
}

impl CanonicalItemSelector {
    /// Construct a selector from typed identity and its structural location.
    pub fn new(identity: CanonicalItemIdentity, structural_selector: impl Into<String>) -> Self {
        Self {
            schema_id: CANONICAL_ITEM_SELECTOR_SCHEMA_ID.to_owned(),
            schema_version: CANONICAL_ITEM_SELECTOR_SCHEMA_VERSION.to_owned(),
            language_id: identity.language_id,
            kind: identity.kind,
            symbol: identity.symbol,
            scopes: identity.scopes,
            structural_selector: structural_selector.into(),
        }
    }

    /// Recover the typed item identity carried by this selector.
    pub fn identity(&self) -> CanonicalItemIdentity {
        CanonicalItemIdentity {
            language_id: self.language_id.clone(),
            kind: self.kind.clone(),
            symbol: self.symbol.clone(),
            scopes: self.scopes.clone(),
        }
    }
}
