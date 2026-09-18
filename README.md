# asp-rust

`asp-rust` is the Rust language policy and evidence provider for
repair-oriented coding agents. It complements `rustc`, `rustfmt`, and Clippy by
turning native Rust syntax and Cargo manifest facts into compact
package/module/owner/dependency policy feedback.

ASP Rust is library-first. Runtime search, query, and projection are owned
by ASP Server. The `asp-rust` binary is only the Runtime-managed HTTP/JSON
provider server and is built behind the `provider-server` feature; it does not
expose provider-local business commands.

Policy admission is package-scoped and mounted once as a Cargo test. ASP Rust
does not wrap individual test functions with a procedural macro, so the package
policy scan still runs only once per test binary.

## What It Does

- Builds parser-native project facts from Rust source and Cargo manifests.
- Runs deterministic rule packs for syntax, project policy, modularity, and
  agent repair advice.
- Provides package and dependency-graph `cargo test` gates for downstream crates.
- Exposes parser-owned provider operations through the ASP Server HTTP/JSON
  transport.
- Plans verification obligations for external skills without running benchmarks,
  stress tests, security scanners, or other runtime tools itself.

## Quick Use

For downstream projects, add ASP Rust only as a dev-dependency:

```toml
[dev-dependencies]
asp-rust = { git = "https://github.com/tao3k/asp-rust", branch = "main" }
```

Then mount one test-only Dev Gate from `src/lib.rs`:

```rust,ignore
#[cfg(test)]
asp_rust::asp_rust_cargo_test_gate!(
    mode = deny,
    config = asp_rust::default_asp_rust_config()
);
```

Use `mode = warn` to render findings without failing the test, or `mode = deny`
to enforce the configured blocking contract. For a multi-package workspace,
keep the shared policy in the existing workspace Build Support crate. Member
packages reference that crate under `[dev-dependencies]`, and a thin test target
mounts its policy macro. Build Support may depend on `asp-rust` normally because
the member reaches the entire support graph only through its dev edge.

```rust,ignore
workspace_build_support::asp_workspace_policy_gate!();
```

Build Support owns that thin wrapper and the shared policy:

```rust,ignore
#[doc(hidden)]
pub use asp_rust;

#[macro_export]
macro_rules! asp_workspace_policy_gate {
    () => {
        $crate::asp_rust::asp_rust_workspace_dev_gate!(
            mode = deny,
            policy = $crate::workspace_policy()
        );
    };
}
```

The graph is parsed from the owning Cargo workspace's `members`, `exclude`, and
local path dependencies. ASP Rust admits the complete workspace instance in
deterministic dependency-first order, diamond dependencies occur once, and the
package content-and-policy cache avoids warm-build rescans. Callers do not
select package roots or reconstruct Cargo's build graph.

The binary is started by ASP Runtime using the catalog-declared `serve`
entrypoint. Direct `search`, `query`, `check`, `projection`, and `agent`
commands are intentionally rejected:

```shell
cargo build --features provider-server --bin asp-rust
# Runtime-owned launch shape; required ASP_PROVIDER_* identity variables omitted.
cargo run --features provider-server --bin asp-rust -- serve
```

Global install helpers live in [`Justfile`](Justfile):

```shell
just install-bin-macos
just install-bin-linux
```

## Development

This crate self-applies the default ASP Rust policy through its test suite.
Downstream crates should use the same dev-dependency-only Cargo test boundary.

Useful local checks:

```shell
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
```

## Docs

Detailed package material lives under [`docs/`](docs/index.md):

- [Overview](docs/00_overview.md)
- [ASP Rust Boundary](docs/01_core/101_asp_rust_boundary.md)
- [Rule Catalog](docs/03_features/201_rule_catalog.md)
- [Runner Modes](docs/03_features/202_runner_modes.md)
- [Provider Server](docs/03_features/203_provider_server.md)
- [Verification Policy](docs/03_features/204_verification_policy.md)
- [Repo-Local Agent Skills](skills/README.md)
