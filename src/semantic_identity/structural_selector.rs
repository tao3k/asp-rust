//! Canonical structural-selector component and identity-path codec.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use super::canonical_item_identity::{CanonicalItemIdentityV1, CanonicalItemScopeV1};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StructuralSelectorCodecError(String);

impl StructuralSelectorCodecError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl Display for StructuralSelectorCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for StructuralSelectorCodecError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StructuralSelectorLanguageId(String);

impl StructuralSelectorLanguageId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for StructuralSelectorLanguageId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CanonicalItemIdentityPath(String);

impl CanonicalItemIdentityPath {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CanonicalItemIdentityPath {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

pub(crate) fn encode_canonical_item_identity_path(identity: &CanonicalItemIdentityV1) -> String {
    let mut encoded = format!(
        "item/{}/{}",
        encode_component(identity.kind.as_str()),
        encode_component(identity.symbol.as_str())
    );
    for scope in &identity.scopes {
        encoded.push_str("/scope/");
        encoded.push_str(&encode_component(scope.relation.as_str()));
        encoded.push('/');
        encoded.push_str(&encode_component(scope.kind.as_str()));
        encoded.push('/');
        encoded.push_str(&encode_component(scope.symbol.as_str()));
    }
    encoded
}

pub(crate) fn decode_canonical_item_identity_path(
    language_id: &StructuralSelectorLanguageId,
    encoded: &CanonicalItemIdentityPath,
) -> Result<CanonicalItemIdentityV1, StructuralSelectorCodecError> {
    let segments = encoded.as_str().split('/').collect::<Vec<_>>();
    if segments.len() < 3 || segments[0] != "item" || (segments.len() - 3) % 4 != 0 {
        return Err(StructuralSelectorCodecError::new(
            "canonical item identity path shape is invalid",
        ));
    }
    let mut identity = CanonicalItemIdentityV1::new(
        language_id.as_str(),
        decode_component(segments[1])?,
        decode_component(segments[2])?,
    );
    for scope in segments[3..].chunks_exact(4) {
        if scope[0] != "scope" {
            return Err(StructuralSelectorCodecError::new(
                "canonical item identity scope marker is invalid",
            ));
        }
        identity.scopes.push(CanonicalItemScopeV1::new(
            decode_component(scope[1])?,
            decode_component(scope[2])?,
            decode_component(scope[3])?,
        ));
    }
    if encode_canonical_item_identity_path(&identity) != encoded.as_str() {
        return Err(StructuralSelectorCodecError::new(
            "canonical item identity path is not canonical",
        ));
    }
    Ok(identity)
}

fn encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(
                char::from_digit(u32::from(byte >> 4), 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            encoded.push(
                char::from_digit(u32::from(byte & 0x0f), 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
        }
    }
    encoded
}

fn decode_component(value: &str) -> Result<String, StructuralSelectorCodecError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                    .map_err(|_| StructuralSelectorCodecError::new("invalid percent escape"))?;
                if hex.bytes().any(|byte| byte.is_ascii_lowercase()) {
                    return Err(StructuralSelectorCodecError::new(
                        "percent escapes must use uppercase hex",
                    ));
                }
                decoded
                    .push(u8::from_str_radix(hex, 16).map_err(|_| {
                        StructuralSelectorCodecError::new("invalid percent escape")
                    })?);
                index += 3;
            }
            byte if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') => {
                decoded.push(byte);
                index += 1;
            }
            _ => {
                return Err(StructuralSelectorCodecError::new(
                    "non-canonical selector component",
                ));
            }
        }
    }
    String::from_utf8(decoded)
        .map_err(|_| StructuralSelectorCodecError::new("selector component is not UTF-8"))
}
