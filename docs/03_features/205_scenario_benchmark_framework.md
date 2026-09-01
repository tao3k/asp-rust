# Scenario Benchmark Framework

ASP Rust scenarios need a stable contract before they become performance evidence. A scenario is not only an input fixture; it must also say what the agent is meant to learn, which policy surface is under test, which Cargo test or bench target measures it, which Rust benchmark harness owns that target, and which speed and memory budgets protect the hot path.

The framework has three layers.

1. Contract validation is numeric and strict. The harness reads scenario metadata and benchmark receipts, then fails when required fields are missing or when observed speed or memory exceeds the configured budget.
2. Coverage validation is structural. The harness discovers every required scenario root and fails when a scenario is missing `benchmark.toml`. This prevents future scenarios from landing as unbounded fixtures.
3. Snapshot validation is semantic and stable. Tests render a normalized scenario receipt through `insta`. Dynamic values such as absolute paths, measured durations, memory bytes, and timestamps are replaced with placeholders. The snapshot records the output shape, policy ids, agent-facing guidance, Cargo/harness benchmark entry, and pass/fail status.

This split keeps the performance gate useful. The numeric gate catches real regressions; the snapshot catches confusing output or contract drift without turning every machine-specific timing value into snapshot noise.

## Fixture Layout

Native ASP Rust scenarios live under a bounded fixture root:

```text
tests/unit/scenarios/<group>/<scenario-id>/
  scenario.toml
  benchmark.toml
  inputs/
  expected/
  receipts/
```

`scenario.toml` describes the intent:

```toml
id = "control-flow-v1"
title = "Control-flow verification stays bounded"
policy_ids = ["RUST-AGENT-CFG-001"]
agent_goal = "Find the control-flow owner before editing."
inputs = "inputs"
expected = "expected"
```

`benchmark.toml` describes the performance contract:

```toml
harness = "libtest"
test = "scenario_benchmark_control_flow_v1_snapshot"
snapshot = "scenario_benchmark_control_flow_v1"
target_total = "25ms"
max_total = "100ms"
observed_total = "18ms"
regression_budget = "20ms"
memory_budget_bytes = 8388608
observed_memory_bytes = 4194304
target_rationale = "Parser-native owner selection should remain a small fixture test."

[observed_timings]
parse = "750us"
render = "1.2ms"
snapshot = "500us"
```

Required fields are part of the contract. A scenario without speed, memory, Cargo/harness benchmark entry, or rationale is invalid even if the fixture currently passes.

For Criterion, Divan, or iai-callgrind, use the Cargo bench target and a focused case instead of a shell command:

```toml
harness = "criterion"
bench = "parser_owner_lookup"
case = "workspace_file_guard"
```

CLI AST patch scenarios use their existing manifest shape and are adapted into the same benchmark contract:

```text
tests/fixtures/ast_patch_scenarios/<scenario-id>/
  scenario.json
  benchmark.toml
  input/
  expected/
  packet.json
```

Every directory with `scenario.toml` under `tests/unit/scenarios` and every directory with `scenario.json` under `tests/fixtures/ast_patch_scenarios` is a required benchmark scenario. The suite gate must fail when any of those roots is missing `benchmark.toml`.

Build-system scenarios are executable graph fixtures rather than prose-only
examples. The V1 package-once scenario is a four-package Cargo diamond:

```text
tests/unit/scenarios/build_system/workspace_dependency_graph_package_once_v1/
  inputs/
    Cargo.toml
    app/       # depends on left and right
    left/      # depends on shared
    right/     # depends on shared
    shared/
  expected/
  scenario.toml
  benchmark.toml
```

Its snapshot test selects `app` through the production Build DAG API and proves
the dependency-first order `shared`, `left`, `right`, `app`, with `shared`
present exactly once. Separate focused fixtures cover selected-root closure,
unknown package rejection, and cycle rejection. This keeps Scenario V1 aligned
with the downstream build API instead of duplicating graph semantics in TOML.

## Gate Semantics

The harness reports:

- `pass` when observed speed and memory stay within budget.
- `fail` when any observed value exceeds its budget.
- `invalid` when required metadata is missing or contradictory.

`observed_total` must be less than or equal to `max_total`. `observed_memory_bytes` must be less than or equal to `memory_budget_bytes`. `target_total` and `regression_budget` are explanatory fields that tell an agent where optimization headroom remains.

Durations use Rust `Duration`-style unit strings instead of integer-only millisecond fields. Use `ns`, `us`, `ms`, or `s`; decimal values are accepted when they resolve exactly to nanoseconds, such as `750us`, `1.2ms`, and `0.5s`.

The current default fixture budget for small AST patch scenario contracts is `target_total = "25ms"`, `max_total = "100ms"`, and `memory_budget_bytes = 8388608`. Wider gates need a scenario-specific rationale in `target_rationale`; a large number without rationale is a contract failure in review even when the numeric check passes.

## Insta Role

`insta` snapshots should include normalized receipts such as:

```text
scenario: control-flow-v1
status: pass
policies: RUST-AGENT-CFG-001
bench_entry: harness=libtest test=scenario_benchmark_control_flow_v1_snapshot snapshot=scenario_benchmark_control_flow_v1
observed_total: <measured>
target_total: 25ms
max_total: 100ms
observed_memory_bytes: <measured>
memory_budget_bytes: 8388608
timings: parse=<measured>, render=<measured>, snapshot=<measured>
agent_goal: Find the control-flow owner before editing.
```

The snapshot makes the agent-facing contract reviewable. The numeric assertions remain in normal Rust tests so regressions are caught by values, not by snapshot churn.
