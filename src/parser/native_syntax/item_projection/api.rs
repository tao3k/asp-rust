#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustItemProjectionNodeSyntax {
    pub line: usize,
    pub end_line: usize,
    pub kind: &'static str,
    pub role: &'static str,
    pub label: String,
    pub depth: usize,
    pub canonical_item_identity: Option<crate::content_identity::CanonicalItemIdentity>,
}

pub(in crate::parser::native_syntax) fn item_projection_nodes(
    item: &syn::Item,
) -> Vec<RustItemProjectionNodeSyntax> {
    super::node_walk::item_projection_nodes(item)
}
