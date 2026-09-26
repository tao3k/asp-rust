---
name: rust-verification-performance
description: "Use when a asp-rust task touches latency, throughput, allocation behavior, parser/render loops, or Rust performance verification contracts."
---

# Rust Performance Verification

Use this skill when the changed owner is latency-sensitive, throughput-sensitive,
allocation-sensitive, parser-heavy, renderer-heavy, async/concurrent, or on a
hot path.

The harness does not run benchmarks itself. It plans an obligation for an
external Agent skill and accepts a receipt or waiver for the current
fingerprint.

If the compact task line already contains `skill=rust-verification-performance`
or another configured performance skill id, do not reload this file just to
learn the contract. Use the structured JSON task or the project-owned adapter
configuration. This file is fallback guidance for configuration or repair.

## Trigger Signals

- The owner profile includes `RustOwnerResponsibility::LatencySensitive`.
- The task changes parser loops, render loops, snapshot generation, dependency
  graph traversal, module discovery, config planning, async execution, cache
  behavior, or allocation-heavy collection logic.
- The user asks about performance, regression, p50/p99/p999, throughput,
  allocations, profiling, benchmark stability, or runtime degradation.
- A policy change increases the amount of code or project shape the harness must
  scan per run.

## Evidence Contract

A `RustVerificationTaskKind::Performance` receipt must report these evidence
keys for the task fingerprint:

- `benchmark_command`: the exact command that was run.
- `baseline`: the comparison point, such as main branch, previous release, saved
  benchmark baseline, or explicit historical measurement.
- `regression_threshold`: the allowed change before the SLA is considered
  broken.
- `latency_or_throughput`: the measured latency, throughput, or both.
- `allocation_profile`: allocation count, bytes, allocator profile, or a clear
  statement that allocations are not part of this owner's contract.
- `profile_artifact`: path or link to a flamegraph, profiler report, benchmark
  report, or other retained artifact.

If one key is not applicable, the receipt still needs a concrete explanation for
that key. Empty prose does not clear the task.

## Recommended Rust Tooling

- Prefer the project-owned benchmark command when one exists.
- Use `cargo bench` for native benchmark entrypoints.
- Use Criterion when statistical comparison and saved baselines matter.
- Use Divan when the project wants lightweight Rust benchmark functions.
- Use iai-callgrind when deterministic instruction/cache-style evidence is more
  useful for CI than wall-clock timing.
- Use flamegraph or another profiler when the task is about the bottleneck,
  allocation source, lock contention, or CPU hot path rather than benchmark
  score alone.

## Agent Workflow

1. Identify the owner path and task fingerprint from the verification plan.
2. Select the narrowest benchmark/profiling command that exercises that owner.
3. Run after unit tests pass unless the contract says otherwise.
4. Compare against the declared baseline and threshold.
5. Emit a `RustVerificationReceipt::passed(...)` only when every required
   evidence key is present.
6. Use `RustVerificationWaiver` only when the task is intentionally out of scope;
   include owner, reason, and expiry.

Do not turn this into a Clippy-style optimization lint. The harness only asks
for performance evidence when project structure, responsibility config, or
parser facts say that evidence matters.
