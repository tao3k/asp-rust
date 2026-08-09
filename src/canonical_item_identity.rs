//! Harness-owned canonical Rust item identity carried by shared v1 wire contracts.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! string_identity {
    ($name:ident) => {
        #[doc = concat!("Provider-owned string identity for `", stringify!($name), "`.")]
        #[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Returns the identity value as a string slice.
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

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

string_identity!(CanonicalItemLanguageIdV1);
string_identity!(CanonicalItemKindV1);
string_identity!(CanonicalItemSymbolV1);
string_identity!(CanonicalItemScopeRelationV1);
string_identity!(CanonicalItemScopeKindV1);
string_identity!(CanonicalItemScopeSymbolV1);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
/// One canonical owner or conditional scope attached to an item identity.
pub struct CanonicalItemScopeV1 {
    /// Relationship between the item and this scope.
    pub relation: CanonicalItemScopeRelationV1,
    /// Provider-native kind of the scope owner.
    pub kind: CanonicalItemScopeKindV1,
    /// Provider-native symbol of the scope owner.
    pub symbol: CanonicalItemScopeSymbolV1,
}

impl CanonicalItemScopeV1 {
    /// Creates a canonical scope from provider-native components.
    pub fn new(
        relation: impl Into<CanonicalItemScopeRelationV1>,
        kind: impl Into<CanonicalItemScopeKindV1>,
        symbol: impl Into<CanonicalItemScopeSymbolV1>,
    ) -> Self {
        Self {
            relation: relation.into(),
            kind: kind.into(),
            symbol: symbol.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
/// Canonical provider-owned identity for one Rust item.
pub struct CanonicalItemIdentityV1 {
    /// Language provider that owns the identity.
    pub language_id: CanonicalItemLanguageIdV1,
    /// Rust item kind.
    pub kind: CanonicalItemKindV1,
    /// Rust item symbol.
    pub symbol: CanonicalItemSymbolV1,
    /// Ordered owner and conditional scopes.
    #[serde(default)]
    pub scopes: Vec<CanonicalItemScopeV1>,
}

impl CanonicalItemIdentityV1 {
    /// Creates an unscoped canonical item identity.
    pub fn new(
        language_id: impl Into<CanonicalItemLanguageIdV1>,
        kind: impl Into<CanonicalItemKindV1>,
        symbol: impl Into<CanonicalItemSymbolV1>,
    ) -> Self {
        Self {
            language_id: language_id.into(),
            kind: kind.into(),
            symbol: symbol.into(),
            scopes: Vec::new(),
        }
    }

    /// Appends an ordered owner or conditional scope.
    pub fn with_scope(
        mut self,
        relation: impl Into<CanonicalItemScopeRelationV1>,
        kind: impl Into<CanonicalItemScopeKindV1>,
        symbol: impl Into<CanonicalItemScopeSymbolV1>,
    ) -> Self {
        self.scopes
            .push(CanonicalItemScopeV1::new(relation, kind, symbol));
        self
    }

    /// Validates that all required identity components are non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.language_id.as_str().is_empty()
            || self.kind.as_str().is_empty()
            || self.symbol.as_str().is_empty()
        {
            return Err("canonical item identity fields must be non-empty".to_owned());
        }
        if self.scopes.iter().any(|scope| {
            scope.relation.as_str().is_empty()
                || scope.kind.as_str().is_empty()
                || scope.symbol.as_str().is_empty()
        }) {
            return Err("canonical item scope fields must be non-empty".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
/// Canonical identity paired with its structural selector wire value.
pub struct CanonicalItemSelectorV1 {
    /// Shared canonical selector schema identity.
    pub schema_id: String,
    /// Shared canonical selector schema version.
    pub schema_version: String,
    /// Language provider that owns the identity.
    pub language_id: CanonicalItemLanguageIdV1,
    /// Provider-owned item kind.
    pub kind: CanonicalItemKindV1,
    /// Provider-owned item symbol.
    pub symbol: CanonicalItemSymbolV1,
    /// Ordered provider-owned scope frames.
    pub scopes: Vec<CanonicalItemScopeV1>,
    /// Original structural selector.
    pub structural_selector: String,
}

impl CanonicalItemSelectorV1 {
    /// Creates a selector from its parsed identity and wire value.
    pub fn new(identity: CanonicalItemIdentityV1, structural_selector: impl Into<String>) -> Self {
        let CanonicalItemIdentityV1 {
            language_id,
            kind,
            symbol,
            scopes,
        } = identity;
        Self {
            schema_id: "asp.canonical-item-selector.v1".to_owned(),
            schema_version: "1".to_owned(),
            language_id,
            kind,
            symbol,
            scopes,
            structural_selector: structural_selector.into(),
        }
    }

    /// Returns the original structural selector.
    pub fn structural_selector(&self) -> &str {
        &self.structural_selector
    }

    /// Reconstructs the provider-owned identity from the canonical wire packet.
    pub fn identity(&self) -> CanonicalItemIdentityV1 {
        CanonicalItemIdentityV1 {
            language_id: self.language_id.clone(),
            kind: self.kind.clone(),
            symbol: self.symbol.clone(),
            scopes: self.scopes.clone(),
        }
    }
}
