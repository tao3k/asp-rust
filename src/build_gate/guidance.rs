pub(super) fn downstream_build_gate_agent_guidance(gate_label: &str) -> String {
    format!(
        "\
[asp-rust-agent-guidance]
gate: {gate_label}
trigger: cargo test runs the shared Build Support dependency before tests.
repair:
- keep member crates free of policy-only build scripts.
- declare one shared Build Support build-dependency across governed packages.
- put common policy in the shared workspace policy package.
- construct AspRustWorkspacePolicy once, then derive members with member_crate or member_crate_with_config.
- add crate-local owners, receipts, waivers, or report obligations in the member override only.
- rerun cargo test after updating policy or evidence.
"
    )
}
