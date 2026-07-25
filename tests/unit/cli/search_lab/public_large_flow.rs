use tempfile::TempDir;

use super::public_fixtures::{
    write_public_codex_web_search_workspace, write_public_tokio_bytes_fixture,
};
use super::{FORBIDDEN_FLOW_PATTERNS, assert_lab_packet};
use crate::cli::support::{run_search, run_search_with_stdin};

fn rg_json_absolute_position_field() -> String {
    ["absolute", "off", "set"].join("_")
}

#[test]
fn public_large_tokio_bytes_flow_connects_prime_to_dependency_api_and_docs_axes() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_public_tokio_bytes_fixture(root);

    let prime = run_search(root, &["prime"]);
    assert_lab_packet(
        "public_tokio_bytes_prime",
        &prime,
        42,
        &[
            "[search-prime] mode=package package=.",
            "|package . t=lib,test dep=bytes",
            "|feature io-util dep=bytes next=features:io-util",
            "|dep bytes import=bytes pkg=bytes version=^1 kind=normal opt=true source=manifest manager=cargo",
            "|api-candidate RuntimeFrame reason=public-item owner=src/io/mod.rs",
            "|test-surface tests=tests next=tests",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let prime_seeds = run_search(root, &["prime", "--view", "seeds", "--seeds", "6"]);
    assert_lab_packet(
        "public_tokio_bytes_prime_seeds",
        &prime_seeds,
        10,
        &[
            "[search-prime] root=.",
            "F=feature:feature(io-util)!features",
            "C=cfg:cfg(feature:io-util)!cfg",
            "frontier=F.features",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let feature_flow = run_search(root, &["features", "io-util", "cfg", "owners", "tests"]);
    assert_lab_packet(
        "public_tokio_bytes_feature_flow",
        &feature_flow,
        18,
        &[
            "[search-features] q=io-util pkg=. feat=1 dep=1 cfg=1",
            "|feature io-util enables=bytes source=manifest manager=cargo",
            "|dep bytes import=bytes pkg=bytes version=^1 kind=normal opt=true source=manifest manager=cargo",
            "|cfg feature:io-util declared_in=features expr=cfg(feature=\"io-util\")",
            "|owner src/lib.rs hit_kind=feature locations=",
            "|owner src/io/mod.rs hit_kind=feature locations=",
            "|test tests/io_util.rs functions=1 owner=src/io/mod.rs",
            "|next cfg:io-util,text:io-util(scope=src),tests",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let dependency_flow = run_search(
        root,
        &[
            "dependency",
            "bytes",
            "items",
            "public-api",
            "docs",
            "tests",
        ],
    );
    assert_lab_packet(
        "public_tokio_bytes_dependency_flow",
        &dependency_flow,
        24,
        &[
            "[search-dependency] q=bytes pkg=. dep=1 own=1 api=",
            " item=",
            " docs=",
            " tests=1",
            "|owner src/io/mod.rs hit_kind=dependency locations=",
            "|item RuntimeFrame kind=struct",
            "|api src/io/mod.rs line=4 dep=bytes kind=struct name=RuntimeFrame",
            "|api src/io/mod.rs line=10 dep=bytes kind=fn name=split_buf",
            "|test tests/io_util.rs functions=1 owner=src/io/mod.rs",
            "|next deps:bytes,import:bytes,docs-use:bytes,crate-source:bytes,tests",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let external_types = run_search(root, &["public-external-types", "--dependency", "bytes"]);
    assert_lab_packet(
        "public_tokio_bytes_external_type_flow",
        &external_types,
        12,
        &[
            "[search-public-external-types] pkg=. dep=1 hit=",
            "|external-type src/io/mod.rs:4 dep=bytes surface=field:payload item=RuntimeFrame type=Bytes",
            "|external-type src/io/mod.rs:4 dep=bytes surface=field:scratch item=RuntimeFrame type=BytesMut",
            "|external-type src/io/mod.rs:10 dep=bytes surface=return item=split_buf type=impl Buf",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let docs_use = run_search(root, &["docs-use", "bytes::Buf"]);
    assert_lab_packet(
        "public_tokio_bytes_docs_use_flow",
        &docs_use,
        10,
        &[
            "[search-docs] q=bytes::Buf pkg=. docs=0 source=registry-source crate=bytes",
            "|note docsSource=registry-source missing=true",
            "[search-callsite] q=Buf pkg=. calls=",
            "|call src/io/mod.rs:",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
}

#[test]
fn public_large_codex_web_search_flow_connects_prime_to_workspace_symbol_axes() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_public_codex_web_search_workspace(root);

    let prime = run_search(root, &["prime", "--package", "ext/web-search"]);
    assert_lab_packet(
        "public_codex_web_search_prime",
        &prime,
        32,
        &[
            "[search-prime] mode=package package=ext/web-search",
            "|package ext/web-search t=lib,test dep=codex-api,codex-protocol",
            "|dep codex-api import=codex_api pkg=codex-api",
            "|dep codex-protocol import=codex_protocol pkg=codex-protocol",
            "|api-candidate WebSearchTool reason=public-item owner=src/tool.rs",
            "|api-candidate command_action reason=public-item owner=src/tool.rs",
            "|test-surface tests=tests next=tests",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let prime_seeds = run_search(
        root,
        &[
            "prime",
            "--package",
            "ext/web-search",
            "--view",
            "seeds",
            "--seeds",
            "8",
        ],
    );
    assert_lab_packet(
        "public_codex_web_search_prime_seed_flow",
        &prime_seeds,
        10,
        &[
            "[search-prime] root=ext/web-search",
            "owner:path(src/tool.rs)!owner",
            "aliases: owner:{O=owner}",
            "entries=owner-tests",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let owner_flow = run_search(
        root,
        &[
            "owner",
            "src/tool.rs",
            "items",
            "--package",
            "ext/web-search",
        ],
    );
    assert_lab_packet(
        "public_codex_web_search_owner_items_flow",
        &owner_flow,
        16,
        &[
            "[search-owner] q=src/tool.rs pkg=ext/web-search own=1 item=",
            "|owner src/tool.rs role=source source=parser-visible-module",
            "|item WebSearchTool kind=struct",
            "|item command_action kind=fn",
            "|item run_command kind=fn",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let api_dependency_flow = run_search(
        root,
        &[
            "dependency",
            "codex-api",
            "items",
            "public-api",
            "docs",
            "tests",
            "--package",
            "ext/web-search",
        ],
    );
    assert_lab_packet(
        "public_codex_web_search_dependency_flow",
        &api_dependency_flow,
        20,
        &[
            "[search-dependency] q=codex-api pkg=ext/web-search dep=1 own=1 api=",
            "|owner src/tool.rs hit_kind=dependency locations=",
            "|item command_action kind=fn",
            "|api src/tool.rs line=6 dep=codex-api kind=fn name=command_action",
            "|test tests/web_search.rs functions=1 owner=src/tool.rs",
            "|next deps:codex-api,import:codex-api,docs-use:codex-api,crate-source:codex-api,tests",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let api_specific_flow = run_search(
        root,
        &[
            "deps",
            "codex-api::SearchCommands",
            "public-api",
            "--package",
            "ext/web-search",
        ],
    );
    assert_lab_packet(
        "public_codex_web_search_api_specific_dependency_flow",
        &api_specific_flow,
        8,
        &[
            "[search-deps] q=codex-api::SearchCommands pkg=ext/web-search dep=1 own=1 api=1 apiQuery=SearchCommands",
            "|owner src/tool.rs hit_kind=dependency-api apiQuery=SearchCommands",
            "|api src/tool.rs line=6 dep=codex-api kind=fn name=command_action",
            "|next dependency:codex-api,docs-use:codex-api::SearchCommands,crate-source:codex-api,import:codex-api,tests:SearchCommands",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
    assert!(
        !api_specific_flow.contains("name=WebSearchTool"),
        "api-specific flow should not re-emit unrelated owner API:\n{api_specific_flow}"
    );
    assert!(
        !api_specific_flow.contains("name=run_command"),
        "api-specific flow should not re-emit unrelated owner API:\n{api_specific_flow}"
    );

    let api_specific_trace_seeds = run_search(
        root,
        &[
            "deps",
            "codex-api::SearchCommands",
            "public-api",
            "--trace",
            "--view",
            "seeds",
            "--seeds",
            "5",
            "--package",
            "ext/web-search",
        ],
    );
    assert_lab_packet(
        "public_codex_web_search_api_specific_trace_seed_flow",
        &api_specific_trace_seeds,
        12,
        &[
            "[search-trace] source=deps query=codex-api::SearchCommands pipes=public-api view=seeds",
            "|stage cargo=1 owners=1 api=1 final=true lines=",
            "[search-dependency] q=codex-api::SearchCommands",
            "D=dependency:pkg(codex-api::SearchCommands)!dependency",
            "rank=D",
            "frontier=D.dependency",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let rg_json_input = format!(
        "{{\"type\":\"match\",\"data\":{{\"path\":{{\"text\":\"src/tool.rs\"}},\"line_number\":6,\"{}\":0,\"lines\":{{\"text\":\"pub fn command_action(command: SearchCommands) -> WebSearchAction {{\\n\"}},\"submatches\":[{{\"match\":{{\"text\":\"SearchCommands\"}},\"start\":31,\"end\":45}}]}}}}\n\
         {{\"type\":\"match\",\"data\":{{\"path\":{{\"text\":\"src/tool.rs\"}},\"line_number\":13,\"{}\":0,\"lines\":{{\"text\":\"pub fn run_command(tool: WebSearchTool) -> Vec<TurnItem> {{\\n\"}},\"submatches\":[{{\"match\":{{\"text\":\"WebSearchTool\"}},\"start\":25,\"end\":38}}]}}}}\n",
        rg_json_absolute_position_field(),
        rg_json_absolute_position_field(),
    );
    let rg_json_ingest = run_search_with_stdin(
        root,
        &["ingest", "items", "tests", "--package", "ext/web-search"],
        &rg_json_input,
    );
    assert_lab_packet(
        "public_codex_web_search_rg_json_ingest_flow",
        &rg_json_ingest,
        10,
        &[
            "[search-ingest] src=rg-json in=2 own=1",
            "|owner src/tool.rs role=source hit_kind=text locations=6:",
            "13:",
            "|item command_action kind=fn responsibilities=match-dispatch,match-arm,early-return public=true next=syntax:command_action",
            "|item run_command kind=fn responsibilities=early-return public=true next=syntax:run_command",
            "|test tests/web_search.rs functions=1 owner=src/tool.rs",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let workspace_prefixed_owner =
        run_search(root, &["owner", "ext/web-search/src/tool.rs", "items"]);
    assert_lab_packet(
        "public_codex_web_search_workspace_prefixed_owner_flow",
        &workspace_prefixed_owner,
        14,
        &[
            "[search-owner] q=ext/web-search/src/tool.rs pkg=ext/web-search own=1",
            "|owner src/tool.rs role=source source=parser-visible-module",
            "|item command_action kind=fn",
            "|item run_command kind=fn",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let workspace_prefixed_owner_seeds = run_search(
        root,
        &[
            "owner",
            "ext/web-search/src/tool.rs",
            "items",
            "--view",
            "seeds",
            "--seeds",
            "8",
        ],
    );
    assert_lab_packet(
        "public_codex_web_search_workspace_prefixed_owner_seed_flow",
        &workspace_prefixed_owner_seeds,
        13,
        &[
            "O=owner:path(ext/web-search/src/tool.rs)!owner",
            "G>{O:selects,I:matches",
            "rank=I,I2,I3,O frontier=I.syntax,I2.syntax,I3.syntax",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let workspace_prefixed_owner_detail = run_search(
        root,
        &[
            "owner",
            "ext/web-search/src/tool.rs",
            "items",
            "--view",
            "both",
        ],
    );
    assert_lab_packet(
        "public_codex_web_search_workspace_prefixed_owner_detail_flow",
        &workspace_prefixed_owner_detail,
        10,
        &[
            "[search-owner] q=ext/web-search/src/tool.rs pkg=ext/web-search own=1 item=",
            "|owner src/tool.rs role=source source=parser-visible-module",
            "|item command_action kind=fn",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
    assert_eq!(
        workspace_prefixed_owner_detail
            .matches("kind=not-found")
            .count(),
        0,
        "detail view should skip unrelated packages for workspace-prefixed paths:\n{workspace_prefixed_owner_detail}"
    );

    let workspace_prefixed_ingest = run_search_with_stdin(
        root,
        &["ingest", "items", "tests"],
        "ext/web-search/src/tool.rs:6:pub fn command_action(command: SearchCommands) -> WebSearchAction {\n",
    );
    assert_lab_packet(
        "public_codex_web_search_workspace_prefixed_ingest_flow",
        &workspace_prefixed_ingest,
        10,
        &[
            "[search-ingest] src=rg-n in=1 own=1",
            "|owner src/tool.rs role=source hit_kind=text locations=6:1 next=owner",
            "|item command_action kind=fn responsibilities=match-dispatch,match-arm,early-return public=true next=syntax:command_action",
            "|test tests/web_search.rs functions=1 owner=src/tool.rs",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let search_commands_symbol = run_search(root, &["symbol", "SearchCommands"]);
    assert_lab_packet(
        "public_codex_web_search_workspace_search_commands_symbol",
        &search_commands_symbol,
        14,
        &[
            "[search-symbol] q=SearchCommands pkg=crates/codex-api defs=1 calls=1",
            "|def src/lib.rs:1 kind=enum name=SearchCommands",
            "[search-symbol] q=SearchCommands pkg=ext/web-search defs=0 calls=",
            "|call src/tool.rs:",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );

    let action_symbol = run_search(root, &["symbol", "WebSearchAction"]);
    assert_lab_packet(
        "public_codex_web_search_workspace_action_symbol",
        &action_symbol,
        14,
        &[
            "[search-symbol] q=WebSearchAction pkg=crates/codex-protocol defs=1 calls=1",
            "|def src/lib.rs:1 kind=enum name=WebSearchAction",
            "[search-symbol] q=WebSearchAction pkg=ext/web-search defs=0 calls=",
            "|call src/tool.rs:",
        ],
        FORBIDDEN_FLOW_PATTERNS,
    );
}
