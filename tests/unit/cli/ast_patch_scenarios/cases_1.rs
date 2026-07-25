use std::ffi::OsString;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::{
    assert_receipt_verification_contains, ast_patch_scenarios_dir,
    ast_patch_target_from_search_item, copy_dir_recursive, copy_target_file_to_temp, env_flag,
    env_path, env_string, exact_read_from_target, read_json, run_ast_patch_scenario,
    run_cli_with_stdin, run_owner_item_search_json, select_search_item, snapshot_dir,
};

#[test]
fn cli_ast_patch_scenarios_match_expected_trees_and_receipts() {
    if crate::cli::support::skip_if_protocol_graph_renderer_unavailable() {
        return;
    }

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ast_patch_scenarios");
    let mut scenarios = fs::read_dir(&fixtures)
        .unwrap_or_else(|error| panic!("read fixtures {}: {error}", fixtures.display()))
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    scenarios.sort();
    assert!(
        !scenarios.is_empty(),
        "expected ast patch scenarios in {}",
        fixtures.display()
    );
    for scenario in scenarios {
        run_ast_patch_scenario(&scenario);
    }

    let provider_schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas")
        .join("rust-ast-patch-real-project-evidence.v1.schema.json");
    let root_schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| panic!("workspace root for {}", env!("CARGO_MANIFEST_DIR")))
        .join("schemas")
        .join("rust-ast-patch-real-project-evidence.v1.schema.json");
    let provider_schema = fs::read_to_string(&provider_schema_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", provider_schema_path.display()));
    if root_schema_path.exists() {
        let root_schema = fs::read_to_string(&root_schema_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", root_schema_path.display()));
        assert_eq!(
            provider_schema, root_schema,
            "Rust evidence schema copy drift"
        );
    }
    let evidence_schema = serde_json::from_str::<Value>(&provider_schema)
        .unwrap_or_else(|error| panic!("parse {}: {error}", provider_schema_path.display()));
    assert_eq!(
        evidence_schema["properties"]["schemaId"]["const"],
        "agent.semantic-protocols.rust-ast-patch-real-project-evidence"
    );

    let evidence_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ast_patch_real_projects");
    let mut evidence_files = fs::read_dir(&evidence_dir)
        .unwrap_or_else(|error| {
            panic!("read evidence fixtures {}: {error}", evidence_dir.display())
        })
        .map(|entry| entry.expect("evidence entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    evidence_files.sort();
    assert!(
        !evidence_files.is_empty(),
        "expected real-project ast patch evidence fixtures in {}",
        evidence_dir.display()
    );
    let evidence_has_event = |receipt: &Value, expected: &str| {
        receipt["verificationEvents"]
            .as_array()
            .unwrap_or_else(|| {
                panic!("real-project evidence receipt events must be an array: {receipt}")
            })
            .iter()
            .any(|event| event.as_str() == Some(expected))
    };

    for evidence_file in evidence_files {
        let evidence = read_json(&evidence_file);
        let label = evidence_file.display();
        assert_eq!(
            evidence["schemaId"], "agent.semantic-protocols.rust-ast-patch-real-project-evidence",
            "{label}"
        );
        assert_eq!(evidence["schemaVersion"], "1", "{label}");
        assert_eq!(evidence["sourceStored"], false, "{label}");
        assert_eq!(
            evidence["selectedMatch"]["patchSafetyLevel"], "ast-patch-safe",
            "{label}"
        );
        assert_eq!(
            evidence["selectedMatch"]["allowedOperations"],
            json!(["replace_item"]),
            "{label}"
        );
        assert_eq!(
            evidence["compact"]["syntax"], "save-token-rustfmt",
            "{label}"
        );
        assert_eq!(
            evidence["compact"]["whitespacePolicy"], "formatter-structural",
            "{label}"
        );
        assert_eq!(evidence["compact"]["exactReadRequired"], true, "{label}");
        let compact_bytes = evidence["compact"]["compactBytes"]
            .as_u64()
            .unwrap_or_else(|| panic!("{label}: compactBytes must be numeric"));
        let exact_bytes = evidence["compact"]["exactBytes"]
            .as_u64()
            .unwrap_or_else(|| panic!("{label}: exactBytes must be numeric"));
        assert!(
            compact_bytes < exact_bytes,
            "{label}: compact projection must save tokens: compact={compact_bytes} exact={exact_bytes}"
        );
        assert!(
            evidence["compact"]["compactToExactRatio"]
                .as_f64()
                .unwrap_or(1.0)
                < 1.0,
            "{label}: compact ratio must be less than exact source"
        );
        assert!(
            evidence["compact"]["responsibilities"]
                .as_array()
                .is_some_and(|responsibilities| responsibilities.len() >= 3),
            "{label}: real-project evidence needs multiple parser-owned responsibilities"
        );
        assert_eq!(evidence["dryRunReceipt"]["status"], "verified", "{label}");
        assert_eq!(
            evidence["dryRunReceipt"]["capability"], "provider-ast-dry-run",
            "{label}"
        );
        assert_eq!(
            evidence["dryRunReceipt"]["mutationAvailable"], false,
            "{label}"
        );
        assert!(
            evidence_has_event(&evidence["dryRunReceipt"], "file-reparsed"),
            "{label}: dry-run receipt must reparse file"
        );
        assert!(
            !evidence_has_event(&evidence["dryRunReceipt"], "file-written"),
            "{label}: dry-run receipt must not write files"
        );
        assert_eq!(evidence["applyTempReceipt"]["status"], "applied", "{label}");
        assert_eq!(
            evidence["applyTempReceipt"]["capability"], "provider-ast-apply",
            "{label}"
        );
        assert_eq!(
            evidence["applyTempReceipt"]["mutationAvailable"], true,
            "{label}"
        );
        assert!(
            evidence_has_event(&evidence["applyTempReceipt"], "formatter-output-reparsed"),
            "{label}: apply receipt must reparse rustfmt output"
        );
        assert!(
            evidence_has_event(&evidence["applyTempReceipt"], "file-written"),
            "{label}: temp apply receipt must write the temp file"
        );
    }
}

#[test]
fn cli_ast_patch_scenario_uses_search_owner_target_for_tokio_style_apply() {
    let fixture = ast_patch_scenarios_dir().join("009_tokio_style_nested_async_fn_apply");
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir_recursive(&fixture.join("input"), temp.path());

    let owner_path = "src/runtime/scheduler/multi_thread/worker.rs";
    let search_packet = run_owner_item_search_json(temp.path(), owner_path, "park_timeout");
    let item = select_search_item(
        "tokio_style_apply",
        "owner item discovery",
        &search_packet,
        Some("fn"),
        Some("park_timeout"),
    );
    assert_eq!(search_packet["method"], "search/owner", "{search_packet}");
    let target = ast_patch_target_from_search_item(item);
    let preimage = exact_read_from_target(temp.path(), &target);
    assert!(preimage.contains("pub(crate) async fn park_timeout"));
    let fixture_packet = read_json(&fixture.join("packet.json"));
    let snippet = fixture_packet["operation"]["snippet"]
        .as_str()
        .expect("fixture replacement snippet");
    let packet = json!({
        "target": target,
        "operation": {
            "op": "replace_item",
            "snippet": snippet,
            "expectedSnippet": preimage,
            "maxEdits": 1
        }
    })
    .to_string();

    let output = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from("apply"),
            OsString::from("--packet"),
            OsString::from("-"),
            temp.path().as_os_str().to_os_string(),
        ],
        &packet,
    );
    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "successful ast-patch apply should not print a receipt: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let expected = snapshot_dir(&fixture.join("expected"));
    let actual = snapshot_dir(temp.path());
    assert_eq!(
        actual, expected,
        "search-owner-derived ast-patch target should apply cleanly"
    );
}

#[test]
#[ignore = "set ASP_AST_PATCH_REAL_ROOT/PATH/QUERY to exercise a real Rust checkout"]
fn cli_ast_patch_real_checkout_search_owner_target_dry_runs_from_env() {
    let Some(root) = env_path("ASP_AST_PATCH_REAL_ROOT") else {
        eprintln!(
            "skipping real checkout ast-patch evidence; set ASP_AST_PATCH_REAL_ROOT, ASP_AST_PATCH_REAL_PATH, and ASP_AST_PATCH_REAL_QUERY"
        );
        return;
    };
    let Some(source_path) = env_string("ASP_AST_PATCH_REAL_PATH") else {
        eprintln!("skipping real checkout ast-patch evidence; ASP_AST_PATCH_REAL_PATH is unset");
        return;
    };
    let Some(query_term) = env_string("ASP_AST_PATCH_REAL_QUERY") else {
        eprintln!("skipping real checkout ast-patch evidence; ASP_AST_PATCH_REAL_QUERY is unset");
        return;
    };

    let search_packet = run_owner_item_search_json(&root, &source_path, &query_term);
    let target_kind = env_string("ASP_AST_PATCH_REAL_TARGET_KIND");
    let target_name = env_string("ASP_AST_PATCH_REAL_TARGET_NAME");
    let item = select_search_item(
        "real_checkout_dry_run",
        "owner item discovery",
        &search_packet,
        target_kind.as_deref(),
        target_name.as_deref(),
    );
    let target = ast_patch_target_from_search_item(item);
    let preimage = exact_read_from_target(&root, &target);
    assert!(!preimage.trim().is_empty(), "empty preimage from {target}");
    let packet = json!({
        "target": target,
        "operation": {
            "op": "replace_item",
            "snippet": preimage,
            "expectedSnippet": preimage,
            "maxEdits": 1
        }
    })
    .to_string();

    let dry_run = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from("dry-run"),
            OsString::from("--packet"),
            OsString::from("-"),
            root.as_os_str().to_os_string(),
        ],
        &packet,
    );
    assert!(dry_run.status.success(), "{dry_run:?}");
    let receipt = serde_json::from_slice::<Value>(&dry_run.stdout).expect("dry-run receipt JSON");
    assert_eq!(receipt["status"], "verified", "{receipt}");
    assert_receipt_verification_contains("real_checkout_dry_run", &receipt, "file-reparsed");

    if env_flag("ASP_AST_PATCH_REAL_APPLY_TEMP") {
        let temp = tempfile::tempdir().expect("tempdir");
        copy_target_file_to_temp(&root, temp.path(), &source_path);
        let apply = run_cli_with_stdin(
            [
                OsString::from("ast-patch"),
                OsString::from("apply"),
                OsString::from("--packet"),
                OsString::from("-"),
                temp.path().as_os_str().to_os_string(),
            ],
            &packet,
        );
        assert!(apply.status.success(), "{apply:?}");
        let receipt = serde_json::from_slice::<Value>(&apply.stdout).expect("apply receipt JSON");
        assert_eq!(receipt["status"], "applied", "{receipt}");
        assert_receipt_verification_contains(
            "real_checkout_apply_temp",
            &receipt,
            "formatter-output-reparsed",
        );
    }
}
