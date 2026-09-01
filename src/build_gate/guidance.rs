pub(super) fn downstream_build_gate_agent_guidance(gate_label: &str) -> String {
    format!(
        "\
[asp-rust-agent-guidance]
gate: {gate_label}
trigger: cargo test runs the member build.rs before tests; keep asp-rust under [build-dependencies].
repair:
- keep build.rs thin and call assert_asp_rust_downstream_policy_with_authority.
- in a workspace, put common policy in the root harness/ module tree.
- construct AspRustWorkspacePolicy once, then derive members with member_crate or member_crate_with_config.
- add crate-local owners, receipts, waivers, or report obligations in the member override only.
- rerun cargo test after updating policy or evidence.
"
    )
}
