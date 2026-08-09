//! Harness-owned structural-selector encoding.

use crate::canonical_item_identity::CanonicalItemIdentityV1;

/// Parses a canonical item selector into provider-owned identity fields.
pub fn parse_canonical_item_selector(
    selector: &str,
) -> Result<crate::canonical_item_identity::CanonicalItemSelectorV1, String> {
    let (language_id, locator) = selector
        .split_once("://")
        .ok_or_else(|| "canonical item selector is missing the language scheme".to_owned())?;
    let (_, identity_path) = locator.rsplit_once("#item/").ok_or_else(|| {
        "canonical item selector is missing the item identity fragment".to_owned()
    })?;
    let mut components = identity_path.split('/');
    let kind = decode_component(
        components
            .next()
            .ok_or_else(|| "canonical item selector is missing the item kind".to_owned())?,
    )?;
    let symbol = decode_component(
        components
            .next()
            .ok_or_else(|| "canonical item selector is missing the item symbol".to_owned())?,
    )?;
    if language_id.is_empty() || kind.is_empty() || symbol.is_empty() {
        return Err("canonical item selector contains an empty identity component".to_owned());
    }

    let mut identity =
        crate::canonical_item_identity::CanonicalItemIdentityV1::new(language_id, kind, symbol);
    while let Some(marker) = components.next() {
        if marker != "scope" {
            return Err(format!(
                "canonical item selector contains an unexpected identity component: {marker}"
            ));
        }
        let relation =
            decode_component(components.next().ok_or_else(|| {
                "canonical item selector scope is missing its relation".to_owned()
            })?)?;
        let kind = decode_component(
            components
                .next()
                .ok_or_else(|| "canonical item selector scope is missing its kind".to_owned())?,
        )?;
        let symbol =
            decode_component(components.next().ok_or_else(|| {
                "canonical item selector scope is missing its symbol".to_owned()
            })?)?;
        identity = identity.with_scope(relation, kind, symbol);
    }
    identity.validate()?;

    Ok(crate::canonical_item_identity::CanonicalItemSelectorV1::new(identity, selector))
}

/// Encodes a provider-owned identity as the canonical item path.
pub fn encode_canonical_item_identity_path(identity: &CanonicalItemIdentityV1) -> String {
    let mut path = format!(
        "item/{}/{}",
        encode_component(identity.kind.as_str()),
        encode_component(identity.symbol.as_str())
    );
    for scope in &identity.scopes {
        path.push_str("/scope/");
        path.push_str(&encode_component(scope.relation.as_str()));
        path.push('/');
        path.push_str(&encode_component(scope.kind.as_str()));
        path.push('/');
        path.push_str(&encode_component(scope.symbol.as_str()));
    }
    path
}

fn decode_component(component: &str) -> Result<String, String> {
    let bytes = component.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        let encoded = bytes
            .get(index + 1..index + 3)
            .ok_or_else(|| format!("invalid percent escape in selector component: {component}"))?;
        let high = decode_hex(encoded[0])
            .ok_or_else(|| format!("invalid percent escape in selector component: {component}"))?;
        let low = decode_hex(encoded[1])
            .ok_or_else(|| format!("invalid percent escape in selector component: {component}"))?;
        decoded.push((high << 4) | low);
        index += 3;
    }
    String::from_utf8(decoded)
        .map_err(|_| format!("selector component is not valid UTF-8: {component}"))
}

fn decode_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_component(component: &str) -> String {
    let mut encoded = String::with_capacity(component.len());
    for byte in component.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}
