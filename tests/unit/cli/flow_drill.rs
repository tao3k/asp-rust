use serde_json::Value;
use tempfile::TempDir;

use super::support::{
    normalize_temp_root, run_cli, run_search, run_search_with_stdin,
    write_complex_dependency_fixture, write_search_fixture,
};

#[test]
fn cli_rust_flow_drill_exercises_registry_prime_search_and_ingest() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let registry = run_cli([
        "agent".as_ref(),
        "doctor".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(registry.status.success(), "{registry:?}");
    let registry_json = serde_json::from_slice::<Value>(&registry.stdout).expect("registry json");
    assert_eq!(
        registry_json["registryId"],
        "agent.semantic-protocols.semantic-language-registry"
    );
    assert_eq!(
        registry_json["protocolId"],
        "agent.semantic-protocols.semantic-language"
    );

    let language = registry_json["languages"][0].as_object().expect("language");
    assert_eq!(language["languageId"], "rust");
assert_eq!(language["providerId"], "asp-rust");
assert_eq!(language["binary"], "asp-rust");
assert_eq!(
    language["namespace"],
    "agent.semantic-protocols.languages.rust.asp-rust"
);
    let methods = language["methods"].as_array().expect("methods");
    for method in [
        "agent/doctor",
        "agent/guide",
        "search/workspace",
        "search/prime",
        "search/dependency",
        "search/deps",
        "search/features",
        "search/ingest",
    ] {
        assert!(
            methods
                .iter()
                .any(|candidate| candidate.as_str() == Some(method)),
            "missing method {method}: {methods:?}"
        );
    }
    let schemas = language["schemas"].as_array().expect("schemas");
    for schema_id in [
        "agent.semantic-protocols.asp-client-exact-query-request",
        "agent.semantic-protocols.semantic-search-packet",
        "agent.semantic-protocols.languages.rust.asp-rust.capabilities",
    ] {
        assert!(
            schemas
                .iter()
                .any(|schema| schema["schemaId"].as_str() == Some(schema_id)),
            "missing schema {schema_id}: {schemas:?}"
        );
    }
    let method_descriptors = language["methodDescriptors"]
        .as_array()
        .expect("method descriptors");
    assert!(method_descriptors.iter().any(|descriptor| {
        descriptor["method"].as_str() == Some("search/ingest")
            && descriptor["acceptsStdin"].as_bool() == Some(true)
    }));

    let workspace = run_search(root, &["workspace"]);
    assert!(
        workspace.starts_with("[search-workspace] root=. pkg=1"),
        "{workspace}"
    );
    assert!(
        workspace.contains(
            "|package . root=. manifest=Cargo.toml source=manifest manager=cargo next=package:."
        ),
        "{workspace}"
    );

    let prime = run_search(root, &["prime"]);
    assert!(
        prime.starts_with("[search-prime] mode=package package=."),
        "{prime}"
    );
    assert!(
        prime.contains("|package . t=lib,test dep=anyhow,serde"),
        "{prime}"
    );
    assert!(prime.contains("|feature json"), "{prime}");
    assert!(
        prime.contains("|test-surface tests=tests next=tests"),
        "{prime}"
    );
    assert!(
        prime.contains(
            "|api-candidate Thing reason=public-item owner=src/domain/mod.rs next=docs:Thing"
        ),
        "{prime}"
    );
    assert!(prime.contains("|owner src/lib.rs"), "{prime}");
    assert!(prime.contains("|owner src/domain/mod.rs"), "{prime}");
    assert!(prime.contains("|next owner:src/lib.rs"), "{prime}");

    let dependency_trace = run_cli([
        "search".as_ref(),
        "dependency".as_ref(),
        "serde".as_ref(),
        "items".as_ref(),
        "public-api".as_ref(),
        "docs".as_ref(),
        "tests".as_ref(),
        "--trace".as_ref(),
        "--view".as_ref(),
        "both".as_ref(),
        root.as_os_str(),
    ]);
    assert!(dependency_trace.status.success(), "{dependency_trace:?}");
    let dependency_trace = normalize_temp_root(
        &String::from_utf8(dependency_trace.stdout).expect("dependency trace stdout"),
        root,
    );
    assert!(
        dependency_trace.starts_with(
            "[search-trace] source=dependency query=serde pipes=items,public-api,docs,tests view=both"
        ),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|stage cargo=1 owners=2 items="),
        "{dependency_trace}"
    );
    assert!(dependency_trace.contains(" api=8"), "{dependency_trace}");
    assert!(dependency_trace.contains(" tests=1"), "{dependency_trace}");
    assert!(
        dependency_trace.contains(" final=true lines="),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("[search-dependency] q=serde pkg=. dep=1 own=2 api=8"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|item load kind=fn"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing"),
        "{dependency_trace}"
    );
    assert!(
        dependency_trace.contains("|test tests/domain.rs functions=1 owner=src/lib.rs"),
        "{dependency_trace}"
    );

    let feature = run_search(root, &["features", "json", "cfg", "owners", "tests"]);
    assert!(
        feature.starts_with("[search-features] q=json pkg=. feat=1 dep=1"),
        "{feature}"
    );
    assert!(feature.contains(" cfg=1"), "{feature}");
    assert!(feature.contains(" own=1"), "{feature}");
    assert!(
        feature.contains("|cfg feature:json declared_in=features expr=cfg(feature=\"json\")"),
        "{feature}"
    );
    assert!(
        feature.contains("|owner src/domain/mod.rs hit_kind=feature"),
        "{feature}"
    );

    let external_dep_api = run_search(root, &["deps", "serde@2::Serialize"]);
    assert!(
        external_dep_api.starts_with(
            "[search-deps] q=serde@2::Serialize pkg=. dep=1 own=0 api=0 requestedVersion=2 currentWorkspaceVersion=^1 versionScope=external apiQuery=Serialize"
        ),
        "{external_dep_api}"
    );
    assert!(
        external_dep_api.contains(
            "|note kind=version-scope message=requested-version-is-outside-current-workspace-version"
        ),
        "{external_dep_api}"
    );
    assert!(
        external_dep_api.contains(
            "|next dependency:serde,docs-use:serde::Serialize,crate-source:serde,import:serde,tests:Serialize"
        ),
        "{external_dep_api}"
    );
    assert!(
        !external_dep_api.contains("|owner src/lib.rs"),
        "{external_dep_api}"
    );

    let ingest = run_search_with_stdin(root, &["ingest"], "src/lib.rs:6:pub fn load() -> Thing\n");
    assert!(
        ingest.starts_with("[search-ingest] src=rg-n in=1 own=1"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|owner src/lib.rs role=source hit_kind=text locations=6:1 next=owner"),
        "{ingest}"
    );

    let test_owner = run_search(root, &["owner", "tests/domain.rs"]);
    assert!(
        test_owner.starts_with("[search-owner] q=tests/domain.rs pkg=. own=1 item=0"),
        "{test_owner}"
    );
    assert!(
        test_owner.contains("|owner tests/domain.rs role=test source=parser-visible-module"),
        "{test_owner}"
    );
    assert!(test_owner.contains(" lines=4 "), "{test_owner}");
    assert!(test_owner.contains(" imports=1 "), "{test_owner}");
    assert!(
        test_owner.contains("next=owner:tests/domain.rs,tests:tests/domain.rs"),
        "{test_owner}"
    );
    assert!(!test_owner.contains("source=path-only"), "{test_owner}");

    std::fs::write(root.join("README.md"), "# Fixture notes\n").expect("write readme");
    let path_only = run_search(root, &["owner", "README.md"]);
    assert!(
        path_only.starts_with("[search-owner] q=README.md pkg=. own=1 item=0"),
        "{path_only}"
    );
    assert!(
        path_only.contains("|owner README.md role=source source=path-only next=ingest:README.md"),
        "{path_only}"
    );
    assert!(
        path_only.contains(
            "|note kind=owner-not-found message=\"path exists but is not a parser-visible owner; use search ingest for line evidence\""
        ),
        "{path_only}"
    );
}

#[test]
fn cli_search_ranks_equal_hits_by_project_path_mtime() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);
    set_mtime(root.join("src/domain/mod.rs"), 1_700_000_000);
    set_mtime(root.join("src/lib.rs"), 1_800_000_000);

    let ingest = run_search_with_stdin(
        root,
        &["ingest"],
        "src/domain/mod.rs:2:use serde::Serialize\nsrc/lib.rs:3:use serde::Serialize\n",
    );
    let ingest_owner_lines = ingest
        .lines()
        .filter(|line| line.starts_with("|owner "))
        .collect::<Vec<_>>();
    assert!(
        ingest_owner_lines
            .first()
            .is_some_and(|line| line.starts_with("|owner src/lib.rs ")),
        "{ingest}"
    );

    let deps = run_search(root, &["deps", "serde::Serialize"]);
    let deps_owner_lines = deps
        .lines()
        .filter(|line| line.starts_with("|owner "))
        .collect::<Vec<_>>();
    assert!(
        deps_owner_lines
            .first()
            .is_some_and(|line| line.starts_with("|owner src/lib.rs ")),
        "{deps}"
    );
}

#[test]
fn cli_rust_flow_drill_reduces_search_rounds_with_seeds_and_recipe_plan() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let full = run_search(
        root,
        &[
            "dependency",
            "serde",
            "items",
            "public-api",
            "docs",
            "tests",
        ],
    );
    assert!(full.contains("|owner src/lib.rs"), "{full}");
    assert!(full.contains("|item load kind=fn"), "{full}");
    assert!(
        full.contains("|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing"),
        "{full}"
    );
    assert!(
        full.contains("|test tests/domain.rs functions=1 owner=src/lib.rs"),
        "{full}"
    );

    let seeds = run_cli([
        "search".as_ref(),
        "dependency".as_ref(),
        "serde".as_ref(),
        "items".as_ref(),
        "public-api".as_ref(),
        "docs".as_ref(),
        "tests".as_ref(),
        "--view".as_ref(),
        "seeds".as_ref(),
        root.as_os_str(),
    ]);
    assert!(seeds.status.success(), "{seeds:?}");
    let seeds = normalize_temp_root(&String::from_utf8(seeds.stdout).expect("seed stdout"), root);
    assert!(
        seeds.starts_with("[search-dependency] q=serde alg=seed-frontier"),
        "{seeds}"
    );
    assert!(seeds.lines().count() < full.lines().count(), "{seeds}");
    assert!(
        seeds.contains("D=dependency:pkg(serde)!dependency"),
        "{seeds}"
    );
    assert!(seeds.contains("T=test:path(.)!tests"), "{seeds}");
    assert!(seeds.contains("U=doc-use:path(serde)!docs-use"), "{seeds}");
    assert!(
        seeds.contains("C=crate-source:pkg(serde)!crate-source"),
        "{seeds}"
    );
    assert!(seeds.contains("I=import:path(serde)!import"), "{seeds}");
    assert!(
        seeds.contains("O2=owner:path(src/domain/mod.rs)!owner"),
        "{seeds}"
    );
    assert!(
        seeds.contains(
            "frontier=D.dependency,U.docs-use,C.crate-source,T.tests,O.owner,D2.deps,I.import"
        ),
        "{seeds}"
    );
    assert!(!seeds.contains("|owner src/lib.rs"), "{seeds}");
    assert!(!seeds.contains("|item load"), "{seeds}");
    assert!(!seeds.contains("|api src/domain/mod.rs"), "{seeds}");
    assert!(!seeds.contains("|edge "), "{seeds}");

    let plan = run_cli([
        "search".as_ref(),
        "dependency".as_ref(),
        "serde".as_ref(),
        "--explain".as_ref(),
        "--view".as_ref(),
        "seeds".as_ref(),
        root.as_os_str(),
    ]);
    assert!(plan.status.success(), "{plan:?}");
    let plan = normalize_temp_root(&String::from_utf8(plan.stdout).expect("plan stdout"), root);
    assert!(
        plan.starts_with("[search-plan] view=dependency q=serde mode=seeds"),
        "{plan}"
    );
    assert!(
        plan.contains("|recipe dependency-change focus=multi-pipe token=final-only"),
        "{plan}"
    );
    assert!(
        plan.contains("|prefer search:dependency:serde(items,public-api,docs,tests)"),
        "{plan}"
    );
    assert!(
        plan.contains("|subagent deps=search:deps:serde[::api]"),
        "{plan}"
    );
    assert!(
        plan.contains("|fallback ingest=rg-n:serde(scope=src,tests)"),
        "{plan}"
    );
    assert!(
        plan.contains("|budget commands=3 rounds=2 output=bounded"),
        "{plan}"
    );
    assert!(
        plan.contains("[search-dependency] q=serde alg=seed-frontier"),
        "{plan}"
    );
    assert!(
        plan.contains("D=dependency:pkg(serde)!dependency"),
        "{plan}"
    );
    assert!(plan.contains("T=test:path(.)!tests"), "{plan}");
    assert!(
        plan.contains("frontier=D.dependency,U.docs-use,C.crate-source,T.tests"),
        "{plan}"
    );

    let ingest = run_search_with_stdin(
        root,
        &["ingest", "items", "tests"],
        "src/lib.rs:6:pub fn load() -> Thing\n",
    );
    assert!(
        ingest.starts_with("[search-ingest] src=rg-n in=1 own=1"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|owner src/lib.rs role=source hit_kind=text locations=6:1 next=owner"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|item load kind=fn responsibilities=early-return public=true next=syntax:load"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|test tests/domain.rs functions=1 owner=src/lib.rs"),
        "{ingest}"
    );

    let vimgrep =
        run_search_with_stdin(root, &["ingest"], "src/lib.rs:6:5:pub fn load() -> Thing\n");
    assert!(
        vimgrep.starts_with("[search-ingest] src=vimgrep in=1 own=1"),
        "{vimgrep}"
    );
    assert!(
        vimgrep.contains("|owner src/lib.rs role=source hit_kind=text locations=6:5 next=owner"),
        "{vimgrep}"
    );

    let rg_json_input = format!(
        "{{\"type\":\"match\",\"data\":{{\"path\":{{\"text\":\"src/lib.rs\"}},\"line_number\":6,\"{}\":0,\"lines\":{{\"text\":\"pub fn load() -> Thing\\n\"}},\"submatches\":[{{\"match\":{{\"text\":\"load\"}},\"start\":7,\"end\":11}}]}}}}\n",
        rg_json_absolute_position_field(),
    );
    let rg_json = run_search_with_stdin(root, &["ingest"], &rg_json_input);
    assert!(
        rg_json.starts_with("[search-ingest] src=rg-json in=1 own=1"),
        "{rg_json}"
    );
    assert!(
        rg_json.contains("|owner src/lib.rs role=source hit_kind=text locations=6:8 next=owner"),
        "{rg_json}"
    );

    let nul_paths = run_search_with_stdin(root, &["ingest"], "src/lib.rs\0tests/domain.rs\0");
    assert!(
        nul_paths.starts_with("[search-ingest] src=path-list-nul in=2 own=2"),
        "{nul_paths}"
    );
    assert!(
        nul_paths.contains("|owner src/lib.rs role=source hit_kind=path locations=- next=owner"),
        "{nul_paths}"
    );
    assert!(
        nul_paths.contains("|owner tests/domain.rs role=test hit_kind=path locations=- next=owner"),
        "{nul_paths}"
    );

    let unknown = run_search_with_stdin(root, &["ingest"], "not a real path and not rg\n");
    assert!(
        unknown.starts_with("[search-ingest] error=unrecognized-input lines=1"),
        "{unknown}"
    );
    assert!(
        unknown.contains("|fix pipe paths, rg -n, rg --json, git diff --name-only, or fd output"),
        "{unknown}"
    );
}

fn rg_json_absolute_position_field() -> String {
    ["absolute", "off", "set"].join("_")
}

fn set_mtime(path: impl AsRef<std::path::Path>, seconds: i64) {
    let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds as u64);
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(time))
        .expect("set fixture mtime");
}

fn assert_line_order(rendered: &str, first: &str, second: &str) {
    let first_index = rendered
        .find(first)
        .unwrap_or_else(|| panic!("missing first line fragment {first:?} in:\n{rendered}"));
    let second_index = rendered
        .find(second)
        .unwrap_or_else(|| panic!("missing second line fragment {second:?} in:\n{rendered}"));
    assert!(
        first_index < second_index,
        "expected {first:?} before {second:?} in:\n{rendered}"
    );
}

#[test]
fn cli_rust_flow_drill_regresses_tokio_ignore_bytes_style_flow() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_complex_dependency_fixture(root);
    set_mtime(root.join("src/http/client.rs"), 1_700_000_000);
    set_mtime(root.join("src/io/walk.rs"), 1_800_000_000);

    let prime = run_search(root, &["prime"]);
    assert!(
        prime.starts_with("[search-prime] mode=package package=."),
        "{prime}"
    );
    assert!(prime.contains("dep=bytes,ignore,tokio"), "{prime}");
    assert!(prime.contains("|feature runtime"), "{prime}");
    assert!(prime.contains("|feature walk"), "{prime}");
    assert!(
        prime.contains("|edge O:src/http/mod.rs -mod-> O:src/http/client.rs"),
        "{prime}"
    );
    assert!(
        prime.contains("|edge O:src/io/mod.rs -mod-> O:src/io/walk.rs"),
        "{prime}"
    );
    assert!(
        prime.contains("|api-candidate RuntimeClient reason=public-item owner=src/http/client.rs"),
        "{prime}"
    );
    assert!(
        prime.contains("|api-candidate WalkPlan reason=public-item owner=src/io/walk.rs"),
        "{prime}"
    );

    let tokio_sender = run_search(root, &["deps", "tokio@1::Sender"]);
    assert!(
        tokio_sender.starts_with(
            "[search-deps] q=tokio@1::Sender pkg=. dep=1 own=1 api=0 requestedVersion=1 currentWorkspaceVersion=^1 versionScope=current apiQuery=Sender"
        ),
        "{tokio_sender}"
    );
    assert!(
        tokio_sender.contains("|owner src/http/client.rs hit_kind=dependency-api apiQuery=Sender"),
        "{tokio_sender}"
    );
    assert!(
        tokio_sender.contains(
            "|dependency-topology dep=tokio usageLevel=local_usage topology=needs-index ownerUsage=1"
        ),
        "{tokio_sender}"
    );
    assert!(
        tokio_sender.contains(
            "|next dependency:tokio,docs-use:tokio::Sender,crate-source:tokio,import:tokio,tests:Sender"
        ),
        "{tokio_sender}"
    );

    let ignore_external = run_search(root, &["deps", "ignore@0.3::WalkBuilder"]);
    assert!(
        ignore_external.starts_with(
            "[search-deps] q=ignore@0.3::WalkBuilder pkg=. dep=1 own=0 api=0 requestedVersion=0.3 currentWorkspaceVersion=^0.4 versionScope=external apiQuery=WalkBuilder"
        ),
        "{ignore_external}"
    );
    assert!(
        ignore_external.contains(
            "|note kind=version-scope message=requested-version-is-outside-current-workspace-version"
        ),
        "{ignore_external}"
    );
    assert!(
        !ignore_external.contains("|owner src/io/walk.rs"),
        "{ignore_external}"
    );

    let bytes_dependency = run_search(
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
    assert!(
        bytes_dependency.starts_with("[search-dependency] q=bytes pkg=. dep=1 own=2 api="),
        "{bytes_dependency}"
    );
    assert!(
        bytes_dependency.contains("|owner src/http/client.rs hit_kind=dependency"),
        "{bytes_dependency}"
    );
    assert!(
        bytes_dependency.contains("|owner src/io/walk.rs hit_kind=dependency"),
        "{bytes_dependency}"
    );
    assert_line_order(
        &bytes_dependency,
        "|owner src/io/walk.rs hit_kind=dependency",
        "|owner src/http/client.rs hit_kind=dependency",
    );
    assert!(
        bytes_dependency
            .contains("|api src/http/client.rs line=4 dep=bytes kind=struct name=RuntimeClient"),
        "{bytes_dependency}"
    );
    assert!(
        bytes_dependency.contains("|test tests/flow.rs functions=1 owner=src/http/client.rs"),
        "{bytes_dependency}"
    );

    let external_types = run_search(root, &["public-external-types"]);
    assert!(
        external_types.starts_with("[search-public-external-types] pkg=. dep=3 hit="),
        "{external_types}"
    );
    assert!(
        external_types.contains(
            "|external-type src/http/client.rs:4 dep=tokio surface=field:sender item=RuntimeClient type=Sender<Bytes>"
        ),
        "{external_types}"
    );
    assert!(
        external_types.contains(
            "|external-type src/io/walk.rs:4 dep=ignore surface=field:builder item=WalkPlan type=WalkBuilder"
        ),
        "{external_types}"
    );
    assert!(
        external_types.contains(
            "|external-type src/io/walk.rs:4 dep=bytes surface=field:seed item=WalkPlan type=Bytes"
        ),
        "{external_types}"
    );

    let api = run_search(root, &["api", "send_bytes"]);
    assert!(
        api.starts_with("[search-api] q=send_bytes pkg=. api=1 source=native-parser"),
        "{api}"
    );
    assert!(
        api.contains(
            "|api src/http/client.rs:6 kind=fn name=send_bytes next=owner:src/http/client.rs source=native-parser apiKind=fn public=true docs=false"
        ),
        "{api}"
    );
    assert!(api.contains("apiKind=fn public=true docs=false"), "{api}");

    let feature = run_search(root, &["features", "runtime", "cfg", "owners", "tests"]);
    assert!(
        feature.starts_with("[search-features] q=runtime pkg=. feat=1 dep=2"),
        "{feature}"
    );
    assert!(feature.contains(" cfg=1"), "{feature}");
    assert!(feature.contains(" own=2"), "{feature}");
    assert!(feature.contains(" tests=1"), "{feature}");
    assert!(
        feature.contains("|dep tokio import=tokio pkg=tokio version=^1 kind=normal opt=true"),
        "{feature}"
    );
    assert!(
        feature.contains("|dep bytes import=bytes pkg=bytes version=^1 kind=normal opt=true"),
        "{feature}"
    );
    assert!(
        feature.contains("|test tests/flow.rs functions=1 owner=src/http/client.rs"),
        "{feature}"
    );

    let ingest = run_search_with_stdin(
        root,
        &["ingest", "items", "tests"],
        "src/http/client.rs:4:pub struct RuntimeClient { pub sender: Sender<Bytes>, pub buffer: Bytes }\n\
         src/io/walk.rs:4:pub struct WalkPlan { pub builder: WalkBuilder, pub seed: Bytes }\n",
    );
    assert!(
        ingest.starts_with("[search-ingest] src=rg-n in=2 own=2"),
        "{ingest}"
    );
    assert_line_order(
        &ingest,
        "|owner src/io/walk.rs role=source hit_kind=text",
        "|owner src/http/client.rs role=source hit_kind=text",
    );
    assert!(
        ingest.contains(
            "|owner src/http/client.rs role=source hit_kind=text locations=4:1 next=owner"
        ),
        "{ingest}"
    );
    assert!(
        ingest.contains("|item RuntimeClient kind=struct responsibilities=data-shape public=true"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|owner src/io/walk.rs role=source hit_kind=text locations=4:1 next=owner"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|item WalkPlan kind=struct responsibilities=data-shape public=true"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|test tests/flow.rs functions=1 owner=src/http/client.rs"),
        "{ingest}"
    );

    let ingest_json_input = format!(
        "{{\"type\":\"match\",\"data\":{{\"path\":{{\"text\":\"src/io/walk.rs\"}},\"line_number\":4,\"{}\":0,\"lines\":{{\"text\":\"pub struct WalkPlan {{ pub builder: WalkBuilder, pub seed: Bytes }}\\n\"}},\"submatches\":[{{\"match\":{{\"text\":\"WalkBuilder\"}},\"start\":42,\"end\":53}}]}}}}\n",
        rg_json_absolute_position_field(),
    );
    let ingest_json = run_search_with_stdin(root, &["ingest", "--json"], &ingest_json_input);
    let ingest_json = serde_json::from_str::<Value>(&ingest_json).expect("ingest json");
    assert_eq!(ingest_json["method"], "search/ingest");
    assert_eq!(ingest_json["inputDetection"]["source"], "rg-json");
    assert_eq!(ingest_json["inputDetection"]["lineCount"], 1);
}
