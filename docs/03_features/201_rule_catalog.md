# Rule Catalog

The harness exposes deterministic rule metadata through compact catalog
functions:

- `rust_rule_pack_descriptors()`
- `rust_syntax_rules()`
- `rust_project_policy_rules()`
- `rust_modularity_rules()`
- `rust_agent_policy_rules()`

## Default Rule Packs

Default project execution runs these packs:

1. `rust.syntax`
2. `rust.project_policy`
3. `rust.modularity`
4. `rust.agent_policy`

The package self-applies this same default execution order in its own tests.
Policy drift should be repaired in the crate before the rule catalog is treated
as ready for downstream use.

`rust_rule_pack_descriptors()` returns the same pack order with stable pack ids,
version labels, searchable domains, and default modes. The first three packs are
`blocking`; `rust.agent_policy` is `advisory`.

## Blocking Rules

- `RUST-SYN-R001`: Rust source must parse through `syn`
- `RUST-AGENT-PROJECT-001`: root test file must be an explicit harness entry point
- `RUST-AGENT-PROJECT-002`: directory under `tests/` must be an allowed suite directory
- `RUST-AGENT-PROJECT-003`: source tests must be externalized instead of inline
- `RUST-AGENT-PROJECT-004`: external test mount must point to an existing `tests/unit` file
- `RUST-AGENT-PROJECT-005`: large test leaf should split into a folder-first suite
- `RUST-AGENT-PROJECT-006`: retired root Cargo test target harness gate should migrate to the cargo-check build gate
- `RUST-AGENT-PROJECT-007`: root Cargo test target should stay a thin harness aggregate
- `RUST-AGENT-PROJECT-008`: root Cargo test target modules should use explicit suite `#[path]` mounts
- `RUST-AGENT-PROJECT-009`: retired source cargo-test harness gate should migrate to the cargo-check build gate
- `RUST-AGENT-PROJECT-010`: Rust-native performance verification bindings must have a runnable `harness = false` Cargo bench target
- `RUST-AGENT-PROJECT-011`: cargo-check build gates must run with explicit verification config
- `RUST-AGENT-PROJECT-012`: harness-enabled packages must mount the build-time harness gate for `cargo check`
- `RUST-AGENT-PROJECT-013`: custom harness source/test scope paths must carry an explicit explanation
- `RUST-AGENT-PROJECT-014`: Cargo-backed harness scopes must not be silently removed
- `RUST-AGENT-PROJECT-015`: retired cargo-test advice allowance must carry an explicit explanation
- `RUST-AGENT-PROJECT-016`: retired cargo-test compatibility gates must run with explicit verification config
- `RUST-MOD-R001`: `mod.rs` should stay interface-only with external module declarations and re-exports
- `RUST-MOD-R002`: oversized source file should split by responsibility, including private implementation piles
- `RUST-MOD-R003`: native `use` trees containing `super::super` should use `crate::...` owner/facade imports
- `RUST-MOD-R004`: `lib.rs` should stay a crate facade with module declarations, re-exports, and parser-proven boundary macros only
- `RUST-MOD-R005`: `src/main.rs` and `src/bin` entrypoints should stay thin
- `RUST-MOD-R006`: root `build.rs` should stay a thin Cargo build-script entrypoint
- `RUST-MOD-R007`: a module owner should not have both `foo.rs` and `foo/mod.rs`
- `RUST-MOD-R008`: source modules should not hide implementation in inline `mod name { ... }` blocks
- `RUST-MOD-R009`: scanned source files must be reachable from a crate or binary module tree
- `RUST-MOD-R010`: Rust glob imports should be replaced with explicit owner imports

`lib.rs`, `mod.rs`, `src/main.rs`, `src/bin` entrypoints, and root `build.rs`
are special Rust ownership files. They are treated as
facades/interfaces/adapters instead of implementation owners, so they receive
targeted modularity checks before generic source-size policy matters.
The module tree also has one source-layout owner per module: `foo.rs` and
`foo/mod.rs` should not coexist because that leaves repair agents with two
competing files for the same logical owner.
Inline implementation modules collapse the reasoning tree back into a single
file, so regular source modules should use external file-backed child modules.
Files that are not reachable through `mod` declarations are also blocked:
they look like source to search tools but are invisible to the Rust module tree
and therefore unsafe for agents to treat as live owners.

Root Cargo test target files under `tests/*.rs` are also special entrypoints.
They should mount external suite modules only, while test bodies and helpers
live under `tests/unit`, `tests/integration`, or another documented suite
directory. Module mounts from those root targets must use explicit
`#[path = "suite/file.rs"]` attributes so Rust's implicit module lookup does not
create unclear root-level test structure.

The primary downstream harness gate is the build-time gate. Current policy is
parser-native: it needs Cargo manifest facts and Rust syntax facts, not runtime
test evaluation. A complete build gate has both a Cargo build-dependency on
`asp-rust` and a root `build.rs` call to
`assert_asp_rust_cargo_check_clean_from_env_with_config(...)` or another
build-gate assertion:

```toml
[build-dependencies]
asp-rust = { git = "https://github.com/tao3k/asp-rust", branch = "main" }
```

```rust
fn main() {
    let config = asp_rust::default_asp_rust_config()
        .with_verification_profile_hint(
            asp_rust::RustVerificationProfileHint::new(
                "src/lib.rs",
                [asp_rust::RustOwnerResponsibility::PublicApi],
            ),
        );
    asp_rust::assert_asp_rust_cargo_check_clean_from_env_with_config(
        &config,
    );
}
```

`RUST-AGENT-PROJECT-011` keeps the cargo-check gate from silently running the default
empty verification policy: use the configured form to declare profile hints,
explicit suppressions, receipts, waivers, or skill bindings for the
Agent-facing verification surface. `RUST-AGENT-PROJECT-012` is the Agent-facing closure rule: when a
harness-enabled package lacks the build-dependency, lacks root `build.rs`, has a
root `build.rs` that omits the harness call, or calls the gate without the build
dependency, `cargo check` prints a compact finding that tells the next Agent
exactly which configuration surface to add. `RUST-AGENT-PROJECT-006` and
`RUST-AGENT-PROJECT-009` are migration warnings: if a harness-enabled package still
mounts parser-native policy through a root test target or source cargo-test
macro, the compact finding tells the next Agent to move that policy to
`[build-dependencies]` plus root `build.rs`.

Cargo-test policy is intentionally narrower. Use it only for behavior that is
about the test layer itself: retired source gate configuration, explicit advice
allowance, or future checks that consume runtime test/verification receipts.
`RUST-AGENT-PROJECT-015` and `RUST-AGENT-PROJECT-016` are therefore cargo-test compatibility
rules. Parser-native structure, module ownership, import clarity, scope
coverage, and verification planning reminders belong to cargo check.

Verification policy wiring also has a physical Cargo target check. When a
project configures an active Rust-native performance binding such as
`rust-verification-performance@criterion`, `@divan`, or `@iai-callgrind`,
`RUST-AGENT-PROJECT-010` requires a real `[[bench]]` target with `harness = false` and
an existing bench source file. This keeps the compact verification plan from
claiming a performance skill exists while Cargo has no runnable benchmark
target for the Agent to execute.

Harness scope configuration is policy-governed. Cargo manifest facts and
conventional Cargo layout form the baseline coverage: `src`, explicit
`[lib]`/`[[bin]]` target roots, `tests`, explicit `[[test]]` target roots,
`examples`, explicit `[[example]]` targets, `benches`, explicit `[[bench]]`
targets, and root `build.rs`. A build-script gate therefore cannot shrink the
scan surface just by passing a smaller config. `RUST-AGENT-PROJECT-013` requires every
custom source or test scope path to have a non-empty explanation, preferably by
using `AspRustConfig::with_source_path(path, explanation)` or
`with_test_path(path, explanation)`. `RUST-AGENT-PROJECT-014` catches attempts to
remove Cargo-backed `src`, `tests`, or manifest-declared test coverage without
a matching explanation through `with_source_path_excluded(path, explanation)`,
`with_test_path_excluded(path, explanation)`, or
`with_tests_excluded(explanation)`.

## Agent Advice Rules

`AGENT-*` rules are `Info` findings. They are designed as repair hints for LLMs
and are not blocking by default.
Build-script gates add one Agent-facing layer on top of that library policy:
the default `assert_asp_rust_cargo_check_clean_from_env_with_config(...)`
assertion fails on compact agent advice so the repair contract is visible during
`cargo check`. This does not change rule severity or JSON metadata. Use
`with_cargo_check_advice_allow_explanation(...)` for an explicit cargo-check
policy waiver, or configure the relevant rule/pack when the crate has a clearer local
responsibility model. Source-embedded cargo-test gates keep the same advice
behavior for compatibility, but they are no longer the preferred downstream
entrypoint.
Rules that ask an agent to add Rust doc comments require Clippy-compatible
Markdown: use `clippy::doc_markdown` style and wrap API names, rule IDs, command
names, and other literal identifiers in backticks.

- `RUST-AGENT-DOCS-MODULE-001`: public module surface lacks an intent doc
- `RUST-AGENT-DOCS-PUBLIC-002`: public item lacks a doc comment
- `RUST-AGENT-SOURCE-NAMESPACE-003`: namespace path repeats a segment, including ordinary Rust file stems
- `RUST-AGENT-API-NAME-004`: public item name appears in multiple modules
- `RUST-AGENT-API-FACADE-005`: facade re-exports too many names without a tighter owner surface
- `RUST-AGENT-SOURCE-MODULE-006`: public module name is a generic bucket such as `utils`, `common`, `helpers`, or `shared`
- `RUST-AGENT-SOURCE-PATH-007`: source module file or directory path uses a generic bucket segment
- `RUST-AGENT-DOCS-BRANCH-008`: branch module owns multiple resolved child edges without a reasoning-tree intent doc
- `RUST-AGENT-OWNER-GRAPH-009`: owner dependency graph contains a local owner cycle
- `RUST-AGENT-OWNER-BOUNDARY-010`: owner branch imports another owner's leaf implementation module
- `RUST-AGENT-DOCS-OWNER-FANOUT-011`: branch module fans out to three or more local owners without an intent doc
- `RUST-AGENT-API-TYPE-012`: public semantic identifier parameter uses a primitive string or integer type
- `RUST-AGENT-API-ERROR-013`: public error boundary uses an application error type such as `anyhow::Result`
- `RUST-AGENT-TEST-SUPPORT-014`: test support facade re-exports a name that is not used locally or consumed through the support surface
- `RUST-AGENT-CFG-PUBLIC-015`: public function hides an algorithm behind nested control flow
- `RUST-AGENT-CFG-PUBLIC-016`: public function owns a broad linear algorithm surface without named steps
- `RUST-AGENT-ITER-PUBLIC-017`: public function manually spells simple iterator boilerplate loops
- `RUST-AGENT-API-FLAGS-018`: public function exposes multiple `bool` or `Option<bool>` flag parameters
- `RUST-AGENT-API-PARAMETERS-019`: public function exposes a broad positional parameter surface
- `RUST-AGENT-DATA-FIELD-020`: public data struct exposes multiple primitive semantic fields
- `RUST-AGENT-DATA-ENUM-PAYLOAD-021`: public enum variant exposes multiple primitive semantic payload fields
- `RUST-AGENT-DATA-BOUNDS-022`: public generic data type carries duplicated derivable trait bounds
- `RUST-AGENT-API-SHAPE-023`: public API exposes an anonymous tuple of primitive semantic values
- `RUST-AGENT-DATA-ENUM-TUPLE-024`: public enum tuple variant exposes anonymous primitive semantic payload
- `RUST-AGENT-CFG-IMPL-025`: implementation function nests traversal scaffolding
- `RUST-AGENT-ITER-IMPL-026`: implementation function manually spells simple iterator boilerplate loops
- `RUST-AGENT-API-TYPE-ALIAS-027`: public semantic type alias hides a primitive carrier
- `RUST-AGENT-DATA-STATE-028`: public data model exposes a stringly state, status, kind, mode, type, tag, phase, or category field

## Rendered Diagnostic Policy

Rendered findings intentionally avoid large JSON payloads, human audit headers,
and decorative code-frame markers such as `,-[` or pointer art. The primary
repair surface is compact text for agents:

1. stable rule id
2. `@ path:line:column` locator
3. one `fix:` line
4. source line when available
5. one `Help:` line from the concrete finding summary
6. one `Contract:` line from the rule requirement

When findings exist, compact text starts directly at those finding blocks. It
does not prepend global `Source`, `Files`, `Parsed`, `Issues`, `advice`, or
`No blocking issues` sections; those counters are audit metadata, not repair
instructions. A fully clean render is the only case that emits a global status,
and that status is just `[ok] rust`.

This format is part of the Agent contract. Paths may be long, especially in
worktrees and CI sandboxes, so the renderer uses a single `@ path:line:column`
locator instead of human-oriented code frames that wrap poorly and obscure the
repair action. The `fix:` line names the intended edit, `Help:` explains the
concrete parser fact, and `Contract:` states the stable rule expectation.

`render_asp_rust()` includes advice by default. A report with only
`Info` findings is still clean, but its advice remains visible as ordinary
finding blocks. Use `render_asp_rust_advice()` when a caller wants
only the non-blocking repair hints.

Structured consumers should use `render_asp_rust_json()` or the
serializable `AspRustReport` for JSON output instead of parsing the compact
text render.

The compact text and JSON render contracts are covered by repository snapshots
under `tests/unit/snapshots`. Every `RUST-MOD-*` policy also has an
`insta` compact-diagnostic snapshot generated from a real harness fixture, so
changes to structural `Help:` and `Contract:` wording are reviewed per rule.
Every `AGENT-*` policy has the same snapshot treatment for LLM-facing advice,
including multi-finding ambiguity cases such as duplicated public names.

## Path Clarity Policy

Path clarity rules follow Rust syntax and project scope instead of raw text
searches. `RUST-MOD-R003` and `RUST-MOD-R010` consume native `use` tree facts
from `src/parser/`, so grouped uses such as `use super::{super::Owner}` and
glob imports such as `use super::*` are caught while comments and strings are
ignored. The parser also records whether a `use` statement is inside an inline
`#[cfg(test)]` module or a conventional `tests/` root, so glob findings can name
test context without weakening the default no-glob harness contract.
The preferred repair for `super::super` is a stable `crate::...` owner/facade
import. If the target is a leaf implementation, expose the needed API through
the owner facade first instead of adding another relative hop. The concrete
`crate::...` candidate is derived by the reasoning-tree parser from the current
module namespace and inline-module context; the policy renderer only consumes
that parser fact.

`RUST-AGENT-DOCS-MODULE-001`, `RUST-AGENT-DOCS-PUBLIC-002`, `RUST-AGENT-API-NAME-004`, `RUST-AGENT-API-FACADE-005`, `RUST-AGENT-SOURCE-MODULE-006`,
`RUST-AGENT-DOCS-BRANCH-008`, and `RUST-AGENT-API-TYPE-012` through `RUST-AGENT-DATA-STATE-028` consume native facts from
`src/parser/`, including file-level inner doc attributes, public names, public
item doc attributes, public re-export groups, public function parameters, public
function return types, public and internal function or method control-flow
shape, public data-struct field shape, public enum named and tuple variant
payload shape, public generic data-type bounds, public type aliases, public
anonymous tuple API surfaces, support facade re-export names, support-surface
path references, and resolved reasoning-tree child edges.
`RUST-AGENT-SOURCE-NAMESPACE-003` evaluates the default
package harness surface,
including `src/` and `tests/`. It treats
normal Rust file stems as namespace segments, so both `src/domain/domain.rs` and
`tests/unit/unit/helper.rs` produce advisory path clarity findings.
`RUST-AGENT-API-NAME-004` separately reports duplicated public item names across source
modules as non-blocking ambiguity advice. `RUST-AGENT-SOURCE-MODULE-006` catches public generic
bucket modules such as `pub mod utils;`; those names are often where
LLM-generated code loses a real owner boundary without violating Rust syntax,
rustfmt, or Clippy. `RUST-AGENT-SOURCE-PATH-007` catches the same drift at the file system
level, such as `src/helpers.rs` or `src/common/mod.rs`, even when the module is
private. `RUST-AGENT-OWNER-GRAPH-009`, `RUST-AGENT-OWNER-BOUNDARY-010`, and `RUST-AGENT-DOCS-OWNER-FANOUT-011` consume parser-derived
owner dependency edges. They stay advisory because Rust permits these import
shapes, but they are high-signal LLM repair risks: circular owner reasoning,
reaching into another owner's leaf module, and fan-out branches without local
intent documentation. `RUST-AGENT-API-TYPE-012` is derived from type-driven Rust practice:
when a public function exposes a parameter named `id` or `*_id` as `String`,
`&str`, an integer primitive, or `Option` around those primitive carriers, the
harness asks for an owner-named newtype or an explicit primitive-boundary
rationale. Clippy cannot know that a primitive is a semantic identifier, but the
parser can expose the native signature fact for agent repair. `RUST-AGENT-API-ERROR-013` is
derived from Rust error-boundary practice: public library functions should expose
typed recovery contracts rather than application-level catch-all errors such as
`anyhow::Result`, `eyre::Result`, or `Result<_, Box<dyn Error>>`. The rule stays
advisory because binaries and application crates may choose that boundary
intentionally.
`RUST-AGENT-TEST-SUPPORT-014` is narrower than Clippy's ordinary unused-import surface: it only
looks at `tests/**/support.rs` re-exports and asks agents to remove names that
are neither used by the support helpers nor imported or referenced through that
exact `support::Name` surface elsewhere in the package. The support surface is
resolved from parser-derived module namespaces, so a consumed name in
`tests/unit/alpha/support.rs` does not clear the same unused name in
`tests/unit/beta/support.rs`. This catches broad support facades left by LLM
repairs without second-guessing normal private imports.
`RUST-AGENT-CFG-PUBLIC-015` and `RUST-AGENT-CFG-PUBLIC-016` use parser-owned public function and public method
control-flow facts: source line, line span, statement count, largest block
width, branch count, loop count, match count, literal dispatch chain count,
nesting depth, loop nesting depth, and test context. They intentionally do not
enforce rustfmt, naming, complexity metrics, or Clippy style. The goal is
narrower: show an agent where a public algorithm is hard to edit because its
branch structure is hidden in nested control flow, literal `if`/`else if`
dispatch ladders, or one broad linear block. Match-based dispatch, guard
clauses, typed dispatch, and small named pipeline steps are accepted shapes
because they make the reasoning tree explicit before the next edit. The literal
dispatch signal follows Rust's native `match` model from the Book and
Reference: a `match` compares one scrutinee against a series of patterns, which
is exactly the intent an agent loses when LLM code repeats `kind == "..."`
across a public branch ladder.
`RUST-AGENT-ITER-PUBLIC-017` is the Rust native-iterator idiom layer. It is backed by
parser-owned loop facts for simple `for` bodies that manually collect into a
mutable collection, return a boolean predicate answer, increment a count,
accumulate a numeric value, or repeatedly pass over the same simple iterator
source. The rule points agents toward Rust iterator adapters and consumers such
as `map`, `filter`, `filter_map`, `collect`, `sum`, `count`, `any`, and `all`,
or toward a named iterator pipeline helper when a single chain would be too
dense. Repeated simple passes are advisory rather than blocking: sometimes two
passes are clearer, but a public function that performs several small scans over
the same input is often LLM boilerplate that should become a named pipeline or
one explicit accumulator step. It stays conservative: deeply nested algorithm
shapes are left to `RUST-AGENT-CFG-PUBLIC-015`, broad flat procedures are left to `RUST-AGENT-CFG-PUBLIC-016`,
and explicit loops remain valid for effects, state machines, debuggability, or
measured performance work. This mirrors the Rust Book's guidance that iterators
express high-level ideas at low-level performance, the standard `Iterator`
consumer surface, and the Rust Performance Book's narrower performance notes
around iteration, without turning the harness into a blanket "prefer iterators
over every loop" lint.
`RUST-AGENT-CFG-IMPL-025` extends the same parser-owned control-flow facts to internal
implementation functions and `impl` methods. It is aimed at LLM-generated
receipt and report walkers such as `if has_failures { for repo { for query { if
!query.passed { ... }}}}`: Rust allows the code, and Clippy may have nothing to
say, but the algorithm boundary is invisible to the next repair agent. The rule
fires only on non-test internal functions with nested loop traversal and guard
branches, then asks for named iterator, predicate, or receipt-processing helpers
instead of more raw loop scaffolding. Small named helper pipelines and public
API algorithm rules stay separate.
`RUST-AGENT-ITER-IMPL-026` covers the flatter internal companion case: a private function or
method whose loop body only collects, filters, counts, sums, answers a predicate,
or repeats a simple scan over the same iterator source. It reuses the same
parser-owned iterator facts as `RUST-AGENT-ITER-PUBLIC-017`, but keeps the message scoped to
implementation helpers instead of public API shape. The rule deliberately skips
functions already reported by `RUST-AGENT-CFG-IMPL-025`, so an agent receives one compact
piece of advice: deep traversal should become a named traversal boundary, while
flat boilerplate should become iterator adapters or a named iterator helper.
`RUST-AGENT-API-FLAGS-018` is the public flag-surface layer. It is backed by parser-owned
signature facts for `bool`, `&bool`, `Option<bool>`, and referenced optional
booleans. The rule only fires when one public function exposes multiple flag
parameters, because that is where LLM-generated Rust tends to hide modes in
branch-heavy code. It follows Rust API Guidelines `C-CUSTOM-TYPE`: arguments
should convey meaning through deliberate types rather than raw `bool` or
`Option` values. The advice is advisory and API-shaped, not a Clippy style
replacement: use an enum when one mode is selected, a newtype when one boolean
has domain meaning, or a config struct when several independent toggles are
truly part of the public contract.
`RUST-AGENT-API-PARAMETERS-019` is the public positional-surface layer. It is backed by
parser-owned public signature facts, including inherent `impl` methods such as
constructors. The rule fires when one public function exposes five or more
positional parameters outside test context. Rust allows that API shape, but it
is a high-noise agent repair surface because Rust has no named or default
function parameters: preserving the order, optionality, and cross-parameter
meaning requires re-reading callers. The advice follows the Rust builder/config
practice used for broad construction and option surfaces: prefer a named
config/request type or a builder when the public contract has enough knobs that
positional parameters stop carrying clear intent.
`RUST-AGENT-DATA-FIELD-020` moves the same type-safety concern from function signatures to
public data models. It is backed by parser-owned public struct field facts and
fires when a public struct exposes several semantic primitive fields such as
`*_id`, `*_token`, `*_path`, `*_url`, `*_ms`, or boolean mode fields. This is not
a style lint: public DTOs and config structs are allowed, but when many
semantic values remain raw `String`, integer, or `bool` fields, an agent tends
to extend the same primitive model instead of preserving invariants. The advice
follows Rust API Guidelines `C-NEWTYPE` and `C-CUSTOM-TYPE`: create named domain
types for values whose interpretation matters, or explicitly document that this
is a raw transport boundary.
`RUST-AGENT-API-TYPE-ALIAS-027` closes the weak-alias escape hatch for that same boundary. A public
alias such as `pub type UserId = String` gives an agent a named symbol but does
not create a Rust type boundary, so later repairs can still mix identifiers,
tokens, paths, durations, byte counts, and flags across call sites. The parser
records public type aliases and their primitive carriers; the rule only fires
when the alias name looks semantic, and asks for a tuple newtype or named struct
instead of a primitive alias.
`RUST-AGENT-DATA-STATE-028` catches another stringly data-model shape that is especially noisy
for agents: public fields named like `status`, `state`, `kind`, `mode`, `type`,
`tag`, `phase`, or `category` whose carrier is `String` or `Option<String>`.
Rust permits these fields and Clippy cannot know whether they
are closed state catalogs, but LLM repairs tend to extend them with literal
string comparisons. The rule stays advisory and parser-backed, asking for an
enum, newtype, or typed catalog boundary when the public model exposes that
state surface.
`RUST-AGENT-DATA-ENUM-PAYLOAD-021` applies the same data-model boundary to public enum variants with
named payload fields. It does not count enum variants, require
`#[non_exhaustive]`, or judge closed state catalogs. Instead, it catches event,
command, and state variants that expose multiple semantic primitive payload
fields such as `user_id: String` and `request_id: String`. Those variants are
where agents often extend raw event state instead of preserving payload
invariants. The repair direction is to use named domain types for semantic
values or move the payload into a named struct when the variant is carrying a
real event/request contract.
`RUST-AGENT-DATA-BOUNDS-022` covers public generic data-type bounds. It is backed by
parser-owned generic parameter and `where` clause facts for public structs and
enums. The rule follows Rust API Guidelines `C-STRUCT-BOUNDS`: bounds such as
`Clone`, `Debug`, `Default`, `Serialize`, and `Deserialize` should not be placed
on the data type definition unless the structure itself truly requires them.
Putting those bounds on the type makes every consumer satisfy them and turns a
future derive or formatting need into a public API commitment. The repair
direction is to keep the data type generic and place bounds on derived impls,
inherent impls, or methods that actually use the capability.
`RUST-AGENT-API-SHAPE-023` covers public tuple API surfaces such as
`pub fn load(cursor: (String, usize, bool)) -> Result<(String, usize), Error>`.
It follows Rust API Guidelines `C-CUSTOM-TYPE` and `C-NEWTYPE`: public API
arguments and return values should convey semantic meaning through named types
instead of raw primitive bundles. The rule stays narrower than generic type
complexity checks: it only reports parser-confirmed public tuple parameters or
return values that bundle at least two primitive semantic values, including
`Option<(...)>` and `Result<(...)>`. The repair direction is to replace the
tuple with a named request, response, enum, or newtype that gives agents stable
field intent without reading every call site.
`RUST-AGENT-DATA-ENUM-TUPLE-024` covers the enum version of the same ambiguity:
`pub enum Event { Loaded(String, usize, bool) }`. Tuple variants are native
Rust, but when a public event or command variant bundles several primitive
semantic values without names, an agent cannot preserve payload intent from
syntax alone. The rule is deliberately narrower than enum-design lints: it only
reports public tuple variants with at least two primitive semantic payload
positions and ignores test context. The repair direction is to use named fields,
a named payload struct, or domain newtypes so the variant remains explicit
without forcing a large enum redesign.

## Reasoning Tree Policy

The harness treats a Rust project as a reasoning tree for agents: crate
facades and branch modules point to owner modules, and owner modules point to
leaf implementation files. Parser reasoning facts also summarize each owner
branch's import roots (`crate`, `self`, `parent`, `external`, plus
glob/deep/test markers) and local owner dependency edges for compact agent
snapshots. The reasoning tree also exposes package-level owner dependency
edges, such as `src/lib.rs --crate--> src/domain.rs`, while retaining source
line and test-context metadata. `RUST-MOD-R008` keeps those branches file-backed
by rejecting inline source modules outside special entrypoints and `#[cfg(test)]`
test modules. `RUST-MOD-R009` then verifies parser-owned module-tree facts: a
scanned source file must be reachable from crate roots or binary roots through
external `mod` declarations, explicit `#[path]` mounts, or literal `include!`
source shards. `RUST-AGENT-DOCS-BRANCH-008` adds non-blocking advice when a branch file has
multiple resolved child edges without a `//!` intent doc, because agents need a
local navigation summary before they choose which subtree to edit. Those `//!`
intent docs should follow `clippy::doc_markdown` style so harness-generated
repair prompts do not teach a comment style that Clippy will later reject.
Dependency graph agent rules ignore edges observed only inside `#[cfg(test)]`
context.
