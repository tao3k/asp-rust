# Rust Provider AST Patch Scenarios

Each scenario is an end-to-end provider mutation check:

```text
001_name/
  scenario.json
  packet.json
  input/
  expected/
  compact/            # optional save-token projection fixtures
```

The runner copies `input/` to a temporary project, invokes
`asp rust ast-patch <mode> --packet - <temp-project>`, asserts receipt fields
from `scenario.json`, and compares the final filesystem tree to `expected/`.
Successful scenarios may also declare `compactChecks`. Those checks run after
the provider apply and rustfmt pass, so they verify both products from the same
formatted file: `expected/` is the exact source/preimage authority, while
`compact/` stores the token-saving semantic outline fixtures. A compact check
can also reference `compact/apply-patch.json`; that file stores a low-token
patch intent, and the runner turns it back into a provider-native AST patch via
parser-owned `patchSafety.target` plus exact-read fixture content. Save-apply
fixtures may also require input and expected functional complexity, proving both
the preimage compact projection and the replacement compact projection cover a
real feature surface while still saving tokens.
Compact checks can set `minimumFunctionalComplexity` so save-token fixtures
must be backed by parser-owned responsibilities, such as cfg feature gates,
async item shape, generic bounds, mutation, guard branches, loops, and early
returns. Node count, nesting depth, and compact/exact size ratio are residue
checks; they are not the primary proof that a fixture represents a real feature
surface.
Compact code fixtures preserve rustfmt-style four-space indentation derived
from parser depth and keep opening/closing braces visible. They may omit
source trivia and summarize literal payloads, but nested block ownership must
remain visible in the compact text.

These fixtures mirror the portable shape of Codex `apply_patch` scenarios, but
the operation under test is provider-native Rust AST `replace_item`. The
`input/` and `expected/` trees are fake project roots; they only need the source
files under test, not `Cargo.toml`, `Cargo.lock`, or build output.

Scenarios named `tokio_style_*` use portable fixtures shaped like a large async
runtime crate: nested runtime/sync/task modules, cfg-gated async functions,
builder structs, generic channel APIs, and impl blocks. They do not depend on an
external Tokio checkout, so the suite stays hermetic in CI.

For real-checkout evidence, run the ignored test with an external Rust crate
root. It queries the provider-owned `patchSafety.target`, selects an
`ast-patch-safe` match, dry-runs against the real checkout without mutation, and
can optionally apply to a temp copy of the selected file. If several matches
share the same query term, set `ASP_AST_PATCH_REAL_TARGET_KIND` and
`ASP_AST_PATCH_REAL_TARGET_NAME` to remove ambiguity:

```sh
ASP_AST_PATCH_REAL_ROOT=/path/to/tokio/tokio \
ASP_AST_PATCH_REAL_PATH=src/runtime/builder.rs \
ASP_AST_PATCH_REAL_QUERY=Builder \
ASP_AST_PATCH_REAL_TARGET_KIND=impl \
ASP_AST_PATCH_REAL_APPLY_TEMP=1 \
cargo test --features cli,search \
  cli::ast_patch_scenarios::cli_ast_patch_real_checkout_query_target_dry_runs_from_env \
  -- --ignored
```

The `tests/fixtures/ast_patch_real_projects/` files store only evidence
metadata: repository commit, query target, compact/exact byte counts,
parser-owned responsibilities, and dry-run/temp-apply receipt events. They do
not store external project source.
