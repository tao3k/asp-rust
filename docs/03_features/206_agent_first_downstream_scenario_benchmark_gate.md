# Agent-First Downstream Scenario Benchmark Gate

The first consumer of the ASP Rust is an agent that edits code and then
runs tests. The scenario benchmark interface must therefore make the correct
repair path cheaper than bypassing the gate.

The downstream gate is a hard unit-test API, not an advisory report. A downstream
crate installs one test:

```rust
#[test]
fn rule_fixtures_have_scenario_benchmarks() {
    asp_rust::assert_rule_fixture_scenario_benchmarks(env!("CARGO_MANIFEST_DIR"));
}
```

When a rule-applied fixture, policy fixture, AST patch fixture, or declared
scenario fixture is discovered, it must carry a scenario contract and
`benchmark.toml`. The gate fails when the benchmark contract is missing,
invalid, slower than its configured maximum, or over its memory budget.

## Anti-Escape Rules

- No default advisory mode.
- No fixture-local opt-out field.
- No `expires` field.
- No hidden fallback that turns a missing benchmark into a warning.

If a fixture truly does not exercise a rule application path, a future central
allowlist may record it outside the fixture tree. That allowlist must be small,
snapshot-checked, and reviewed as a policy artifact. It must not be the primary
repair path.

## Failure UX

The panic text is written for an agent. It states the failing fixture and gives a
minimal `benchmark.toml` template. The preferred repair is always to add the
benchmark contract:

```text
preferred fix: add benchmark.toml next to the scenario fixture
```

The template keeps the default small-fixture gate as Rust `Duration`-style unit strings:

```toml
harness = "libtest"
test = "<focused-libtest-case>"
snapshot = "<insta-snapshot-name>"
# For Criterion, Divan, or iai-callgrind use:
# harness = "criterion"
# bench = "<cargo-bench-target>"
# case = "<benchmark-group-or-function>"
target_total = "25ms"
max_total = "100ms"
observed_total = "25ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Small rule fixture should stay bounded."

[observed_timings]
fixture = "25ms"
```

Larger fixtures can raise budgets, but the rationale must explain why the
fixture is larger. The gate still checks the numeric budget.

## First Downstream Experiment

After the ASP Rust GitHub CI is green, `languages/orgize` becomes the first
downstream experiment. The experiment should add the unit-test API above, run it
against Orgize's rule or scenario fixtures, and only then extend discovery rules
for fixture layouts that Orgize actually uses.
