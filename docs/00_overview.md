# Overview

`asp-rust` exists to reduce agent search cost in long-lived
Rust projects. It parses a project through native Rust syntax and Cargo
manifest facts, derives package/module/owner/dependency structure, and renders
small repair contracts that an agent can act on without inspecting every file.

The harness does not replace Rust's normal toolchain. `rustc` owns compilation,
`rustfmt` owns formatting, Clippy owns linting, and this crate owns
project-shape feedback for repair loops: scope drift, oversized owners, facade
entrypoints that start carrying implementation, hidden imports, primitive public
API surfaces, and forgotten verification obligations.

## Agent Contract

Compact text is the default because agents are the main consumer. Findings avoid
human audit boilerplate and render the fields needed for repair: stable rule id,
location, fix direction, optional source line, help text, and contract. JSON is
still available for structured tools, and `--agent-snapshot` exposes a
low-noise project reasoning tree when the agent needs orientation instead of a
raw file inventory.

The expected loop is:

1. a downstream crate mounts the build-time harness gate;
2. `cargo check` runs parser-native project policy before runtime tests;
3. compact findings report structural drift or missing configuration;
4. the agent edits code or `AspRustConfig` until the finding disappears, or
   records an explicit project-local rationale.

## Policy Surfaces

Project-root runners execute the full policy surface. By default they cover
Rust code under `src/`, `tests/`, `examples/`, and `benches/`, plus root
`build.rs` when it exists. Cargo workspace members are evaluated with their own
crate scopes. Explicit-path runners are syntax probes because they do not have
project ownership context.

`cargo check` is the primary downstream gate for parser-native policy: syntax,
Cargo manifest facts, module and owner graph, import clarity, scope coverage,
build-gate closure, and verification planning obligations. Cargo-test gates
remain a compatibility surface for retired crates and for this crate's self-apply
path, but they are not the preferred downstream entrypoint.

## Where Details Live

- Build gate, cargo-test compatibility, and configuration: [Runner Modes](03_features/202_runner_modes.md)
- Default rule packs and diagnostic rendering: [Rule Catalog](03_features/201_rule_catalog.md)
- Runtime-owned search/query and the language package boundary: [Provider Server](03_features/203_provider_server.md)
- Verification planning, receipts, waivers, and report artifacts: [Verification Policy](03_features/204_verification_policy.md)
- Boundary rationale and self-apply contract: [ASP Rust Boundary](01_core/101_asp_rust_boundary.md)
