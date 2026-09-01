# ASP Rust Self-Apply Policy Design

## Goal

`asp-rust` should use the same project harness policy
that it asks downstream Rust projects to mount. The crate already does this in
code; this design makes that contract explicit in docs and locks it with focused
tests.

## Contract

The crate self-applies policy through two mounts:

1. `src/self_policy.rs` mounts `asp_rust_cargo_test_gate!`, which is
   compiled by the library test target.
2. `tests/unit_test.rs` is a thin aggregate that mounts suite modules only.

That shape keeps the source-embedded library gate as the single policy entry
point for `cargo test --lib` and ordinary test runs.

## Policy Semantics

Default project runs execute all built-in rule packs:

1. `rust.syntax`
2. `rust.project_policy`
3. `rust.modularity`
4. `rust.agent_policy`

`Warning` and `Error` findings block `assert_clean()` by default. `Info`
findings stay visible in rendered advice but do not fail the gate. `AGENT-*`
rules are intentionally `Info` so agents get repair hints without turning every
style concern into hard policy.

The public `rust_rule_pack_descriptors()` catalog exposes this pack order with
stable ids, version labels, searchable domains, and default modes. Descriptor
order should match default execution order.

Project-root runners execute this full policy surface. Explicit-path runners do
not build a project scope, so they serve as syntax probes and leave
project-scoped packs quiet.

Default project runs are package-level: all Rust code under `src/`, `tests/`,
`examples/`, and `benches/` is in the harness, and root `build.rs` is included
when present. This keeps the harness from silently ignoring test helpers,
source-backed unit suites, root test targets, examples, benchmarks, or Cargo
build-script entrypoints.

Configured source roots are part of the policy contract: source-scoped packs use
`AspRustScope.source_paths`, not a hardcoded `src` directory. Setting
`include_tests = false` skips recursive parsing of test roots, but does not turn
off filesystem-level test-layout policy.

`lib.rs` is a crate facade, not an implementation owner. It may declare external
modules and re-export APIs; macro implementations, self-policy mounts, and other
logic must move into owned modules. `mod.rs` is the matching module-boundary
facade. Together, `lib.rs` and `mod.rs` are special Rust ownership files and get
targeted policy rather than being treated as ordinary leaves.
Binary entrypoints are also special files: `src/main.rs` and `src/bin` targets
should stay thin, with parser/options/execution logic moved into owned modules.
Root `build.rs` follows the same entrypoint rule for Cargo build scripts.

Path clarity policy is syntax-backed. `super::super` checks inspect parsed Rust
`use` trees instead of matching raw text, and repeated namespace advice evaluates
the package harness surface, including `src/`, `tests/`, and ordinary Rust file
stems.

## Documentation Surface

The README and core docs should state that the package is library-like while
still self-hosted by its own harness. `development.md` should contain only this
repo's validation commands, not inherited benchmark or snapshot workflows from
unrelated parser projects.

## Tests

Focused policy tests should cover:

1. default blocking severities are exactly `Warning` and `Error`;
2. every `AGENT-*` catalog rule is `Info`;
3. the current project harness run is clean under its own default policy;
4. the source-backed self-apply mount remains present in `src/lib.rs`;
5. the root Cargo test target relies on the source-backed cargo-test gate;
6. path clarity rules catch syntax-level `super::super`, repeated namespace
   segments in source and test roots, and duplicated public names;
7. root Cargo test targets stay thin aggregates instead of owning test bodies;
8. root Cargo test target module mounts use explicit `#[path]` attributes into
   allowed suite directories;
9. root `build.rs` is scanned and stays a thin build-script entrypoint.

These tests live under `tests/unit` and are included by `tests/unit_test.rs`, so
adding coverage does not introduce a new root test target that must be justified.

## Fixed Layout Policy

The harness accepts no project-local layout allowlist. A package that needs an
isolated test process declares a Cargo test target whose source lives below a
standard suite directory; all other findings are repaired directly.

## Render Policy

The default renderer includes non-blocking advice after the blocking section.
Projects with only `Info` findings remain clean, but the compact advice remains
visible to repair-oriented agents.

Finding text renders as a compact repair card: `@ path:line:column`, `fix:`,
source line when available, `Help:` from the concrete finding summary, and
`Contract:` from the stable rule requirement. Structured consumers should keep
using the serializable `AspRustReport` fields or
`render_asp_rust_json()` for JSON
output instead of parsing compact text.

The CLI follows the same render contract: compact text is default, `--json`
emits the structured report, and process exit status is based only on
configured-blocking findings.
