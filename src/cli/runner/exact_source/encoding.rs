//! Exact-source transport encoding helpers.

pub(crate) fn decode_canonical_base64(input: &[u8]) -> Option<Vec<u8>> {
    use base64::Engine as _;

    base64::engine::general_purpose::STANDARD.decode(input).ok()
}
