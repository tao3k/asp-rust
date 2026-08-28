//! Schema-compatible exact-selector projection packet construction.

use serde::{Deserialize, Serialize};

use super::exact_selector_merkle::{
    ContentDigestV1, ExactProjectionModeV1, ParserLanguageIdV1, blake3_content_digest_v1,
    canonical_content_digest_v1,
};
use agent_semantic_content_identity::CanonicalItemSelector;

const SCHEMA_ID: &str = "agent.semantic-protocols.exact-selector-projection-packet";
const SCHEMA_VERSION: &str = "1";
const DIGEST_ALGORITHM: &str = "blake3-256";

macro_rules! packet_text {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

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

packet_text!(ProjectionPacketProviderIdV1);
packet_text!(ProjectionPacketOwnerPathV1);
packet_text!(ProjectionPacketStructuralSelectorV1);
packet_text!(ProjectionPacketPayloadBase64V1);

pub(crate) type ProjectionPacketLanguageIdV1 = ParserLanguageIdV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ExactSelectorProjectionEncodingV1 {
    Base64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExactSelectorProjectionPacketV1 {
    schema_id: String,
    schema_version: String,
    digest_algorithm: String,
    language_id: ProjectionPacketLanguageIdV1,
    provider_id: ProjectionPacketProviderIdV1,
    canonical_item_selector: CanonicalItemSelector,
    parser_identity_digest: ContentDigestV1,
    query_pack_digest: ContentDigestV1,
    owner_path: ProjectionPacketOwnerPathV1,
    source_blob_digest: ContentDigestV1,
    parser_fact_digest: ContentDigestV1,
    structural_selector: ProjectionPacketStructuralSelectorV1,
    projection_mode: ExactProjectionModeV1,
    projection_encoding: ExactSelectorProjectionEncodingV1,
    projection_payload_base64: ProjectionPacketPayloadBase64V1,
}

pub(crate) struct ExactSelectorProjectionPacketV1Input<'a> {
    pub(crate) language_id: &'a ProjectionPacketLanguageIdV1,
    pub(crate) provider_id: &'a ProjectionPacketProviderIdV1,
    pub(crate) canonical_item_selector: CanonicalItemSelector,
    pub(crate) parser_identity_digest: &'a ContentDigestV1,
    pub(crate) query_pack_digest: &'a ContentDigestV1,
    pub(crate) owner_path: &'a ProjectionPacketOwnerPathV1,
    pub(crate) structural_selector: &'a ProjectionPacketStructuralSelectorV1,
    pub(crate) projection_mode: ExactProjectionModeV1,
    pub(crate) source: &'a [u8],
    pub(crate) normalized_parser_facts: &'a [u8],
    pub(crate) projection: &'a [u8],
}

pub(crate) fn build_exact_selector_projection_packet_v1(
    input: ExactSelectorProjectionPacketV1Input<'_>,
) -> ExactSelectorProjectionPacketV1 {
    debug_assert!(input.canonical_item_selector.validate().is_ok());
    let source_blob_digest = blake3_content_digest_v1(input.source);
    let parser_fact_digest = canonical_content_digest_v1(
        b"asp.parser-fact.v1",
        &[
            input.language_id.as_str().as_bytes(),
            input.parser_identity_digest.as_str().as_bytes(),
            input.query_pack_digest.as_str().as_bytes(),
            source_blob_digest.as_str().as_bytes(),
            input.normalized_parser_facts,
        ],
    );
    ExactSelectorProjectionPacketV1 {
        schema_id: SCHEMA_ID.to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
        digest_algorithm: DIGEST_ALGORITHM.to_string(),
        language_id: input.language_id.clone(),
        provider_id: input.provider_id.clone(),
        canonical_item_selector: input.canonical_item_selector,
        parser_identity_digest: input.parser_identity_digest.clone(),
        query_pack_digest: input.query_pack_digest.clone(),
        owner_path: input.owner_path.clone(),
        source_blob_digest,
        parser_fact_digest,
        structural_selector: input.structural_selector.clone(),
        projection_mode: input.projection_mode,
        projection_encoding: ExactSelectorProjectionEncodingV1::Base64,
        projection_payload_base64: ProjectionPacketPayloadBase64V1::from(encode_base64(
            input.projection,
        )),
    }
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(TABLE[(first >> 2) as usize] as char);
        encoded.push(TABLE[(((first & 3) << 4) | (second >> 4)) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[(((second & 15) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(third & 63) as usize] as char
        } else {
            '='
        });
    }
    encoded
}
