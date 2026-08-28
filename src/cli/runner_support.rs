//! Shared helpers for the Rust harness CLI runner.

use std::env;
use std::path::{Path, PathBuf};

pub(super) fn print_help() {
    println!(
        "asp-rust [--json | --agent-snapshot] [PROJECT_ROOT]\n\
             asp-rust search <view> [ARGS] [PIPE...] [--json] [--package PACKAGE] [PROJECT_ROOT]\n\
             asp-rust query --selector SELECTOR --projection source|callable-skeleton [--workspace PROJECT_ROOT]\n\
             asp-rust check <--changed|--full> [--json] [PROJECT_ROOT]\n\
             asp-rust behavior snapshot --path PATH [--json]\n\
             asp-rust determinism readiness [--include-tests] [--json] [PROJECT_ROOT]\n\
             asp-rust verification <performance-index|stability-index> [--json] [PROJECT_ROOT]\n\
             asp-rust receipt <adapter> [--dry-run] [--json] [PROJECT_ROOT]\n\
             asp-rust proof pilot dependency-graph-acyclicity [--max-nodes N] [--json]\n\
             asp-rust review packet [--receipt-json PATH] [--behavior-json PATH] [--determinism-json PATH] [--proof-json PATH] [--waiver-json PATH] [--json] [PROJECT_ROOT]\n\
             asp-rust evidence graph --review-packet-json PATH [--json] [PROJECT_ROOT]\n\
             asp-rust evidence assurance --evidence-graph-json PATH [--json] [PROJECT_ROOT]\n\
             asp-rust ast-patch <dry-run|apply> --packet <semantic-ast-patch.json|-> [PROJECT_ROOT]\n\
             asp-rust guide [PROJECT_ROOT]\n\
             asp-rust agent doctor [--json] [PROJECT_ROOT]\n\n\
         Runs the default package-level Rust harness.\n\n\
         Compact text is the default agent-facing repair surface.\n\
         Use --json to emit the structured RustHarnessReport audit shape.\n\
          Use --agent-snapshot to emit a low-noise reasoning-tree summary.\n\
          Use search for RFC line-protocol exploration views.\n\
          Use query for exact parser-owned selector projection.\n\
          Use ast-patch dry-run/apply for provider-native structural patch receipts."
    );
}

pub(super) fn print_search_help() {
    println!(
        "asp-rust search prime [--workspace PROJECT_ROOT] [--package PACKAGE]\n\
asp-rust search guide [PROJECT_ROOT]\n\
asp-rust search owner <path-or-owner> [items tests] [--scope SCOPE] [PROJECT_ROOT]\n\
         asp-rust search owner <path-or-owner> items --query SYMBOL [PROJECT_ROOT]\n\
         asp-rust search workspace [--package PACKAGE] [PROJECT_ROOT]\n\
         asp-rust search targets [--package PACKAGE] [PROJECT_ROOT]\n\
asp-rust search deps [dep[/subpath][@version][::api]] [public-api] [PROJECT_ROOT]\n\
asp-rust search dependency-topology --json [--workspace PROJECT_ROOT]\n\
asp-rust projection <relative-owner> --json --workspace PROJECT_ROOT\n\
asp-rust search env [toolchain|cfg] [PROJECT_ROOT]\n\
asp-rust search compare env stable nightly [PROJECT_ROOT]\n\
asp-rust search code comments [--owner OWNER] [PROJECT_ROOT]\n\
asp-rust search extension <extension-id> [PROJECT_ROOT]\n\
asp-rust search policy <rule-id-or-alias> [owner tests] [PROJECT_ROOT]\n\
asp-rust search query <code-shaped-query> [owner tests] [PROJECT_ROOT]\n\
asp-rust search features [feature] [cfg owners tests] [PROJECT_ROOT]\n\
         asp-rust search dependency <crate-or-import-or-package> [items public-api docs tests] [PROJECT_ROOT]\n\
         asp-rust search <symbol|callsite|import|cfg|pattern|docs|docs-use|api> <query> [PROJECT_ROOT]\n\
asp-rust search <owner|dependency|tests> --query-set TERM [--query-set TERM...] [PROJECT_ROOT]\n\
         asp-rust search public-external-types [--dependency DEP] [PROJECT_ROOT]\n\
         Emits compact RFC line protocol for deterministic agent exploration.\n\
         Search discovers owner candidates; exact item identity belongs to query-item packets.\n\
         Compact text is the default; --json wraps the same packet for tools.\n\
         RFC controls accepted here: --trace, --explain, --view graph|hits|both|seeds,\n\
         --depth N, --dir out|in|both, --edge LIST, --item-slice, --dependency DEP,\n\
         --seeds N, --query-set TERM, --query SYMBOL, --lines."
    );
}

pub(super) fn print_check_help() {
    println!(
        "asp-rust check --changed [--json] [PROJECT_ROOT]\n\
         asp-rust check --full [--json] [PROJECT_ROOT]\n\n\
         Runs the policy surface and renders compact findings by default."
    );
}

pub(super) fn print_agent_help() {
    println!(
        "asp-rust agent doctor [--json] [PROJECT_ROOT]\n\n\
         Hook install/runtime is owned by semantic-agent-hook in the root toolchain.\n\
         Use --json to emit the semantic-language registry contract."
    );
}

pub(super) fn moved_agent_action(action: &str) -> String {
    if action == "guard" {
        return "asp-rust agent guard moved to asp hook; use asp hook --client codex pre-tool --emit decision".to_string();
    }
    format!("asp-rust agent {action} moved to asp hook; use asp hook {action} --client codex")
}

pub(super) fn print_agent_doctor(project_root: &Path, _client: Option<&str>) {
    println!(
        "[agent-doctor] status=ok provider=asp-rust runtime=semantic-agent-hook project={}",
        project_root.display()
    );
}

pub(super) fn print_guide(_project_root: &Path) {
    print!(
        r#"[agent-guide] lang=rust provider=asp-rust protocol=agent-guide.v1 root=.
|contract intent=agent-owned harness=typed-selectors no=--goal,--intent,nl-planning
|surface search purpose=tool-map output=search-guide code=false
|surface query purpose=exact-selector-projection output=pure-source|callable-skeleton
|surface check purpose=verification output=receipt|compact-findings
|surface patch purpose=mutation authority=agent-core|apply_patch|ast-patch
|catalog reasoningProfiles=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg entries=owner-query,query-deps,owner-tests,finding-frontier,feature-cfg routes=path,read-frontier

|flow bootstrap start="search guide ." then="choose evidence-state route; prime only when owner map unknown" next="use search-guide command=search reasoning <profile> --owner/--query/--dependency ... --view seeds"
|flow exact-read start="search owner <owner-path> items --query <symbol> --view seeds" then="query --selector <exact-structural-selector> --projection source --workspace <workspace-root>"
|flow wide-read-protection trigger="raw-source-denied" next="search owner <owner-path> items --workspace <workspace-root> --view seeds" output=owner-item-frontier code=false

|cmd prime=asp rust search prime --workspace <workspace-root> --view seeds condition=owner-map-unknown
|cmd pipe=asp rust search pipe '<term>' --workspace <workspace-root> --view seeds condition=ambiguous-query
|cmd query-source=asp rust query --selector <exact-structural-selector> --projection source --workspace <workspace-root>
|cmd query-skeleton=asp rust query --selector <exact-structural-selector> --workspace <workspace-root> --projection callable-skeleton
|cmd evidence-graph=asp rust evidence graph --review-packet-json <semantic-review-packet.json> --json <workspace-root>
|cmd evidence-analyze=asp rust evidence analyze --evidence-graph-json <semantic-evidence-graph.json> --json <workspace-root>

|refer search-guide="search guide ." use=low-frequency-tool-map
|refer query-guide="query guide ." use=exact-projection-contract

|rule search-no-code default=true reason=avoid-inline-code-token-bloat
|rule query-source-stdout pure=true explicitProjection=required
|rule displayLineRange/sourceLocatorHint are display hints; execute structural selectors or owner/symbol routes, not line ranges
|avoid raw-read,manual-window-scan,inline-code-in-search,broad-lexical,search-json-in-prompt,repeat-wide-read
"#
    );
}

pub(super) fn is_command(args: &[std::ffi::OsString], command: &str) -> bool {
    args.first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| arg == command)
}

pub(super) fn search_view_requires_query(view: &str) -> bool {
    matches!(
        view,
        "owner"
            | "policy"
            | "code"
            | "env"
            | "compare"
            | "extension"
            | "dependency"
            | "tests"
            | "symbol"
            | "callsite"
            | "import"
            | "query"
            | "cfg"
            | "pattern"
            | "docs"
            | "docs-use"
            | "api"
            | "reasoning"
            | "semantic-facts"
    )
}

pub(super) fn search_view_accepts_optional_query(view: &str) -> bool {
    matches!(view, "deps" | "features")
}

pub(super) fn search_view_supports_query_set(view: &str) -> bool {
    matches!(view, "owner" | "dependency" | "tests")
}

pub(super) fn is_known_search_view(view: &str) -> bool {
    matches!(
        view,
        "prime"
            | "guide"
            | "workspace"
            | "targets"
            | "deps"
            | "env"
            | "compare"
            | "features"
            | "policy"
            | "code"
            | "extension"
            | "owner"
            | "dependency"
            | "tests"
            | "lexical"
            | "symbol"
            | "callsite"
            | "import"
            | "query"
            | "cfg"
            | "patterns"
            | "pattern"
            | "docs"
            | "docs-use"
            | "api"
            | "public-external-types"
            | "reasoning"
            | "semantic-facts"
            | "dependency-topology"
            | "dependency-topology-metadata"
            | "ingest"
    )
}

pub(super) fn is_search_pipe(value: &str) -> bool {
    matches!(
        value,
        "owner"
            | "owners"
            | "usage"
            | "items"
            | "tests"
            | "examples"
            | "benches"
            | "docs"
            | "docs-use"
            | "api"
            | "public-external-types"
            | "public-api"
            | "cfg"
            | "features"
            | "dependents"
    )
}

pub(super) fn discover_rust_project_root() -> Result<PathBuf, String> {
    let current =
        env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))?;
    rust_project_root_for_path(&current)
}

pub(super) fn rust_project_root_for_path(path: &Path) -> Result<PathBuf, String> {
    crate::parser::cargo_project_root_for_path(path)
}

pub(super) fn rust_package_root_for_path(path: &Path) -> Result<PathBuf, String> {
    crate::parser::cargo_package_root_for_path(path)
}

pub(super) fn parse_usize_option(option: &str, value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("expected integer value after {option}"))
}

pub(super) fn split_csv_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
