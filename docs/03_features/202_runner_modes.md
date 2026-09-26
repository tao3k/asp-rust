# Runner Modes

The harness exposes two runner modes with different policy scope.

## Project Runner

Use `run_asp_rust()` or `assert_asp_rust_clean()` when a
caller has a project root. The project runner discovers conventional source and
test roots, builds a `AspRustScope`, and runs all default rule packs.
With the default config, every Rust file under `src/`, `tests/`, `examples/`,
and `benches/` is in the harness, and root `build.rs` is included when it
exists, so this is the crate package-level gate:

1. `rust.syntax`
2. `rust.project_policy`
3. `rust.modularity`
4. `rust.agent_policy`

This is the mode used by the Dev Gate.

When the requested root is a Cargo workspace or a directory that contains
multiple nested `Cargo.toml` package manifests, the project runner evaluates
each package as its own member scope. Test layout, `lib.rs` facade policy,
source-backed test mounts, and module reachability are therefore checked against
the owning crate root instead of the workspace directory. Workspace package
facts come from the shared Cargo manifest parser, so discovery and policy use
the same `Cargo.toml` interpretation.

## Cargo Test Embedding

Downstream crates load the harness only as a dev-dependency:

```toml
[dev-dependencies]
asp-rust = { git = "https://github.com/tao3k/asp-rust", branch = "main" }
```

```rust
#[cfg(test)]
asp_rust::asp_rust_cargo_test_gate!(
    mode = deny,
    config = asp_rust::default_asp_rust_config()
);
```

The gate runs only under `cargo test`. Native Rust syntax, Cargo manifest facts,
source/test scope, ownership, verification planning, and receipt policy are all
evaluated there. Ordinary `cargo build`, `cargo check`, and consumers of the
published crate do not resolve ASP Rust through that package edge.

Use `mode = warn` for an observation gate that renders findings and passes, or
`mode = deny` for an admission gate that fails on configured blocking
severities:

```rust
#[cfg(test)]
asp_rust::asp_rust_cargo_test_gate!(
    mode = warn,
    config = asp_rust::default_asp_rust_config()
);
```

`RUST-AGENT-PROJECT-006` reports normal/build dependency placement,
`RUST-AGENT-PROJECT-009` reports legacy build-script activation, and
`RUST-AGENT-PROJECT-012` reports an activated test gate without the required
dev-dependency closure.

## Configuration

`AspRustConfig.source_dir_names` and `test_dir_names` are project-root
relative paths. Source-scoped rule packs use the resolved `source_paths` as
their ownership boundary, so custom source roots receive the same source-test,
modularity, and agent advice checks as `src`.

Cargo is the baseline project manager for discovery. The project runner reads
`Cargo.toml` through the parser layer and keeps Cargo-owned code coverage in
scope even when an Agent passes a smaller config: conventional `src` and
`tests`, explicit `[lib]`, `[[bin]]`, and `[[test]]` target roots, plus
`examples`, `benches`, explicit `[[example]]`/`[[bench]]` targets, and root
`build.rs`. This applies to build-script gates too, so `build.rs` cannot become
a second hand-written scanner that quietly avoids old debt.

Custom scope paths must explain why they exist. Prefer
`with_source_path(path, explanation)` and `with_test_path(path, explanation)`;
directly mutating `source_dir_names` or `test_dir_names` without the matching
explanation map triggers `RUST-AGENT-PROJECT-013`. This prevents an Agent from shrinking
the harness to a few files just to avoid old policy debt.

Removing Cargo-backed scopes also needs a reason. If `src`, `tests`, or a
manifest-declared test target exists but an Agent removes it from the configured
scope, `RUST-AGENT-PROJECT-014` reports the attempt unless the config uses
`with_source_path_excluded(path, explanation)`,
`with_test_path_excluded(path, explanation)`, or
`with_tests_excluded(explanation)`.

Package target paths such as root `build.rs`, `examples/`, and `benches/` are
tracked separately from source roots. They receive syntax checks and
package-scope path advice without becoming public source API for agent doc/name
advice.

`include_tests = true` is the default and keeps configured test roots inside the
package-level harness. `include_tests = false` is an explicit downgrade that
removes configured test roots from recursive parsing. It does not disable
filesystem-level project policy such as root test-layout and test-target gate
structure checks. Use the explicit-path runner for syntax-only probes.

Policy findings are configurable through `AspRustConfig` after rule
evaluation and before the report is returned. `disabled_rules` removes matching
rule ids from the final finding list, while `rule_severity_overrides` changes a
matching finding's severity for that run. The `with_disabled_rule`,
`with_disabled_rules`, `with_disabled_rule_pack`, `with_rule_severity`,
`with_rule_pack_severity`, and `with_blocking_severities` builder methods
provide the stable library API for those controls. Pack-level helpers use the
`RustRulePack` enum and expand into the same rule-id collections, so the
serialized config shape remains unchanged. This keeps the default catalogs
deterministic while giving downstream crates a narrow way to turn a rule or pack
into advisory output or suppress rules they have intentionally replaced with
local policy.

Cargo-test `advice = allow` is not a generic pass switch. If a source gate uses
`asp_rust_cargo_test_gate!(advice = allow, config = { ... })`, the
same config should call `with_cargo_test_advice_allow_explanation(...)`.
Without that compact explanation, `RUST-AGENT-PROJECT-015` keeps the finding visible so
an Agent has to state why advisory policy may pass in the test layer instead of
silently using `allow` to avoid repairs.

## Explicit-Path Runner

Use `run_asp_rust_paths()` or `assert_asp_rust_paths_clean()` when a caller
only wants to inspect explicit files or directories. This runner has no project
scope, so project-scoped packs do not emit findings. The practical contract is:

1. `rust.syntax` still validates every discovered Rust file;
2. `rust.project_policy`, `rust.modularity`, and `rust.agent_policy` stay quiet
   because they require a project root and conventional ownership boundaries.

Use the project runner for repository policy gates. Use the explicit-path runner
for focused parser checks, editor integrations, and lightweight syntax probes.
