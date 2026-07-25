//! Semantic-language registry JSON for agent capability discovery.

use std::path::Path;

use serde_json::{Value, json};

pub(super) fn print_agent_registry(project_root: &Path) -> Result<(), String> {
    let registry = agent_registry_json(project_root);
    println!(
        "{}",
        serde_json::to_string(&registry)
            .map_err(|error| format!("failed to render agent registry JSON: {error}"))?
    );
    Ok(())
}

fn rust_query_pack_descriptor() -> Value {
    let manifest: Value =
        serde_json::from_str(include_str!("../../provider/asp-provider-manifest.json"))
            .expect("embedded Rust provider manifest must be valid JSON");
    manifest["queryPackDescriptor"].clone()
}

fn rust_tree_sitter_query_catalogs() -> Vec<Value> {
    const EMBEDDED_QUERY_SOURCES: [(&str, &str); 5] = [
        (
            "declarations",
            include_str!("../../tree-sitter/tree-sitter-rust/queries/declarations.scm"),
        ),
        (
            "imports",
            include_str!("../../tree-sitter/tree-sitter-rust/queries/imports.scm"),
        ),
        (
            "calls",
            include_str!("../../tree-sitter/tree-sitter-rust/queries/calls.scm"),
        ),
        (
            "macros",
            include_str!("../../tree-sitter/tree-sitter-rust/queries/macros.scm"),
        ),
        (
            "cfg",
            include_str!("../../tree-sitter/tree-sitter-rust/queries/cfg.scm"),
        ),
    ];
    let profile: Value = serde_json::from_str(include_str!(
        "../../tree-sitter/tree-sitter-rust/grammar-profile.json"
    ))
    .expect("embedded Rust grammar profile must be valid JSON");
    profile["catalogs"]
        .as_array()
        .expect("embedded Rust grammar profile must declare catalogs")
        .iter()
        .map(|catalog| {
            let mut descriptor = catalog.clone();
            let catalog_id = descriptor["id"]
                .as_str()
                .expect("embedded Rust query catalog must declare an id");
            let source = EMBEDDED_QUERY_SOURCES
                .iter()
                .find_map(|(id, source)| (*id == catalog_id).then_some(*source))
                .expect("every Rust query catalog must have an embedded source");
            assert!(
                !source.trim().is_empty(),
                "embedded Rust query source is empty"
            );
            let fields = descriptor
                .as_object_mut()
                .expect("embedded Rust query catalog must be an object");
            fields.insert(
                "sourceDelivery".to_string(),
                Value::String("provider-binary-embedded".to_string()),
            );
            fields.insert("sourceByteLength".to_string(), json!(source.len()));
            descriptor
        })
        .collect()
}

fn agent_registry_json(project_root: &Path) -> Value {
    let search_methods = [
        "workspace",
        "prime",
        "owner",
        "dependency",
        "deps",
        "features",
        "targets",
        "symbol",
        "callsite",
        "import",
        "query",
        "tests",
        "cfg",
        "patterns",
        "pattern",
        "docs",
        "docs-use",
        "api",
        "public-external-types",
        "policy",
        "semantic-facts",
        "workspace-scope",
        "ingest",
        "compare",
    ];
    let mut methods = search_methods
        .iter()
        .map(|view| format!("search/{view}"))
        .chain([
            "agent/doctor".to_string(),
            "agent/guide".to_string(),
            "ast-patch/apply".to_string(),
            "ast-patch/dry-run".to_string(),
            "check/changed".to_string(),
            "check/full".to_string(),
            "evidence/assurance".to_string(),
            "evidence/graph".to_string(),
            "proof/pilot".to_string(),
            "query".to_string(),
            "query/exact-selector".to_string(),
            "query/owner-items".to_string(),
            "review/packet".to_string(),
            "verification/performance-index".to_string(),
            "verification/stability-index".to_string(),
        ])
        .collect::<Vec<_>>();
    methods.sort();

    let mut method_descriptors = search_methods
        .iter()
        .map(|view| search_method_descriptor(view))
        .collect::<Vec<_>>();
    method_descriptors.extend([
        json!({
            "method": "query",
            "command": "query",
            "input": "catalog-id",
            "requiredOptions": ["--catalog|--treesitter-query"],
            "outputSchemaIds": ["agent.semantic-protocols.semantic-tree-sitter-query"],
            "packetSchemas": ["semantic-tree-sitter-query.v1"],
            "supportsJson": true,
            "supportsCompact": true,
            "outputModes": ["frontier", "json", "code"],
            "queryInputForms": ["selector", "code-shaped", "catalog-id", "s-expression"],
            "grammarId": "tree-sitter-rust",
            "grammarProfileVersion": "2026-06-04.v1",
            "grammarProfileSchema": "semantic-tree-sitter-grammar-profile.v1",
            "grammarProfilePath": "tree-sitter/tree-sitter-rust/grammar-profile.json",
            "adapterModes": ["native-projection"],
            "sourceAuthorities": ["native-parser-adapter", "native-parser"],
            "executionBackends": ["native-parser"],
            "renderProfiles": ["compact-graph-frontier"],
            "queryCatalogs": rust_tree_sitter_query_catalogs(),
            "supportedPredicates": ["#eq?", "#any-eq?", "#any-of?", "#match?", "#any-match?", "#not-eq?", "#not-match?"],
            "unsupportedPredicates": [],
            "cacheReplay": false,
            "codeOutput": { "mode": "pure-code", "multiMatch": "deny", "requires": ["exact-selector", "unique-predicate"] },
            "unsupportedPatternBehavior": "diagnostic"
        }),
        json!({
            "acceptedQuerySetSelectors": ["exact-set"],
            "cacheReplay": true,
            "command": "query",
            "input": "owner-path",
            "method": "query/owner-items",
            "adapterModes": ["native-projection"],
            "executionBackends": ["native-parser"],
            "outputModes": ["frontier", "json", "code", "names"],
            "outputSchemaIds": ["agent.semantic-protocols.semantic-query-packet"],
            "packetSchemas": ["semantic-query-packet.v1"],
            "queryInputForms": ["selector", "code-shaped"],
            "querySetScopes": ["owner"],
            "renderProfiles": ["compact-graph-frontier"],
            "requiredOptions": ["--term"],
            "sourceAuthorities": ["native-parser"],
            "unsupportedPatternBehavior": "diagnostic",
            "codeOutput": { "mode": "pure-code", "multiMatch": "deny", "requires": ["exact-selector", "unique-match"] },
            "supportsCompact": true,
            "supportsJson": true,
            "supportsQuerySet": true
        }),
        json!({
            "cacheReplay": true,
            "command": "query",
            "method": "query/exact-selector",
            "adapterModes": ["native-projection"],
            "executionBackends": ["native-parser"],
            "outputModes": ["frontier", "json", "code", "names"],
            "outputSchemaIds": [
                "agent.semantic-protocols.semantic-query-packet"
            ],
            "packetSchemas": [
                "semantic-query-packet.v1",
                "semantic-read-packet.v1"
            ],
            "queryInputForms": ["selector"],
            "renderProfiles": ["corpus-locator"],
            "requiredOptions": ["--selector"],
            "sourceAuthorities": ["native-parser"],
            "unsupportedPatternBehavior": "diagnostic",
            "codeOutput": { "mode": "pure-code", "multiMatch": "deny", "requires": ["exact-selector"] },
            "supportsCompact": true,
            "supportsJson": true
        }),
        json!({ "command": "check", "method": "check/changed", "supportsCompact": true, "supportsJson": true }),
        json!({ "command": "check", "method": "check/full", "supportsCompact": true, "supportsJson": true }),
        json!({
            "command": "proof",
            "input": "pilot",
            "method": "proof/pilot",
            "outputSchemaIds": ["agent.semantic-protocols.semantic-formal-proof-pilot"],
            "supportsCompact": true,
            "supportsJson": true
        }),
        json!({
            "command": "review",
            "input": "packet",
            "method": "review/packet",
            "outputSchemaIds": ["agent.semantic-protocols.semantic-review-packet"],
            "supportsCompact": true,
            "supportsJson": true
        }),
        json!({
            "command": "verification",
            "input": "performance-index",
            "method": "verification/performance-index",
            "supportsCompact": true,
            "supportsJson": true
        }),
        json!({
            "command": "verification",
            "input": "stability-index",
            "method": "verification/stability-index",
            "supportsCompact": true,
            "supportsJson": true
        }),
        json!({
            "command": "evidence",
            "input": "graph",
            "method": "evidence/graph",
            "outputSchemaIds": ["agent.semantic-protocols.semantic-evidence-graph"],
            "requiredOptions": ["--review-packet-json"],
            "supportsCompact": true,
            "supportsJson": true
        }),
        json!({
            "command": "evidence",
            "input": "assurance",
            "method": "evidence/assurance",
            "outputSchemaIds": ["agent.semantic-protocols.semantic-assurance-case"],
            "requiredOptions": ["--evidence-graph-json"],
            "supportsCompact": true,
            "supportsJson": true
        }),
        json!({
            "command": "ast-patch",
            "input": "dry-run",
            "method": "ast-patch/dry-run",
            "mutationAvailable": false,
            "outputSchemaIds": ["agent.semantic-protocols.semantic-ast-patch-receipt"],
            "requiredOptions": ["--packet"],
            "supportsCompact": false,
            "supportsJson": true
        }),
        json!({
            "command": "ast-patch",
            "input": "apply",
            "method": "ast-patch/apply",
            "mutationAvailable": true,
            "outputSchemaIds": ["agent.semantic-protocols.semantic-ast-patch-receipt"],
            "requiredOptions": ["--packet"],
            "supportsCompact": false,
            "supportsJson": true
        }),
        json!({
            "clients": ["codex"],
            "command": "agent",
            "method": "agent/doctor",
            "outputSchemaIds": ["agent.semantic-protocols.semantic-language-registry"],
            "supportsCompact": true,
            "supportsJson": true
        }),
        json!({
            "clients": ["codex"],
            "command": "agent",
            "method": "agent/guide",
            "supportsCompact": true,
            "supportsJson": false
        }),
    ]);
    for descriptor in &mut method_descriptors {
        let invocation = method_invocation(descriptor);
        if let Value::Object(fields) = descriptor {
            fields.insert("invocation".to_string(), invocation);
        }
    }

    json!({
        "registryId": "agent.semantic-protocols.semantic-language-registry",
        "registryVersion": "1",
        "protocolId": "agent.semantic-protocols.semantic-language",
        "protocolVersion": "1",
        "projectRoot": display_absolute_cli_path(project_root),
        "languages": [{
            "languageId": "rust",
            "providerId": "rs-harness",
            "binary": "rs-harness",
            "namespace": "agent.semantic-protocols.languages.rust.rs-harness",
            "displayName": "Rust Harness",
            "methods": methods,
            "methodDescriptors": method_descriptors,
            "queryPackDescriptor": rust_query_pack_descriptor(),
            "schemas": [
                { "path": "schemas/semantic-language-registry.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-language-registry", "schemaVersion": "1" },
                { "path": "schemas/semantic-search-packet.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-search-packet", "schemaVersion": "1" },
                { "path": "schemas/semantic-compare-packet.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-compare-packet", "schemaVersion": "1" },
                { "path": "schemas/semantic-query-packet.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-query-packet", "schemaVersion": "1" },
                { "path": "schemas/semantic-tree-sitter-query.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-tree-sitter-query", "schemaVersion": "1" },
                { "path": "schemas/semantic-tree-sitter-grammar-profile.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-tree-sitter-grammar-profile", "schemaVersion": "1" },
                { "path": "schemas/semantic-relation-plan.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-relation-plan", "schemaVersion": "1" },
                { "path": "schemas/semantic-flow-lite.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-flow-lite", "schemaVersion": "1" },
                { "path": "schemas/semantic-codeql-evidence.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-codeql-evidence", "schemaVersion": "1" },
                { "path": "schemas/semantic-source-location.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-source-location", "schemaVersion": "1" },
                { "path": "schemas/semantic-tree-sitter-provenance.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-tree-sitter-provenance", "schemaVersion": "1" },
                { "path": "schemas/semantic-read-packet.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-read-packet", "schemaVersion": "1" },
                { "path": "schemas/semantic-graph.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-graph", "schemaVersion": "1" },
                { "path": "schemas/semantic-fact-graph.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-fact-graph", "schemaVersion": "1" },
                { "path": "schemas/semantic-fact-ontology.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-fact-ontology", "schemaVersion": "1" },
                { "path": "schemas/semantic-workspace-scope.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-workspace-scope", "schemaVersion": "1" },
                { "path": "schemas/semantic-type-surface.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-type-surface", "schemaVersion": "1" },
                { "path": "schemas/semantic-invariant-candidate.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-invariant-candidate", "schemaVersion": "1" },
                { "path": "schemas/semantic-verification-receipt.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-verification-receipt", "schemaVersion": "1" },
                { "path": "schemas/semantic-behavior-snapshot.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-behavior-snapshot", "schemaVersion": "1" },
                { "path": "schemas/semantic-determinism-readiness.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-determinism-readiness", "schemaVersion": "1" },
                { "path": "schemas/semantic-dev-command-log.v1.schema.json", "schemaId": "agent.semantic-protocols.dev-command-log", "schemaVersion": "1" },
                { "path": "schemas/semantic-formal-proof-pilot.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-formal-proof-pilot", "schemaVersion": "1" },
                { "path": "schemas/semantic-review-packet.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-review-packet", "schemaVersion": "1" },
                { "path": "schemas/semantic-evidence-graph.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-evidence-graph", "schemaVersion": "1" },
                { "path": "schemas/semantic-assurance-case.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-assurance-case", "schemaVersion": "1" },
                { "path": "schemas/semantic-handle.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-handle", "schemaVersion": "1" },
                { "path": "schemas/semantic-native-syntax-fact-index.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-native-syntax-fact-index", "schemaVersion": "1" },
                { "path": "schemas/semantic-ast-patch.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-ast-patch", "schemaVersion": "1" },
                { "path": "schemas/semantic-ast-patch-receipt.v1.schema.json", "schemaId": "agent.semantic-protocols.semantic-ast-patch-receipt", "schemaVersion": "1" },
                { "schemaId": "agent.semantic-protocols.rust-ast-patch-real-project-evidence", "schemaVersion": "1", "path": "schemas/rust-ast-patch-real-project-evidence.v1.schema.json" },
                { "path": "schemas/rust-semantic-capabilities.v1.schema.json", "schemaId": "agent.semantic-protocols.languages.rust.rs-harness.capabilities", "schemaVersion": "1" }
            ]
        }]
    })
}

fn method_invocation(descriptor: &Value) -> Value {
    let method = descriptor
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let command = descriptor
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut argv = vec!["rs-harness".to_string()];
    if let Some(args) = descriptor
        .get("benchmarkInvocation")
        .and_then(|benchmark| benchmark.get("args"))
        .and_then(Value::as_array)
    {
        argv.extend(args.iter().filter_map(Value::as_str).map(str::to_string));
    } else {
        match method {
            "query/owner-items" => argv.extend([
                "query".to_string(),
                "--from-hook".to_string(),
                "item-skeleton".to_string(),
                "--selector".to_string(),
                "{selector}".to_string(),
                "--term".to_string(),
                "{query}".to_string(),
                "--workspace".to_string(),
                "{workspace}".to_string(),
            ]),
            "query/exact-selector" => argv.extend([
                "query".to_string(),
                "--selector".to_string(),
                "{owner}".to_string(),
                "--workspace".to_string(),
                "{workspace}".to_string(),
                "--view".to_string(),
                "seeds".to_string(),
            ]),
            _ => {
                argv.push(command.to_string());
                if let Some((_, action)) = method.split_once('/') {
                    argv.push(action.to_string());
                }
            }
        }
    }
    let stdin_mode = if descriptor
        .get("acceptsStdin")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "pipe-candidates"
    } else {
        "none"
    };
    json!({
        "argv": argv,
        "stdinMode": stdin_mode,
    })
}

fn search_method_descriptor(view: &str) -> Value {
    let mut descriptor = json!({
        "method": format!("search/{view}"),
        "command": "search",
        "view": view,
        "outputSchemaIds": search_output_schema_ids(view),
        "requiresQuery": search_view_requires_query(view),
        "acceptsStdin": view == "ingest",
        "supportsPackageScope": true,
        "benchmarkInvocation": search_benchmark_invocation(view),
        "supportsJson": true,
        "supportsCompact": true
    });
    let Value::Object(fields) = &mut descriptor else {
        return descriptor;
    };
    let accepted_pipes = accepted_search_pipes(view);
    if !accepted_pipes.is_empty() {
        fields.insert("acceptedPipes".to_string(), json!(accepted_pipes));
    }
    if search_view_supports_query_set(view) {
        fields.insert("supportsQuerySet".to_string(), json!(true));
        fields.insert(
            "acceptedQuerySetSelectors".to_string(),
            json!(search_query_set_selectors(view)),
        );
    }
    let capabilities = search_capabilities(view);
    if !capabilities.is_empty() {
        fields.insert("capabilities".to_string(), json!(capabilities));
    }
    if view == "owner" {
        fields.insert("cacheReplay".to_string(), json!(true));
        fields.insert("executionBackends".to_string(), json!(["native-parser"]));
        fields.insert(
            "packetSchemas".to_string(),
            json!(["semantic-search-packet.v1"]),
        );
    }
    if view == "semantic-facts" {
        fields.insert("acceptsStdin".to_string(), json!(true));
        fields.insert("supportsCompact".to_string(), json!(false));
        fields.insert(
            "outputSchemaIds".to_string(),
            json!(["agent.semantic-protocols.semantic-fact-graph"]),
        );
        fields.insert(
            "packetSchemas".to_string(),
            json!(["semantic-fact-graph.v1", "semantic-fact-ontology.v1"]),
        );
        fields.insert("outputModes".to_string(), json!(["json"]));
        fields.insert("input".to_string(), json!("search semantic-facts <query>"));
    }
    if view == "workspace-scope" {
        fields.insert("supportsCompact".to_string(), json!(false));
        fields.insert(
            "outputSchemaIds".to_string(),
            json!(["agent.semantic-protocols.semantic-workspace-scope"]),
        );
        fields.insert(
            "packetSchemas".to_string(),
            json!(["semantic-workspace-scope.v1"]),
        );
        fields.insert("outputModes".to_string(), json!(["json"]));
        fields.insert("input".to_string(), json!("search workspace-scope"));
    }
    let ingest_required_for = search_ingest_required_for(view);
    if !ingest_required_for.is_empty() {
        fields.insert("ingestRequiredFor".to_string(), json!(ingest_required_for));
    }
    descriptor
}

fn search_benchmark_invocation(view: &str) -> Value {
    match view {
        "owner" => json!({
            "args": [
                "search", "owner", "{owner}", "items", "--query", "{query}",
                "--workspace", "{workspace}", "--view", "seeds"
            ],
            "expectsJson": false,
            "maxElapsedMs": 15_000,
        }),
        "lexical" => json!({
            "args": [
                "search", "lexical", "--query", "{query}", "--query", "{dependency}",
                "--workspace", "{workspace}", "--view", "seeds"
            ],
            "expectsJson": false,
            "maxElapsedMs": 15_000,
        }),
        "dependency" | "deps" => json!({
            "args": ["search", view, "{dependency}", "--workspace", "{workspace}", "--view", "seeds"],
            "expectsJson": false,
            "maxElapsedMs": 15_000,
        }),
        "tests" => json!({
            "args": ["search", "tests", "{owner}", "--workspace", "{workspace}", "--view", "seeds"],
            "expectsJson": false,
            "maxElapsedMs": 15_000,
        }),
        "ingest" => json!({
            "args": ["search", "ingest", "--workspace", "{workspace}", "--view", "seeds"],
            "stdinTemplate": "{owner}:1:{query}\\n",
            "expectsJson": false,
            "maxElapsedMs": 15_000,
        }),
        "semantic-facts" => json!({
            "args": ["search", "semantic-facts", "{query}", "--workspace", "{workspace}", "--json"],
            "expectsJson": true,
            "maxElapsedMs": 15_000,
        }),
        "workspace-scope" => json!({
            "args": ["search", "workspace-scope", "--workspace", "{workspace}", "--json"],
            "expectsJson": true,
            "maxElapsedMs": 100,
        }),
        "api" | "callsite" | "cfg" | "compare" | "docs" | "docs-use" | "import" | "pattern"
        | "policy" | "query" | "symbol" => json!({
            "args": ["search", view, "{query}", "--workspace", "{workspace}", "--view", "seeds"],
            "expectsJson": false,
            "maxElapsedMs": 15_000,
        }),
        _ => json!({
            "args": ["search", view, "--workspace", "{workspace}", "--view", "seeds"],
            "expectsJson": false,
            "maxElapsedMs": 15_000,
        }),
    }
}

fn search_output_schema_ids(view: &str) -> Vec<&'static str> {
    let mut schema_ids = vec!["agent.semantic-protocols.semantic-search-packet"];
    if view == "public-external-types" {
        schema_ids.push("agent.semantic-protocols.semantic-type-surface");
    }
    if view == "policy" {
        schema_ids.push("agent.semantic-protocols.semantic-handle");
    }
    if view == "query" {
        schema_ids.push("agent.semantic-protocols.semantic-native-syntax-fact-index");
    }
    if view == "compare" {
        schema_ids.push("agent.semantic-protocols.semantic-compare-packet");
    }
    schema_ids
}

fn search_view_requires_query(view: &str) -> bool {
    matches!(
        view,
        "owner"
            | "policy"
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
            | "semantic-facts"
            | "compare"
    )
}

fn search_view_supports_query_set(view: &str) -> bool {
    matches!(view, "owner" | "dependency" | "tests")
}

fn search_query_set_selectors(_view: &str) -> Vec<&'static str> {
    vec!["exact-set"]
}

fn accepted_search_pipes(view: &str) -> Vec<&'static str> {
    match view {
        "owner" => vec!["items", "tests"],
        "policy" => vec!["owner", "tests"],
        "dependency" => vec!["items", "public-api", "docs", "tests"],
        "deps" => vec!["public-api"],
        "features" => vec!["cfg", "owners", "tests"],
        "query" => vec!["owner", "tests"],
        "ingest" => vec!["items", "tests"],
        _ => Vec::new(),
    }
}

fn search_capabilities(view: &str) -> Vec<Value> {
    match view {
        "workspace" => vec![
            semantic_capability("workspace-router"),
            rust_capability("cargo-workspace-search"),
        ],
        "prime" => vec![
            semantic_capability("package-prime-map"),
            rust_capability("rust-module-tree-prime"),
        ],
        "owner" => vec![
            semantic_capability("reasoning-owner-search"),
            rust_capability("parser-visible-module-owner-search"),
            rust_capability("test-owner-search"),
            rust_capability("rust-native-syntax-query"),
            semantic_capability("path-owner-fallback"),
        ],
        "dependency" => vec![
            semantic_capability("dependency-manifest-search"),
            rust_capability("dependency-local-usage-search"),
        ],
        "deps" => vec![
            semantic_capability("dependency-manifest-search"),
            rust_capability("dependency-local-usage-search"),
            semantic_capability("dependency-version-scope"),
            rust_capability("dependency-api-token-usage-search"),
        ],
        "features" => vec![rust_capability("cargo-feature-search")],
        "targets" => vec![rust_capability("cargo-target-search")],
        "cfg" => vec![rust_capability("cargo-cfg-search")],
        "symbol" => vec![rust_capability("symbol-definition-search")],
        "callsite" => vec![rust_capability("owner-callsite-search")],
        "import" => vec![rust_capability("import-edge-search")],
        "query" => vec![
            semantic_capability("code-shaped-query-routing"),
            rust_capability("rust-native-syntax-query"),
        ],
        "tests" => vec![rust_capability("test-owner-search")],
        "lexical" => vec![
            semantic_capability("lexical-candidate-search"),
            rust_capability("parser-visible-source-lexical-search"),
        ],
        "patterns" | "pattern" => vec![rust_capability("pattern-recipe-search")],
        "docs" => vec![rust_capability("native-docs-api-search")],
        "docs-use" => vec![
            rust_capability("native-docs-api-search"),
            rust_capability("owner-callsite-search"),
        ],
        "api" => vec![rust_capability("native-api-shape-search")],
        "public-external-types" => vec![rust_capability("public-external-type-search")],
        "policy" => vec![
            semantic_capability("policy-rule-handle-search"),
            rust_capability("rust-project-policy-rule-handle-search"),
            rust_capability("rust-agent-policy-rule-handle-search"),
        ],
        "semantic-facts" => vec![
            semantic_capability("graph-turbo-provider-facts"),
            rust_capability("rust-syn-field-type-collection-facts"),
        ],
        "workspace-scope" => vec![
            semantic_capability("package-manager-workspace-scope"),
            rust_capability("cargo-workspace-membership"),
        ],
        "ingest" => vec![
            semantic_capability("external-candidate-ingest"),
            semantic_capability("stdin-shape-detection"),
            semantic_capability("owner-grouped-ingest"),
        ],
        _ => Vec::new(),
    }
}

fn search_ingest_required_for(view: &str) -> Vec<Value> {
    match view {
        "owner" => vec![rust_ingest_surface("non-parser-path")],
        "lexical" => vec![
            rust_ingest_surface("non-parser-text"),
            rust_ingest_surface("docs-text"),
            rust_ingest_surface("schema-json"),
            rust_ingest_surface("generated-artifact"),
        ],
        _ => Vec::new(),
    }
}

fn semantic_capability(name: &str) -> Value {
    capability("semantic", name)
}

fn rust_capability(name: &str) -> Value {
    capability("rust", name)
}

fn rust_ingest_surface(name: &str) -> Value {
    capability("rust", name)
}

fn capability(namespace: &str, name: &str) -> Value {
    json!({
        "languageId": "rust",
        "namespace": namespace,
        "name": name
    })
}

fn display_absolute_cli_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
