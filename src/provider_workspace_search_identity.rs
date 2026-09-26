//! Harness-owned workspace-search identity primitives.

use std::collections::BTreeMap;

const HASH_ALGORITHM: &[u8] = b"blake3";
const LEAF_DOMAIN: &[u8] = b"asp.leaf.v1";
const LEAF_KIND: &[u8] = b"text";
const LEAF_MEDIA_TYPE: &[u8] = b"application/vnd.asp.source-content-digest";
const NODE_DOMAIN: &[u8] = b"asp.node.v1";
const SNAPSHOT_KIND: &[u8] = b"sourceSnapshot";
const SNAPSHOT_SCHEMA_ID: &[u8] = b"asp.source-snapshot.v1";
const SNAPSHOT_SCHEMA_VERSION: &[u8] = b"1";
const SOURCE_CHILD_ROLE: &[u8] = b"source";

#[derive(Debug, Clone, PartialEq, Eq)]
/// Provider-owned source snapshot root used to validate complete owner coverage.
pub struct WorkspaceSnapshot {
    root_digest: String,
}

impl WorkspaceSnapshot {
    /// Reconstructs a deterministic snapshot root from owner paths and content digests.
    pub fn from_file_hashes<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let files = files
            .into_iter()
            .map(|(path, digest)| (normalize_snapshot_path(path), digest))
            .collect::<BTreeMap<_, _>>();
        let mut hasher = blake3::Hasher::new();

        for part in [
            NODE_DOMAIN,
            SNAPSHOT_KIND,
            SNAPSHOT_SCHEMA_ID,
            SNAPSHOT_SCHEMA_VERSION,
            b"none",
            b"none",
            b"none",
        ] {
            update_part(&mut hasher, part);
        }

        for (index, (path, digest)) in files.into_iter().enumerate() {
            let leaf_digest = source_digest_leaf(digest);
            for part in [
                SOURCE_CHILD_ROLE,
                path.as_bytes(),
                HASH_ALGORITHM,
                leaf_digest.as_bytes(),
                &(index as u64).to_be_bytes(),
            ] {
                update_part(&mut hasher, part);
            }
        }

        Self {
            root_digest: hasher.finalize().to_hex().to_string(),
        }
    }

    #[must_use]
    /// Returns the lowercase BLAKE3 root digest.
    pub fn root_digest(&self) -> &str {
        &self.root_digest
    }
}

fn normalize_snapshot_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let components = normalized
        .split('/')
        .fold(Vec::new(), |mut components, component| {
            match component {
                "" | "." => {}
                ".." => {
                    components.pop();
                }
                component => components.push(component),
            }
            components
        });
    components.join("/")
}

fn source_digest_leaf(digest: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    for part in [LEAF_DOMAIN, LEAF_KIND, LEAF_MEDIA_TYPE, digest.as_bytes()] {
        update_part(&mut hasher, part);
    }
    hasher.finalize().to_hex().to_string()
}

fn update_part(hasher: &mut blake3::Hasher, part: &[u8]) {
    hasher.update(&(part.len() as u64).to_be_bytes());
    hasher.update(part);
}
