# asp-rust Skills

This directory contains repo-local Agent skills for working on
`asp-rust`.

The skills are intentionally small and harness-specific. They are not a vendored
copy of general Rust best-practice catalogs, and they are not meant to be loaded
on every harness run. They are passive progressive guidance for humans and
Agents that need to configure the code-level policy surface.

Once `RustVerificationSkillBinding` is configured for a task kind, compact
verification output switches to a short `skill=<id>` dispatch hint and stops
printing repeated contract, requirement, and fact lines for that task. The full
contract remains available in JSON.

## When To Load

- Read [`asp-rust-policy`](asp-rust-policy/SKILL.md) when onboarding to
  this repository or repairing parser/policy drift that compact output cannot
  resolve alone.
- Read [`rust-verification-profile`](rust-verification-profile/SKILL.md) when
  configuring verification profiles, receipts, waivers, or skill bindings.
- Read [`rust-verification-performance`](rust-verification-performance/SKILL.md)
  only when no configured performance binding exists, or when repairing that
  binding.

## Reference Repositories Studied

These repositories were shallow-cloned under `.run/tmp` on 2026-05-01 and used
only as organization references:

- [`leonardomso/rust-skills`](https://github.com/leonardomso/rust-skills)
  (`89910e858533`): useful as a compact rule-catalog shape.
- [`actionbook/rust-skills`](https://github.com/actionbook/rust-skills)
  (`1f4becdcb88d`): useful as a skill-folder and routing shape.

Do not copy their rule text into this repository. Convert useful patterns into
parser facts, verification contracts, skill bindings, or configuration helpers.
This repository's skills must remain original, compact, and tied to
`asp-rust` facts.
