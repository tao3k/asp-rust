//! Typed digest and projection-mode values used by exact-selector packets.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ParserLanguageIdV1(String);

impl ParserLanguageIdV1 {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ParserLanguageIdV1 {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ParserLanguageIdV1 {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ContentDigestV1(String);

impl ContentDigestV1 {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn parse_content_digest_v1(value: &str) -> Result<ContentDigestV1, String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("content digest must be 64 hexadecimal characters".to_string());
    }
    Ok(ContentDigestV1(value.to_ascii_lowercase()))
}

pub(crate) fn blake3_content_digest_v1(bytes: &[u8]) -> ContentDigestV1 {
    ContentDigestV1(blake3::hash(bytes).to_hex().to_string())
}

pub(crate) fn canonical_content_digest_v1(domain: &[u8], parts: &[&[u8]]) -> ContentDigestV1 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(&(part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    ContentDigestV1(hasher.finalize().to_hex().to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ExactProjectionModeV1 {
    Code,
    Names,
}
