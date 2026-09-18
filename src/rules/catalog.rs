//! Rule-pack catalog metadata.

use crate::RulePackDescriptor;

/// Return stable metadata for the default ASP Rust rule packs.
#[must_use]
pub fn rust_rule_pack_descriptors() -> Vec<RulePackDescriptor> {
    vec![
        RulePackDescriptor {
            id: "rust.syntax",
            version: "1",
            domains: &["rust", "syntax"],
            default_mode: "blocking",
        },
        RulePackDescriptor {
            id: "rust.project_policy",
            version: "1",
            domains: &["rust", "project-policy", "tests"],
            default_mode: "blocking",
        },
        RulePackDescriptor {
            id: "rust.modularity",
            version: "1",
            domains: &["rust", "modularity"],
            default_mode: "blocking",
        },
        RulePackDescriptor {
            id: "rust.agent_policy",
            version: "1",
            domains: &["rust", "agent-policy"],
            default_mode: "advisory",
        },
    ]
}
