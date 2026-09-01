---
name: asp-rust-policy
description: "Use when modifying asp-rust Rust source, tests, policy rules, parser facts, renderers, or snapshots."
---

# ASP Rust Policy

Use this skill when onboarding to this repository or when compact harness output
does not give enough context to repair `src/` or `tests/` Rust code directly.
Do not reread it on every run after the relevant code-level policy and skill
bindings are configured.

## Operating Contract

- Treat the native Rust parser as the fact layer. New policy behavior must
  consume parser/discovery facts, not raw string scans or ad hoc file lists.
- Do not compete with rustc, rustfmt, or Clippy. Harness policy covers
  LLM/Agent structural risk: owner drift, module shape, unclear edit surfaces,
  special-file bloat, oversized files, unclear dependency paths, compact repair
  output, and verification obligations.
- Keep compact text Agent-facing. Avoid human audit headers such as package
  counts, file counts, parsed counts, source-root summaries, empty sections, and
  success prose. Clean compact output should stay minimal.
- Keep `src/` and `tests/` fully inside harness self-apply. A policy change that
  would let `cargo test` or `cargo test --lib` escape the harness is a regression.
- Prefer owner-level APIs and `crate::...` paths over broad scope imports.
  `super::super` and broad glob imports are structural clarity risks for this
  repository even when Rust permits them.
- Keep medium or complex work folder-first. If a file starts collecting several
  unrelated responsibilities, split by parser, rule, renderer, profile, fixture,
  or snapshot ownership.
- Every new policy shape needs focused tests and, where compact text changes,
  an insta snapshot.

## Parser-First Checklist

- Add or extend parser facts first.
- Deduplicate facts at the parser/discovery boundary when the policy would
  otherwise need local deduplication.
- Preserve line numbers and test-context metadata when diagnostics or policies
  depend on them.
- Let rules ask precise questions of the fact model instead of rediscovering
  path or syntax relationships.

## Validation

Run the repository validation set before committing code changes:

```shell
direnv exec . cargo test
direnv exec . cargo fmt --check
direnv exec . cargo clippy --all-targets -- -D warnings
git diff --check
```

For docs-only skill updates, `git diff --check` is the minimum. If the update
changes executable examples or validation instructions, run the full set.

GitHub Actions should be checked for Ubuntu and Windows. A pending macOS check
can be ignored while the project account has no macOS Actions quota.
