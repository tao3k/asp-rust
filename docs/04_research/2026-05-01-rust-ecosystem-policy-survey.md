# Rust Ecosystem Policy Survey

Date: 2026-05-01

This note records a focused survey of official Rust guidance and mature Rust
repositories. Its purpose is to separate native Rust/Cargo facts from
`asp-rust` policy choices, especially where our LLM-oriented
defaults are stricter than common Rust practice.

## Sources

Official references:

- [Cargo package layout](https://doc.rust-lang.org/nightly/cargo/guide/project-layout.html)
- [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html)
- [rustdoc documentation guide](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- [Rust API Guidelines: documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [PingCAP Rust module style guide](https://pingcap.github.io/style-guide/rust/modules.html)

Repository sample, shallow-cloned on 2026-05-01:

| Repository | Commit |
| --- | --- |
| [tokio-rs/tokio](https://github.com/tokio-rs/tokio/tree/26dee92b535d0a204531b6a04ee761f06c402d7e) | `26dee92b535d` |
| [clap-rs/clap](https://github.com/clap-rs/clap/tree/7e0bccab8cf7be047fc84d804d19c7b30715d3fb) | `7e0bccab8cf7` |
| [serde-rs/serde](https://github.com/serde-rs/serde/tree/fa7da4a93567ed347ad0735c28e439fca688ef26) | `fa7da4a93567` |
| [serde-rs/json](https://github.com/serde-rs/json/tree/dc8003a88e7142529cf4a7429c4778af31dadf50) | `dc8003a88e71` |
| [hyperium/hyper](https://github.com/hyperium/hyper/tree/b12f6525432e7fbe80b749fec26f8ed7723006fc) | `b12f6525432` |
| [tower-rs/tower](https://github.com/tower-rs/tower/tree/251296dc54a044383dffd16d2179b443e2615672) | `251296dc54a0` |
| [dtolnay/anyhow](https://github.com/dtolnay/anyhow/tree/841522b2aa09732fecee40804440d2c35c68c480) | `841522b2aa09` |
| [rustls/rustls](https://github.com/rustls/rustls/tree/949c440f3113d7346f2e2afd244dcabb3638d631) | `949c440f3113` |
| [BurntSushi/ripgrep](https://github.com/BurntSushi/ripgrep/tree/4519153e5e461527f4bca45b042fff45c4ec6fb9) | `4519153e5e46` |
| [rust-lang/rust-analyzer](https://github.com/rust-lang/rust-analyzer/tree/aa64e4828a2bbba44463c1229a81c748d3cce583) | `aa64e4828a2b` |

## Official Guidance Boundary

Cargo layout and target discovery are native facts. Package roots, `src/lib.rs`,
`src/main.rs`, `src/bin`, `examples`, `benches`, integration tests, and explicit
manifest targets belong in the parser/discovery layer rather than in rule-local
path guessing.

Documentation and module-shape guidance are softer. Rustdoc and API guidelines
support crate-level and public-item documentation, while the PingCAP style guide
supports module-level docs, small coherent modules, clear re-exports, and avoiding
generic `util` buckets. These are good harness signals for agent repair, but they
are not equivalent to Cargo or compiler requirements.

Glob imports are nuanced. The PingCAP guide discourages glob imports generally,
but explicitly treats prelude imports and `use super::*` in test modules as common
exceptions. That makes a blanket `glob import = warning` rule useful for
LLM-oriented strictness, but too strict to present as universal Rust practice.

## Repository Signals

The following counts are grep-level signals from the sampled checkout. They are
not policy decisions by themselves.

| Repository | Rust files | `src/lib.rs` files | root `src/lib.rs` lines | `use super::*` hits | `#[cfg(test)]` hits | `mod tests` hits |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| tokio-rs/tokio | 777 | 7 | - | 82 | 49 | 40 |
| clap-rs/clap | 329 | 8 | 110 | 23 | 22 | 20 |
| serde-rs/serde | 208 | 3 | - | 19 | 0 | 0 |
| serde-rs/json | 70 | 1 | 441 | 3 | 0 | 0 |
| hyperium/hyper | 95 | 1 | 139 | 17 | 29 | 17 |
| tower-rs/tower | 133 | 4 | - | 9 | 14 | 8 |
| dtolnay/anyhow | 37 | 1 | 728 | 3 | 1 | 1 |
| rustls/rustls | 167 | 8 | - | 50 | 67 | 51 |
| BurntSushi/ripgrep | 100 | 9 | - | 16 | 161 | 36 |
| rust-lang/rust-analyzer | 1462 | 44 | - | 244 | 462 | 334 |

Observed `lib.rs` pattern:

- Several mature crates have large facade files. Examples include
  `tokio/tokio/src/lib.rs` at 694 lines, `serde_json/src/lib.rs` at 441 lines,
  `anyhow/src/lib.rs` at 728 lines, and `rust-analyzer/crates/hir/src/lib.rs`
  above 7k lines.
- Large `lib.rs` files are often docs, feature gates, module declarations, and
  re-exports. A line-count-only `lib.rs` policy would be noisy.
- Implementation in special entrypoints is still a useful harness risk signal,
  but the rule must focus on native item shape, not raw size.

Observed test/import pattern:

- `#[cfg(test)] mod tests` and `use super::*` are common in mature projects.
- Some `use super::*` hits are clearly inside tests; a parser-native future cut
  should classify use statements by enclosing `#[cfg(test)]` module or test-only
  source root before deciding severity.
- Until that context exists, glob-import strictness should stay configurable.

Observed workspace pattern:

- Mature repos often use workspaces with many crates and package-local `src/lib.rs`
  files. Package-level scope is the correct harness unit. Workspace-root
  assumptions should not drive source policy.

## Policy Matrix

| Surface | Evidence | Harness stance |
| --- | --- | --- |
| Cargo package and target layout | Cargo docs define conventional files and manifest overrides. | Parser/discovery fact layer, default on. |
| Workspace member scope | Mature repos are often multi-crate workspaces. | Package-local evaluation, default on. |
| `lib.rs`, `main.rs`, `mod.rs`, `build.rs` special roles | Cargo and module semantics make these native entrypoints. | Parser fact layer; policy should inspect item shape, not line count. |
| `lib.rs` facade implementation | Mature `lib.rs` can be large but often facade/doc/re-export heavy. | Keep special-file implementation policy, but make disabling/severity override available. |
| Source file bloat | Large files occur in mature repos, sometimes intentionally. | Keep as LLM drift signal; thresholds and severity should remain configurable. |
| Inline source tests | Common in mainstream Rust. | This is harness-specific, not universal; keep configurable. |
| `use super::*` / glob imports | Discouraged generally, but common in test modules and preludes. | Keep parser-native detection; default strictness is LLM-oriented and must be configurable. |
| Module-level docs and public docs | Supported by rustdoc/API/style guidance. | Agent advice by default; non-blocking. |
| Generic module names (`utils`, `common`) | Style guidance treats them as code smells. | Agent advice by default; non-blocking. |
| `super::super` owner escape | Rust permits it; agent repair often loses ownership clarity here. | Harness policy, default warning for source ownership roots; configurable. |

## Implications For This Harness

1. The parser remains the source of truth. New policies should ask for package,
   module, owner branch, edge kind, target kind, and test-context facts instead
   of scanning raw strings.
2. Defaults can be opinionated for LLM-generated code, but every rule that
   conflicts with common Rust practice needs library-level configuration. The
   current `disabled_rules` and `rule_severity_overrides` interface, plus
   library-only rule-pack helpers that expand into those fields, is the right
   first boundary.
3. The next parser-native improvement should classify native `use` statements by
   context: source root vs package target vs test root, top-level vs nested,
   inside `#[cfg(test)]` module, and prelude-like path. That would let
   `RUST-MOD-R010` be strict in owner modules while quieter in tests.
4. Do not add Clippy-shaped rules. The harness should avoid style rules that
   rustfmt, rustc, or Clippy already own. It should focus on project facts that
   help agents choose the correct owner, branch, and edit surface.

## Follow-up: Facade Boundary Macros

A second pass against official docs and live GitHub source samples sharpened the
`lib.rs` policy boundary:

- Cargo defines `src/lib.rs` as the library target root, but it does not require
  that file to be only `mod` and `pub use`.
- Mature crates often keep crate-level feature contracts in `lib.rs`. Tokio uses
  cfg-gated `compile_error!` items plus feature-gated module and re-export
  macros; Serde keeps docs.rs/cross-crate re-export shims in the crate root;
  Cargo exposes some boundary error helpers in its library root.
- Those forms are different from LLM drift where the facade grows local business
  structs, helper functions, or `macro_rules!` implementations.

Harness implication: `RUST-MOD-R004` should continue to reject implementation
items in `lib.rs`, but parser-native facade exceptions are valid when the
top-level macro is a cfg-gated `compile_error!` contract or when `syn` can parse
the macro body as facade-only items (`mod`, `use`, `extern crate`, or recursively
facade-only macro invocations). Local `macro_rules!` definitions still remain
implementation and should stay outside the facade.

## Follow-up: Rust Skill Catalogs

Two Rust skill repositories were shallow-cloned on 2026-05-01:

| Repository | Commit | Useful Pattern |
| --- | --- | --- |
| [leonardomso/rust-skills](https://github.com/leonardomso/rust-skills/tree/89910e858533) | `89910e858533` | Compact rule-catalog folders such as `perf-*`, `mem-*`, `async-*`, `proj-*`, and `test-*`. |
| [actionbook/rust-skills](https://github.com/actionbook/rust-skills/tree/1f4becdcb88d) | `1f4becdcb88d` | Skill routing, skill folders, performance skill triggers, and adapter-oriented Agent workflow. |

Harness implications:

1. Do not migrate large Rust skill Markdown into the active prompt path. That
   would recreate the token-noise problem the harness is meant to solve.
2. Convert useful rule-catalog content into typed contracts, evidence keys,
   parser facts, or configurable policy helpers. For example, performance
   guidance becomes `RustVerificationTaskKind::Performance` evidence such as
   `benchmark_command`, `baseline`, `regression_threshold`,
   `latency_or_throughput`, `allocation_profile`, and `profile_artifact`.
3. Convert skill routing into configuration, not prose. The harness now exposes
   `RustVerificationSkillBinding` so a project can bind a task kind to a
   configured Agent skill adapter. Once configured, compact output emits only a
   short `skill=<id>` dispatch hint while structured JSON keeps the full
   contract.
4. Treat the binding as part of the verification task fingerprint. If an Agent
   switches from one adapter to another, stale receipts should not silently clear
   the new obligation.
5. Keep the Markdown under top-level `skills/` passive and progressive. It is
   useful for onboarding or repairing configuration, but a healthy configured
   project should not need to reread it on every run.
6. Split skill routing from skill execution contracts. A
   `RustVerificationSkillBinding` answers "which configured skill handles this
   task?", while a `RustVerificationSkillDescriptor` answers "what compact tool,
   command, standard, inputs, and receipt fields define that adapter?" The
   default verification render should expose only `contract_ref` and let Agents
   expand descriptors on demand.
7. Keep service pressure and Rust-native performance separate. `k6` is a good
   `stress` adapter for HTTP/API/service-boundary SLA evidence, but it is not
   the Rust performance baseline. The `performance` task should prefer
   `cargo bench`, Criterion, Divan, iai-callgrind, and profiling artifacts for
   Rust code-level latency, throughput, allocation, instruction, and cache
   regression evidence.
8. The actionbook type-driven skill maps cleanly to advisory parser policy when
   it is restricted to semantic API boundaries. `RUST-AGENT-API-TYPE-012` therefore records
   public function parameters through `syn` and flags `id` or `*_id` parameters
   carried by `String`, `&str`, integer primitives, or `Option` wrappers. This
   keeps the useful "primitive obsession" signal without turning clone usage,
   unwraps, index loops, or other Clippy-shaped anti-pattern examples into
   harness rules.
9. The actionbook error-handling skill maps to parser policy at the public
   boundary, not at every `unwrap` or `?` site. `RUST-AGENT-API-ERROR-013` records public
   function return types through `syn` and flags application-style catch-all
   boundaries such as `anyhow::Result`, `eyre::Result`, and
   `Result<_, Box<dyn Error>>`. This preserves the library-vs-application
   signal while avoiding Clippy-shaped panic, unwrap, and ignored-error rules.
