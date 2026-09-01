# asp-rust

`asp-rust` is the Rust language policy and evidence provider for
repair-oriented coding agents. It complements `rustc`, `rustfmt`, and Clippy by
turning native Rust syntax and Cargo manifest facts into compact
package/module/owner/dependency policy feedback.

ASP Rust is library-first. Runtime search, query, and projection are owned
by ASP Server. The `asp-rust` binary is only the Runtime-managed HTTP/JSON
provider server and is built behind the `provider-server` feature; it does not
expose provider-local business commands.

The workspace's `build-support` crate contains lightweight build-script
plumbing and does not own policy evaluation. Policy admission remains
package-scoped; ASP Rust does not wrap individual test functions with a
procedural macro that would repeat the package policy scan per test.

## What It Does

- Builds parser-native project facts from Rust source and Cargo manifests.
- Runs deterministic rule packs for syntax, project policy, modularity, and
  agent repair advice.
- Provides a build-script `cargo check` gate for downstream crates.
- Exposes parser-owned provider operations through the ASP Server HTTP/JSON
  transport.
- Plans verification obligations for external skills without running benchmarks,
  stress tests, security scanners, or other runtime tools itself.

## Quick Use

For downstream projects, add ASP Rust as a build-dependency:

```toml
[build-dependencies]
asp-rust = { git = "https://github.com/tao3k/asp-rust", branch = "main" }
```

Then call the build gate from a thin root `build.rs`:

```rust,ignore
fn main() {
    let config = asp_rust::default_asp_rust_config();
    asp_rust::assert_asp_rust_cargo_check_clean_from_env_with_config(
        &config,
    );
}
```

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

This crate self-applies the default ASP Rust policy. Downstream crates should
prefer the build-time `cargo check` gate; this crate uses a self-apply path
because it cannot build-depend on itself.

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
