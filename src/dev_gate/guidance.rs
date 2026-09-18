pub(super) fn downstream_dev_gate_agent_guidance(gate_label: &str) -> String {
    format!(
        "\
[asp-rust-agent-guidance]
gate: {gate_label}
trigger: cargo test runs the shared Build Support policy gate.
repair:
- keep member crates free of policy-only build scripts.
- declare the shared Build Support crate under [dev-dependencies].
- put common policy in the shared workspace policy package.
- mount its policy macro from a thin Cargo test target.
- construct AspRustWorkspacePolicy once, then derive members with member_crate or member_crate_with_config.
- add crate-local owners, receipts, waivers, or report obligations in the member override only.
- rerun cargo test after updating policy or evidence.
"
    )
}
