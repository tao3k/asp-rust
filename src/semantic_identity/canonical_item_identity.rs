//! Typed canonical item and selector identities.

use serde::{Deserialize, Serialize};

const CANONICAL_ITEM_SELECTOR_SCHEMA_ID: &str = "asp.canonical-item-selector.v1";
const CANONICAL_ITEM_SELECTOR_SCHEMA_VERSION: &str = "v1";

macro_rules! canonical_item_text {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn as_str(&self) -> &str {
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

canonical_item_text!(CanonicalItemScopeRelationV1);
canonical_item_text!(CanonicalItemScopeKindV1);
canonical_item_text!(CanonicalItemScopeSymbolV1);
canonical_item_text!(CanonicalItemLanguageIdV1);
canonical_item_text!(CanonicalItemKindV1);
canonical_item_text!(CanonicalItemSymbolV1);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CanonicalItemScopeV1 {
    pub(crate) relation: CanonicalItemScopeRelationV1,
    pub(crate) kind: CanonicalItemScopeKindV1,
    pub(crate) symbol: CanonicalItemScopeSymbolV1,
}

impl CanonicalItemScopeV1 {
    pub(crate) fn new(
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CanonicalItemIdentityV1 {
    pub(crate) language_id: CanonicalItemLanguageIdV1,
    pub(crate) kind: CanonicalItemKindV1,
    pub(crate) symbol: CanonicalItemSymbolV1,
    pub(crate) scopes: Vec<CanonicalItemScopeV1>,
}

impl CanonicalItemIdentityV1 {
    pub(crate) fn new(
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

    pub(crate) fn with_scope(
        mut self,
        relation: impl Into<CanonicalItemScopeRelationV1>,
        kind: impl Into<CanonicalItemScopeKindV1>,
        symbol: impl Into<CanonicalItemScopeSymbolV1>,
    ) -> Self {
        self.scopes
            .push(CanonicalItemScopeV1::new(relation, kind, symbol));
        self
    }

    fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("languageId", self.language_id.as_str()),
            ("kind", self.kind.as_str()),
            ("symbol", self.symbol.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("canonical item identity {field} must not be empty"));
            }
        }
        if self.scopes.iter().any(|scope| {
            [
                scope.relation.as_str(),
                scope.kind.as_str(),
                scope.symbol.as_str(),
            ]
            .iter()
            .any(|value| value.trim().is_empty())
        }) {
            return Err("canonical item identity scope fields must not be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CanonicalItemSelectorV1 {
    schema_id: String,
    schema_version: String,
    pub(crate) language_id: CanonicalItemLanguageIdV1,
    pub(crate) kind: CanonicalItemKindV1,
    pub(crate) symbol: CanonicalItemSymbolV1,
    pub(crate) scopes: Vec<CanonicalItemScopeV1>,
    pub(crate) structural_selector: String,
}

impl CanonicalItemSelectorV1 {
    pub(crate) fn new(
        identity: CanonicalItemIdentityV1,
        structural_selector: impl Into<String>,
    ) -> Self {
        Self {
            schema_id: CANONICAL_ITEM_SELECTOR_SCHEMA_ID.to_string(),
            schema_version: CANONICAL_ITEM_SELECTOR_SCHEMA_VERSION.to_string(),
            language_id: identity.language_id,
            kind: identity.kind,
            symbol: identity.symbol,
            scopes: identity.scopes,
            structural_selector: structural_selector.into(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.schema_id != CANONICAL_ITEM_SELECTOR_SCHEMA_ID
            || self.schema_version != CANONICAL_ITEM_SELECTOR_SCHEMA_VERSION
        {
            return Err("canonical item selector contract identity mismatch".to_string());
        }
        let identity = CanonicalItemIdentityV1 {
            language_id: self.language_id.clone(),
            kind: self.kind.clone(),
            symbol: self.symbol.clone(),
            scopes: self.scopes.clone(),
        };
        identity.validate()?;
        let (language_id, selector_body) = self
            .structural_selector
            .split_once("://")
            .ok_or_else(|| "canonical item selector must include <language>://".to_string())?;
        let (owner_path, identity_path) = selector_body
            .split_once('#')
            .ok_or_else(|| "canonical item selector must include an item fragment".to_string())?;
        if language_id != self.language_id.as_str() || owner_path.trim().is_empty() {
            return Err("canonical item selector owner or language mismatch".to_string());
        }
        let decoded =
            crate::semantic_identity::structural_selector::decode_canonical_item_identity_path(
                &crate::semantic_identity::structural_selector::StructuralSelectorLanguageId::from(
                    language_id,
                ),
                &crate::semantic_identity::structural_selector::CanonicalItemIdentityPath::from(
                    identity_path,
                ),
            )
            .map_err(|error| error.to_string())?;
        if decoded != identity {
            return Err("canonical item selector identity mismatch".to_string());
        }
        Ok(())
    }
}
