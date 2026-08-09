pub(crate) fn decode_canonical_base64(encoded: &[u8]) -> Option<Vec<u8>> {
    if !encoded.len().is_multiple_of(4) {
        return None;
    }
    let mut decoded = Vec::with_capacity(encoded.len() / 4 * 3);
    let chunk_count = encoded.len() / 4;
    for (index, chunk) in encoded.chunks_exact(4).enumerate() {
        let last = index + 1 == chunk_count;
        let first = base64_value(chunk[0])?;
        let second = base64_value(chunk[1])?;
        decoded.push((first << 2) | (second >> 4));
        if chunk[2] == b'=' {
            if !last || chunk[3] != b'=' || second & 0x0f != 0 {
                return None;
            }
            continue;
        }
        let third = base64_value(chunk[2])?;
        decoded.push((second << 4) | (third >> 2));
        if chunk[3] == b'=' {
            if !last || third & 0x03 != 0 {
                return None;
            }
            continue;
        }
        decoded.push((third << 6) | base64_value(chunk[3])?);
    }
    Some(decoded)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
