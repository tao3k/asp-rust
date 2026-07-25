#[derive(Clone, Debug)]
pub(super) struct ParseArtifactItem {
    pub(super) identity: crate::semantic_identity::canonical_item_identity::CanonicalItemIdentityV1,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
}
