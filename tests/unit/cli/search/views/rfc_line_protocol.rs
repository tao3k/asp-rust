use super::{run_cli, run_search, run_search_with_stdin, write_search_fixture};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn cli_search_views_render_rfc_line_protocol() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_search_fixture(root);

    let deps = run_search(root, &["deps"]);
    assert!(deps.starts_with("[search-deps] pkg=. dep=2"), "{deps}");
    assert!(
        deps.contains(
            "|dep serde import=serde pkg=serde version=^1 kind=normal opt=true source=manifest manager=cargo feat=derive"
        ),
        "{deps}"
    );
    assert!(
        deps.contains(
            "|dep anyhow import=anyhow pkg=anyhow version=^1 kind=normal opt=false source=manifest manager=cargo"
        ),
        "{deps}"
    );

    let dep = run_search(root, &["deps", "serde"]);
    assert!(
        dep.starts_with("[search-deps] q=serde pkg=. dep=1 own=0 api=0"),
        "{dep}"
    );
    assert!(
        dep.contains("|dep serde import=serde pkg=serde version=^1"),
        "{dep}"
    );
    assert!(
        dep.contains("|dependency-topology dep=serde usageLevel=manifest topology=asp-owned"),
        "{dep}"
    );
    assert!(
        dep.contains("|next dependency:serde,docs-use:serde,crate-source:serde,import:serde,tests"),
        "{dep}"
    );

    let dep_usage = run_search(root, &["deps", "serde", "usage"]);
    assert!(
        dep_usage.starts_with("[search-deps] q=serde pkg=. dep=1 own=2 api=0"),
        "{dep_usage}"
    );
    assert!(
        dep_usage.contains("|owner src/lib.rs hit_kind=dependency"),
        "{dep_usage}"
    );
    assert!(
        dep_usage.contains("|owner src/domain/mod.rs hit_kind=dependency"),
        "{dep_usage}"
    );

    let dep_public_api = run_search(root, &["deps", "serde", "public-api"]);
    assert!(
        dep_public_api.starts_with("[search-deps] q=serde pkg=. dep=1 own=2 api=8"),
        "{dep_public_api}"
    );
    assert!(
        dep_public_api.contains("|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing"),
        "{dep_public_api}"
    );

    let features = run_search(root, &["features"]);
    assert!(
        features.starts_with("[search-features] pkg=. feat=2"),
        "{features}"
    );
    assert!(
        features
            .contains("|feature json enables=dep:serde,serde/derive source=manifest manager=cargo"),
        "{features}"
    );

    let feature = run_search(root, &["features", "json", "cfg", "owners", "tests"]);
    assert!(
        feature.starts_with("[search-features] q=json pkg=. feat=1 dep=1"),
        "{feature}"
    );
    assert!(feature.contains(" cfg=1"), "{feature}");
    assert!(feature.contains(" own=1"), "{feature}");
    assert!(
        feature
            .contains("|feature json enables=dep:serde,serde/derive source=manifest manager=cargo"),
        "{feature}"
    );
    assert!(
        feature.contains("|cfg feature:json declared_in=features expr=cfg(feature=\"json\")"),
        "{feature}"
    );
    assert!(
        feature.contains("|owner src/domain/mod.rs hit_kind=feature"),
        "{feature}"
    );
    assert!(
        feature.contains("|next cfg:json,text:json(scope=src),tests"),
        "{feature}"
    );

    let workspace = run_search(root, &["workspace"]);
    assert!(
        workspace.starts_with("[search-workspace] root=. pkg=1"),
        "{workspace}"
    );
    assert!(
        workspace.contains("|package . root=. manifest=Cargo.toml"),
        "{workspace}"
    );
    let workspace_seeds = run_search(root, &["workspace", "--view", "seeds"]);
    assert!(
        workspace_seeds.starts_with("[search-workspace] root=. alg=seed-frontier"),
        "{workspace_seeds}"
    );
    assert!(
        workspace_seeds.contains("aliases: graph:{G=search,P=package}"),
        "{workspace_seeds}"
    );
    assert!(
        workspace_seeds.contains("P=package:pkg(.)"),
        "{workspace_seeds}"
    );
    assert!(
        workspace_seeds.contains("frontier=P.owner"),
        "{workspace_seeds}"
    );
    assert!(!workspace_seeds.contains("G>{}"), "{workspace_seeds}");

    let targets = run_search(root, &["targets"]);
    assert!(
        targets.starts_with("[search-targets] pkg=. source=1 test=1"),
        "{targets}"
    );
    assert!(
        targets.contains(
            "|target path=src/lib.rs source=manifest manager=cargo next=owner:src/lib.rs"
        ),
        "{targets}"
    );
    assert!(
        targets.contains(
            "|target path=tests/domain.rs source=manifest manager=cargo next=owner:tests/domain.rs"
        ),
        "{targets}"
    );

    let dependency = run_search(
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
    assert!(
        dependency.starts_with("[search-dependency] q=serde pkg=. dep=1 own=2 api=8"),
        "{dependency}"
    );
    assert!(dependency.contains(" item="), "{dependency}");
    assert!(dependency.contains(" docs=8"), "{dependency}");
    assert!(dependency.contains(" tests=1"), "{dependency}");
    assert!(
        dependency.contains("|dep serde import=serde pkg=serde"),
        "{dependency}"
    );
    assert!(
        dependency.contains("|owner src/lib.rs hit_kind=dependency"),
        "{dependency}"
    );
    assert!(
        dependency.contains("|owner src/domain/mod.rs hit_kind=dependency"),
        "{dependency}"
    );
    assert!(dependency.contains("|item load kind=fn"), "{dependency}");
    assert!(
        dependency.contains(
            "|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing public=true doc=false reason=dependency-owner next=docs:Thing,tests"
        ),
        "{dependency}"
    );
    assert!(
        dependency.contains(
            "|test tests/domain.rs functions=1 owner=src/lib.rs reason=symbol:load next=owner:tests/domain.rs"
        ),
        "{dependency}"
    );
    assert!(
        dependency
            .contains("|next deps:serde,import:serde,docs-use:serde,crate-source:serde,tests"),
        "{dependency}"
    );
    assert!(
        dependency.contains("avoid=web-search,docs.rs-search,raw-read"),
        "{dependency}"
    );

    let dependency_set = run_search(root, &["dependency", "serde,anyhow", "items", "tests"]);
    assert!(
        dependency_set.starts_with(
            "[search-dependency] q=serde,anyhow querySet=2 selector=exact-set pkg=. dep=2 own=2 api=0"
        ),
        "{dependency_set}"
    );
    assert!(dependency_set.contains(" item="), "{dependency_set}");
    assert!(dependency_set.contains(" tests=1"), "{dependency_set}");
    assert!(
        dependency_set.contains("|dep serde import=serde pkg=serde"),
        "{dependency_set}"
    );
    assert!(
        dependency_set.contains("|dep anyhow import=anyhow pkg=anyhow"),
        "{dependency_set}"
    );
    assert!(
        dependency_set.contains("|owner src/lib.rs hit_kind=dependency"),
        "{dependency_set}"
    );
    assert!(
        dependency_set
            .contains("|next deps:serde,import:serde,docs-use:serde,crate-source:serde,tests"),
        "{dependency_set}"
    );
    assert!(
        dependency_set
            .contains("|next deps:anyhow,import:anyhow,docs-use:anyhow,crate-source:anyhow,tests"),
        "{dependency_set}"
    );
    assert!(
        dependency_set.contains("avoid=web-search,docs.rs-search,raw-read"),
        "{dependency_set}"
    );

    let dependency_set_json = run_cli([
        "search".as_ref(),
        "dependency".as_ref(),
        "serde,anyhow".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(
        dependency_set_json.status.success(),
        "{dependency_set_json:?}"
    );
    let value =
        serde_json::from_slice::<Value>(&dependency_set_json.stdout).expect("dependency set json");
    assert_eq!(value["query"], "serde,anyhow");
    assert_eq!(value["header"]["fields"]["querySet"], 2);
    assert_eq!(value["header"]["fields"]["selector"], "exact-set");
    assert_eq!(value["querySet"][0]["value"], "serde");
    assert_eq!(value["querySet"][0]["kind"], "dependency");
    assert_eq!(value["querySet"][0]["selector"], "exact");
    assert_eq!(value["querySet"][1]["value"], "anyhow");
    assert_eq!(value["queryComposition"]["mode"], "query-set");
    assert_eq!(value["queryComposition"]["view"], "dependency");
    assert_eq!(value["queryComposition"]["selector"], "exact-set");

    let owner = run_search(root, &["owner", "src/lib.rs", "items"]);
    assert!(
        owner.starts_with("[search-owner] q=src/lib.rs pkg=."),
        "{owner}"
    );
    assert!(owner.contains("|owner src/lib.rs"), "{owner}");
    assert!(owner.contains("|item load kind=fn"), "{owner}");
    assert!(
        owner.contains("|item WireApi kind=trait public=true"),
        "{owner}"
    );
    assert!(owner.contains("syn=trait_item/name"), "{owner}");
    assert!(!owner.contains("|synthesis"), "{owner}");

    let owner_json = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        "src/lib.rs".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(owner_json.status.success(), "{owner_json:?}");
    let value = serde_json::from_slice::<Value>(&owner_json.stdout).expect("owner json");
    assert_eq!(
        value["searchSynthesis"]["algorithm"],
        "bounded-reachability-depth1"
    );
    assert_eq!(value["searchSynthesis"]["scope"], "owner");
    assert_eq!(value["searchSynthesis"]["ownerPath"], "src/lib.rs");

    let owner_item_query = run_search(root, &["owner", "src/lib.rs", "items", "--query", "load"]);
    assert!(
        owner_item_query
            .starts_with("[search-owner] q=src/lib.rs pkg=. own=1 item=1 itemQuery=load"),
        "{owner_item_query}"
    );
    assert!(
        owner_item_query
            .contains("|item load kind=fn responsibilities=early-return public=true next=syntax:load read=src/lib.rs:6:6"),
        "{owner_item_query}"
    );
    assert!(
        owner_item_query.contains("structuralSelector=rust://src/lib.rs#item/function/load"),
        "{owner_item_query}"
    );
    assert!(
        owner_item_query.contains(
            "canonicalItemSelector={\"schemaId\":\"asp.canonical-item-selector.v1\",\"schemaVersion\":\"v1\",\"languageId\":\"rust\",\"kind\":\"function\",\"symbol\":\"load\",\"scopes\":[],\"structuralSelector\":\"rust://src/lib.rs#item/function/load\"}"
        ),
        "{owner_item_query}"
    );
    assert!(
        owner_item_query.contains("syn=function_item/name"),
        "{owner_item_query}"
    );
    assert!(!owner_item_query.contains("|code "), "{owner_item_query}");
    assert!(!owner_item_query.contains(" text="), "{owner_item_query}");
    assert!(
        !owner_item_query.contains("clone_value"),
        "{owner_item_query}"
    );
    let owner_names_only = run_search(
        root,
        &[
            "owner",
            "src/lib.rs",
            "items",
            "--query",
            "loa",
            "--names-only",
        ],
    );
    assert!(
        owner_names_only.contains(
            "|query itemQuery=loa status=hit match=fallback-contains item=1 reason=parser-item-fallback output=names next=query-code"
        ),
        "{owner_names_only}"
    );
    assert!(
        owner_names_only.contains("|item load kind=fn"),
        "{owner_names_only}"
    );
    assert!(
        !owner_names_only.contains("|code path=src/lib.rs"),
        "{owner_names_only}"
    );
    let owner_multi_query = run_search(
        root,
        &[
            "owner",
            "src/lib.rs",
            "items",
            "--query",
            "load|clone_value",
        ],
    );
    assert!(
        owner_multi_query.starts_with(
            "[search-owner] q=src/lib.rs pkg=. own=1 item=2 itemQuery=load|clone_value"
        ),
        "{owner_multi_query}"
    );
    assert!(
        owner_multi_query.contains("|item load kind=fn responsibilities=early-return public=true"),
        "{owner_multi_query}"
    );
    assert!(
        owner_multi_query
            .contains("|item clone_value kind=fn responsibilities=early-return public=true"),
        "{owner_multi_query}"
    );
    assert!(
        owner_multi_query.contains("syn=function_item/name"),
        "{owner_multi_query}"
    );
    assert!(!owner_multi_query.contains(" text="), "{owner_multi_query}");
    let owner_set = run_search(root, &["owner", "src/lib.rs,src/domain/mod.rs", "items"]);
    assert!(
        owner_set.starts_with(
            "[search-owner] q=src/lib.rs,src/domain/mod.rs querySet=2 selector=exact-set pkg=. own=2"
        ),
        "{owner_set}"
    );
    assert!(owner_set.contains("|owner src/lib.rs"), "{owner_set}");
    assert!(
        owner_set.contains("|owner src/domain/mod.rs"),
        "{owner_set}"
    );
    assert!(owner_set.contains("|item load kind=fn"), "{owner_set}");
    assert!(
        owner_set.contains("|item make_thing kind=fn"),
        "{owner_set}"
    );

    let symbol = run_search(root, &["symbol", "load"]);
    assert!(
        symbol.starts_with("[search-symbol] q=load pkg=."),
        "{symbol}"
    );
    assert!(symbol.contains("|def src/lib.rs"), "{symbol}");
    assert!(symbol.contains("kind=fn name=load"), "{symbol}");

    let callsite = run_search(root, &["callsite", "make_thing"]);
    assert!(
        callsite.starts_with("[search-callsite] q=make_thing pkg=."),
        "{callsite}"
    );
    assert!(callsite.contains("|call src/lib.rs"), "{callsite}");

    let import = run_search(root, &["import", "serde"]);
    assert!(
        import.starts_with("[search-import] q=serde pkg=. own=2"),
        "{import}"
    );
    assert!(
        import.contains("|owner src/lib.rs hit_kind=import"),
        "{import}"
    );
    assert!(
        import.contains("|owner src/domain/mod.rs hit_kind=import"),
        "{import}"
    );

    let owner_frontier = run_search(root, &["owner", "src/lib.rs"]);
    assert!(
        owner_frontier.contains("|hot load kind=fn responsibilities=early-return public=true"),
        "{owner_frontier}"
    );
    assert!(
        owner_frontier
            .contains("|test tests/domain.rs functions=1 owner=src/lib.rs reason=symbol:load"),
        "{owner_frontier}"
    );
    assert!(!owner_frontier.contains("|item load "), "{owner_frontier}");

    let owner_with_tests = run_search(root, &["owner", "src/lib.rs", "items", "tests"]);
    assert!(
        owner_with_tests.contains("|item load kind=fn responsibilities=early-return public=true"),
        "{owner_with_tests}"
    );
    assert!(
        owner_with_tests.contains("syn=function_item/name"),
        "{owner_with_tests}"
    );
    assert!(
        owner_with_tests
            .contains("|test tests/domain.rs functions=1 owner=src/lib.rs reason=symbol:load"),
        "{owner_with_tests}"
    );

    let cfg = run_search(root, &["cfg", "json"]);
    assert!(
        cfg.starts_with("[search-cfg] q=json pkg=. cfg=1 dep=0 own=1"),
        "{cfg}"
    );

    let patterns = run_search(root, &["patterns"]);
    assert!(patterns.starts_with("[search-patterns] n=4"), "{patterns}");
    assert!(!patterns.contains("|pat clone-in-loop"), "{patterns}");
    assert!(!patterns.contains("|pat unsafe-boundary"), "{patterns}");
    assert!(
        !patterns.contains("|pat tokio-spawn-unawaited"),
        "{patterns}"
    );
    assert!(!patterns.contains("|pat public-primitive-id"), "{patterns}");
    assert!(
        patterns.contains("|pat public-error-boundary lang=rust scope=src"),
        "{patterns}"
    );
    assert!(
        patterns.contains("|pat public-external-type lang=rust scope=src option=dependency"),
        "{patterns}"
    );

    let anyhow_pattern = run_search(root, &["pattern", "public-anyhow-result"]);
    assert!(
        anyhow_pattern.starts_with(
            "[search-pattern] pattern=public-anyhow-result pkg=. hit=2 source=native-parser"
        ),
        "{anyhow_pattern}"
    );
    assert!(
        anyhow_pattern.contains(
            "|api src/lib.rs:11 kind=fn name=fallible next=owner:src/lib.rs source=native-parser"
        ),
        "{anyhow_pattern}"
    );
    assert!(
        anyhow_pattern.contains(
            "|api src/lib.rs:18 kind=method name=wire next=owner:src/lib.rs source=native-parser"
        ),
        "{anyhow_pattern}"
    );
    assert!(
        !anyhow_pattern.contains("hit_kind=text"),
        "{anyhow_pattern}"
    );

    let error_boundary_pattern = run_search(root, &["pattern", "public-error-boundary"]);
    assert!(
        error_boundary_pattern.starts_with(
            "[search-pattern] pattern=public-error-boundary pkg=. hit=2 source=native-parser"
        ),
        "{error_boundary_pattern}"
    );
    assert!(
        error_boundary_pattern.contains("name=fallible"),
        "{error_boundary_pattern}"
    );
    assert!(
        error_boundary_pattern.contains("name=wire"),
        "{error_boundary_pattern}"
    );
    assert!(
        error_boundary_pattern.contains("source=native-parser"),
        "{error_boundary_pattern}"
    );

    let external_pattern = run_search(
        root,
        &["pattern", "public-external-type", "--dependency", "serde"],
    );
    assert!(
        external_pattern.starts_with(
            "[search-pattern] pattern=public-external-type q=serde pkg=. dep=1 own=2 api=8"
        ),
        "{external_pattern}"
    );
    assert!(
        external_pattern.contains("|api src/domain/mod.rs line=4 dep=serde kind=struct name=Thing"),
        "{external_pattern}"
    );

    let api_shape = run_search(
        root,
        &["pattern", "public-api-shape", "--owner", "src/lib.rs"],
    );
    assert!(
        api_shape.starts_with(
            "[search-pattern] pattern=public-api-shape q=src/lib.rs pkg=. own=1 item=6"
        ),
        "{api_shape}"
    );
    assert!(api_shape.contains("|item load kind=fn"), "{api_shape}");

    let docs = run_search(root, &["docs", "Thing"]);
    assert!(
        docs.starts_with("[search-docs] q=Thing pkg=. docs=1 source=native-parser"),
        "{docs}"
    );
    assert!(docs.contains("|api src/domain/mod.rs"), "{docs}");
    assert!(docs.contains("kind=struct name=Thing"), "{docs}");

    let fallible_docs = run_search(root, &["docs", "fallible"]);
    assert!(
        fallible_docs.starts_with("[search-docs] q=fallible pkg=. docs=1 source=native-parser"),
        "{fallible_docs}"
    );
    assert!(
        fallible_docs.contains(
            "|api src/lib.rs:10 kind=fn name=fallible next=owner:src/lib.rs source=native-parser docs=local-parser apiKind=fn public=true docs=true"
        ),
        "{fallible_docs}"
    );

    let api = run_search(root, &["api", "fallible"]);
    assert!(
        api.starts_with("[search-api] q=fallible pkg=. api=1 source=native-parser"),
        "{api}"
    );
    assert!(
        api.contains(
            "|api src/lib.rs:10 kind=fn name=fallible next=owner:src/lib.rs source=native-parser apiKind=fn public=true docs=true"
        ),
        "{api}"
    );

    let method_api = run_search(root, &["api", "as_thing"]);
    assert!(
        method_api.starts_with("[search-api] q=as_thing pkg=. api=1 source=native-parser"),
        "{method_api}"
    );
    assert!(
        method_api.contains(
            "kind=method name=as_thing next=owner:src/lib.rs source=native-parser apiKind=method public=true docs=false"
        ),
        "{method_api}"
    );

    let trait_method_api = run_search(root, &["api", "wire"]);
    assert!(
        trait_method_api.starts_with("[search-api] q=wire pkg=. api=1 source=native-parser"),
        "{trait_method_api}"
    );
    assert!(
        trait_method_api.contains(
            "kind=method name=wire next=owner:src/lib.rs source=native-parser apiKind=method public=true docs=false"
        ),
        "{trait_method_api}"
    );

    let external_types = run_search(root, &["public-external-types"]);
    assert!(
        external_types.starts_with("[search-public-external-types] pkg=. dep=2 hit=3"),
        "{external_types}"
    );
    assert!(
        external_types.contains(
            "|external-type src/lib.rs:11 dep=anyhow surface=return item=fallible type=anyhow::Result<Thing> source=native-parser next=dependency:anyhow,docs:anyhow::Result<Thing>"
        ),
        "{external_types}"
    );
    assert!(
        external_types.contains(
            "|external-type src/lib.rs:13 dep=serde surface=field:serializer item=PublicWire type=serde::Serialize source=native-parser next=dependency:serde,docs:serde::Serialize"
        ),
        "{external_types}"
    );

    let serde_external_types =
        run_search(root, &["public-external-types", "--dependency", "serde"]);
    assert!(
        serde_external_types.starts_with("[search-public-external-types] pkg=. dep=1 hit=1"),
        "{serde_external_types}"
    );
    assert!(
        serde_external_types.contains("dep=serde surface=field:serializer"),
        "{serde_external_types}"
    );

    let docs_use = run_search(root, &["docs-use", "load"]);
    assert!(
        docs_use.contains("[search-docs] q=load pkg=. docs=1 source=native-parser"),
        "{docs_use}"
    );
    assert!(
        docs_use.contains("[search-callsite] q=load pkg=."),
        "{docs_use}"
    );

    let dependency_docs = run_search(root, &["docs", "serde::Serialize"]);
    assert!(
        dependency_docs.starts_with(
            "[search-docs] q=serde::Serialize pkg=. docs=0 source=registry-source crate=serde"
        ),
        "{dependency_docs}"
    );
    assert!(
        dependency_docs.contains("|note docsSource=registry-source missing=true"),
        "{dependency_docs}"
    );

    let current_version_api = run_search(root, &["api", "serde@1::Serialize"]);
    assert!(
        current_version_api.starts_with(
            "[search-api] q=serde@1::Serialize pkg=. api=0 source=registry-source crate=serde requestedVersion=1 versionScope=current currentWorkspaceVersion=^1"
        ),
        "{current_version_api}"
    );

    let external_docs = run_search(root, &["docs", "serde@2::Serialize"]);
    assert!(
        external_docs.starts_with(
            "[search-docs] q=serde@2::Serialize pkg=. docs=0 source=registry-source crate=serde requestedVersion=2 versionScope=external currentWorkspaceVersion=^1"
        ),
        "{external_docs}"
    );
    assert!(
        external_docs.contains("|note docsSource=registry-source missing=true"),
        "{external_docs}"
    );
    assert!(!external_docs.contains("|api "), "{external_docs}");

    let external_api_json = run_cli([
        "search".as_ref(),
        "api".as_ref(),
        "serde@2::Serialize".as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(external_api_json.status.success(), "{external_api_json:?}");
    let value =
        serde_json::from_slice::<Value>(&external_api_json.stdout).expect("external api json");
    let header_fields = value["header"]["fields"]
        .as_object()
        .expect("header fields");
    assert_eq!(header_fields["source"], "registry-source");
    assert_eq!(header_fields["crate"], "serde");
    assert_eq!(header_fields["requestedVersion"], "2");
    assert_eq!(header_fields["versionScope"], "external");
    assert_eq!(header_fields["currentWorkspaceVersion"], "^1");
    let note_fields = value["notes"][0]["fields"]
        .as_object()
        .expect("note fields");
    assert_eq!(note_fields["apiSource"], "registry-source");

    let tests = run_search(root, &["tests", "domain"]);
    assert!(
        tests.starts_with("[search-tests] q=domain pkg=. tests=1"),
        "{tests}"
    );
    assert!(
        tests.contains("|test tests/domain.rs functions=1 next=owner:tests/domain.rs"),
        "{tests}"
    );

    let owner_tests = run_search(root, &["tests", "src/lib.rs"]);
    assert!(
        owner_tests.starts_with("[search-tests] q=src/lib.rs pkg=. tests=1 own=1"),
        "{owner_tests}"
    );
    assert!(
        owner_tests.contains("|node O:src/lib.rs kind=owner path=src/lib.rs"),
        "{owner_tests}"
    );
    assert!(
        owner_tests.contains(
            "|test tests/domain.rs functions=1 owner=src/lib.rs reason=symbol:load next=owner:tests/domain.rs"
        ),
        "{owner_tests}"
    );

    let tests_set = run_cli([
        "search".as_ref(),
        "tests".as_ref(),
        "--query-set".as_ref(),
        "src/lib.rs".as_ref(),
        "--query-set".as_ref(),
        "src/domain/mod.rs".as_ref(),
        root.as_os_str(),
    ]);
    assert!(tests_set.status.success(), "{tests_set:?}");
    let tests_set_stdout = String::from_utf8(tests_set.stdout).expect("utf8 stdout");
    assert!(
        tests_set_stdout.starts_with("[search-tests] q=src/lib.rs,src/domain/mod.rs querySet=2"),
        "{tests_set_stdout}"
    );
    assert!(
        tests_set_stdout.contains("|node O:src/lib.rs kind=owner path=src/lib.rs"),
        "{tests_set_stdout}"
    );
    assert!(
        tests_set_stdout.contains("|node O:src/domain/mod.rs kind=owner path=src/domain/mod.rs"),
        "{tests_set_stdout}"
    );
    assert!(
        owner_tests.contains("|edge O:src/lib.rs -test-> T:tests/domain.rs"),
        "{owner_tests}"
    );

    let ingest = run_search_with_stdin(
        root,
        &["ingest", "items"],
        "src/lib.rs:4:pub fn load() -> Thing\n",
    );
    assert!(
        ingest.starts_with("[search-ingest] src=rg-n in=1 own=1"),
        "{ingest}"
    );
    assert!(
        ingest.contains("|owner src/lib.rs role=source hit_kind=text locations=4:1 next=owner"),
        "{ingest}"
    );

    let ingest_seeds = run_search_with_stdin(
        root,
        &[
            "ingest", "items", "tests", "--view", "seeds", "--seeds", "8",
        ],
        "src/lib.rs:6:pub fn load() -> Thing\n",
    );
    assert!(
        ingest_seeds.starts_with("[search-ingest] root=. alg=seed-frontier"),
        "{ingest_seeds}"
    );
    assert!(
        ingest_seeds.contains("O=owner:path(src/lib.rs)!owner"),
        "{ingest_seeds}"
    );
    assert!(
        ingest_seeds.contains("T=test:path(src/lib.rs)!tests"),
        "{ingest_seeds}"
    );
    assert!(
        ingest_seeds.contains("S=symbol:symbol(load)!symbol"),
        "{ingest_seeds}"
    );
    assert!(
        !ingest_seeds.contains("|owner src/lib.rs"),
        "{ingest_seeds}"
    );
}
