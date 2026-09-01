# Downstream Verification Gate

This guide is the agent-facing contract for crates that consume
`asp-rust` as a library. It separates build-script semantic
gates from command-line quick checks so downstream agents can generate an
adapted policy without reading harness internals.

## Crate Layout

Use this minimum layout for a crate that wants a Clippy-like semantic gate and
expects the policy to grow:

```text
my-crate/
  Cargo.toml
  build.rs
  harness/
    mod.rs
    owners.rs
    verification.rs
    receipts.rs
    reports.rs
    rules.rs
    dependencies.rs
  src/
    lib.rs
    ...
  tests/
    ...
  benches/
    ...
```

`Cargo.toml` owns the dependency edge. Add the harness under
`[build-dependencies]`, not only `[dev-dependencies]`, because the semantic gate
runs from `build.rs` during `cargo check`, `cargo test`, and workspace builds.

`build.rs` owns only the thin gate entrypoint. It should import the crate-local
policy module and call the harness assertion API:

```rust
#[path = "harness/mod.rs"]
mod harness;

use asp_rust::assert_asp_rust_downstream_policy_from_env;

fn main() {
    assert_asp_rust_downstream_policy_from_env(&harness::policy());
}
```

`harness/mod.rs` owns policy assembly. It should call into smaller modules and
return a `AspRustDownstreamPolicy`:

```rust
use asp_rust::{
    AspRustConfig, AspRustDownstreamPolicy, default_asp_rust_config,
};

mod owners;
mod receipts;
mod reports;
mod rules;
mod verification;

pub fn policy() -> AspRustDownstreamPolicy {
    AspRustDownstreamPolicy::new("my crate", config())
}

fn config() -> AspRustConfig {
    let config = default_asp_rust_config()
        .with_cargo_check_advice_allow_explanation(
            "crate keeps advisory findings visible while blocking policy drift",
        );

    let config = owners::apply(config);
    let config = verification::apply(config);
    let config = receipts::apply(config);
    let config = reports::apply(config);
    rules::apply(config)
}
```

`harness/owners.rs` owns project-relative owner classification. It should map
source files to semantic responsibilities, such as latency-sensitive performance
owners and availability-critical stability owners.

`harness/verification.rs` owns task adapters and task contracts. It should wire
Criterion, explicit task-kind overrides, skill bindings, or task contract
overrides.

`harness/receipts.rs` owns evidence inputs. It should attach verification
receipts, waivers, and known baseline evidence, not policy structure.

`harness/reports.rs` owns report artifact policy. It should configure source
baseline directories, runtime cache artifacts, sidecars, and trace settings.

`harness/rules.rs` owns project-specific rule tuning. It should configure rule
severity overrides, disabled findings, source/test scope exceptions, and
required explanations.

`harness/dependencies.rs` owns dependency baseline policy. It should construct a
`AspRustDependencyBaseline` for exact `Cargo.lock` requirements such
as `asp-rust` version and git rev. Do not parse `Cargo.lock` in
downstream policy modules; the upstream harness API owns lockfile parsing and
agent guidance.

`src/` owns the parser-observed implementation. Do not encode source structure in
the gate by string convention alone; use project-relative owner paths that the
Rust parser can resolve.

`benches/` owns performance evidence when performance owners are configured.
Criterion-backed projects should keep benchmark targets discoverable by Cargo so
receipts can point to real benchmark commands and artifacts.

`tests/` or an explicit project runner owns stability evidence when stability
owners are configured. Stability evidence should describe the command, iteration
window, latency distribution, resource delta, state growth, determinism result,
and artifact path.

## Workspace Layout

A workspace should own common policy once, then derive member crate policy from
that shared baseline. Do not copy the same `owners.rs`, `verification.rs`,
`receipts.rs`, `reports.rs`, and `rules.rs` into every crate when the workspace
can centralize them.

Use this minimum layout for a Cargo workspace with several member crates:

```text
my-workspace/
  Cargo.toml
  harness/
    mod.rs
    members.rs
    owners.rs
    verification.rs
    receipts.rs
    reports.rs
    rules.rs
    dependencies.rs
  crates/
    api/
      Cargo.toml
      build.rs
      src/
    db/
      Cargo.toml
      build.rs
      src/
    transport/
      Cargo.toml
      build.rs
      src/
```

`harness/mod.rs` should expose a workspace policy and a member policy helper:

```rust
use asp_rust::{
    AspRustDependencyBaseline, AspRustDownstreamPolicy,
    AspRustWorkspacePolicy, default_asp_rust_config,
};

pub enum WorkspaceMember {
    Api,
    Db,
    Transport,
}

pub fn workspace_policy() -> AspRustWorkspacePolicy {
    AspRustWorkspacePolicy::new(
        "my-workspace",
        reports::configure(
            receipts::configure(
                verification::configure(
                    owners::configure_common(rules::configure(default_asp_rust_config())),
                ),
            ),
        ),
    )
    .with_dependency_baseline(dependencies::baseline())
}

mod dependencies {
    use super::AspRustDependencyBaseline;

    pub fn baseline() -> AspRustDependencyBaseline {
        AspRustDependencyBaseline::new().require_git_package(
            "asp-rust",
            "0.1.2",
            "rev=<approved-rev>",
        )
    }
}

pub fn member_policy(member: WorkspaceMember) -> AspRustDownstreamPolicy {
    match member {
        WorkspaceMember::Api => workspace_policy().member_crate_with_config("api", owners::api),
        WorkspaceMember::Db => workspace_policy().member_crate_with_config("db", owners::db),
        WorkspaceMember::Transport => {
            workspace_policy().member_crate_with_config("transport", owners::transport)
        }
    }
}
```

Each member crate keeps only a thin `build.rs`:

```rust
#[path = "../../harness/mod.rs"]
mod harness;

use asp_rust::assert_asp_rust_downstream_policy_from_env;

fn main() {
    assert_asp_rust_downstream_policy_from_env(&harness::member_policy(
        harness::WorkspaceMember::Api,
    ));
}
```

`AspRustWorkspacePolicy` is the API boundary for shared workspace
policy. `member_crate` clones the common config into a crate policy.
`member_crate_with_config` clones the common config and then applies a
crate-local override, so a member can add an owner or waiver without mutating the
workspace baseline.

`AspRustDependencyBaseline` should be attached once to the workspace
policy when every member crate must resolve the same harness version or git rev.
Derived member policies inherit that baseline, and their `build.rs` gate searches
upward for the workspace `Cargo.lock`.

When a downstream agent adds the dependency and thin `build.rs`, `cargo test`
automatically triggers the member build.rs gate before tests run. Failing gates
print a stable `[asp-rust-agent-guidance]` block that tells the agent to keep
`build.rs` thin, move common policy into the workspace `harness/` tree, derive
members with `AspRustWorkspacePolicy`, and add crate-local owners,
receipts, waivers, or report obligations only in the member override.
In short: cargo test automatically triggers the member build.rs gate.

If a dependency baseline drifts, failing gates print a stable
`[asp-rust-dependency-guidance]` block. The repair path is to update the
stale direct or transitive dependency edge, refresh `Cargo.lock` with Cargo,
confirm `cargo tree -i` for the package, and rerun `cargo test`. Do not hand-edit
lockfile entries or keep a downstream-specific `Cargo.lock` parser.

## Classification

Library/build.rs semantic gate:

- thin downstream policy object:
  `AspRustDownstreamPolicy`.
- thin downstream policy assertion:
  `assert_asp_rust_downstream_policy_from_env`.
- cargo-check policy gate:
  `assert_asp_rust_cargo_check_clean_from_env_with_config`.
- full verification gate:
  `assert_asp_rust_verification_from_env_with_config`.
- owner classification helpers:
  `with_latency_sensitive_performance_owner` and
  `with_availability_stability_owner`.
- performance adapter wiring:
  `with_criterion_performance_verification`.
- persisted receipts, waivers, and report artifact policy that determine whether
  a crate has complete verification evidence.
- dependency baseline gate: `AspRustDependencyBaseline` with
  `assert_asp_rust_dependency_baseline`, normally attached through
  `AspRustDownstreamPolicy` or `AspRustWorkspacePolicy`.

CLI quick check and observation surface:

- `asp rust check` for fast local policy feedback.
- `asp rust search` and `asp rust query` for agent discovery.
- `asp rust agent doctor` for provider and registry health.
- `asp rust evidence`, `receipt`, `review`, `proof`, `behavior`, and
  `determinism` for bounded diagnostics or artifact generation.

Do not expose full verification as a standalone downstream CLI command. Full
verification is a crate semantic contract and belongs in the library API that
`build.rs` executes.

## Agent Inference

When adapting a downstream crate, an agent should:

1. Inspect Cargo package boundaries and source owners.
2. Create `build.rs` as a thin entrypoint and put policy assembly under
   `harness/mod.rs`.
3. Split policy into `owners.rs`, `verification.rs`, `receipts.rs`,
   `reports.rs`, and `rules.rs` once more than one responsibility is configured.
4. Put shared dependency baselines in `dependencies.rs` or the workspace
   `harness/mod.rs`, attach them through `with_dependency_baseline`, and let
   member crates inherit them from `AspRustWorkspacePolicy`.
5. Mark public dispatch paths, cache hot paths, parser hot paths, or provider
   transport paths as performance owners.
6. Mark timeout, retry, persistence, state growth, deterministic replay, or
   degradation/recovery paths as stability owners.
7. Add benchmark targets only when the selected owners need performance
   evidence.
8. Add stability receipts or runner evidence only when selected owners need
   stability evidence.
9. Keep quick CLI checks as developer feedback, not as the source of truth for
   full verification.

The output should be a thin `build.rs`, a `harness/` module tree, and a small
set of owner profiles that explain why each selected source path needs
performance or stability verification. If a crate is tiny, an agent may inline
the policy in `build.rs`, but the default recommendation is the modular layout
above because policy tends to grow.
