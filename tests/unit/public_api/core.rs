use std::fs;
use std::path::{Path, PathBuf};

use rust_lang_project_harness::{
    RustDiagnosticSeverity, RustRulePack, assert_rust_project_harness_cargo_test_clean,
    assert_rust_project_harness_cargo_test_clean_with_config, default_rust_harness_config,
    render_rust_project_harness, render_rust_project_harness_advice,
    render_rust_project_harness_agent_snapshot, render_rust_project_harness_json,
    run_rust_lang_harness, run_rust_project_harness_for_scope,
};
use tempfile::TempDir;

mod embedded_cargo_test_gate_macro_smoke {
    rust_lang_project_harness::rust_project_harness_cargo_test_gate!(
        config = {
            let mut config = rust_lang_project_harness::default_rust_harness_config();
            config.ignored_dir_names.insert("scenarios".to_string());
            config.with_verification_profile_hint(
                rust_lang_project_harness::RustVerificationProfileHint::new(
                    "src/lib.rs",
                    [rust_lang_project_harness::RustOwnerResponsibility::PublicApi],
                )
                .without_verification_tasks()
                .with_rationale("macro smoke test exercises retired cargo-test gate wiring"),
            )
        }
    );

    mod advice_allow {
        rust_lang_project_harness::rust_project_harness_cargo_test_gate!(
            advice = allow,
            config = {
                rust_lang_project_harness::default_rust_harness_config()
                    .with_verification_profile_hint(
                        rust_lang_project_harness::RustVerificationProfileHint::new(
                            "src/lib.rs",
                            [rust_lang_project_harness::RustOwnerResponsibility::PublicApi],
                        )
                        .without_verification_tasks()
                        .with_rationale(
                            "macro smoke test exercises transitional advice allowance wiring",
                        ),
                    )
                    .with_cargo_test_advice_allow_explanation(
                        "scope=cargo-test macro smoke; owner=public_api::core test; \
                         finding_category=advisory harness smoke findings; \
                         why_safe_now=the test intentionally exercises advice=allow macro wiring; \
                         cleanup_trigger=remove when the compatibility macro no longer supports advice=allow",
                    )
            }
        );
    }
}

#[test]
fn explicit_path_runner_returns_compact_report() {
    let paths = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")];

    let report = run_rust_lang_harness(&paths).expect("run harness over lib.rs");

    assert_eq!(report.file_count(), 1);
    assert!(report.parsed_count() == 1);
    let rendered = render_rust_project_harness(&report);
    assert_eq!(rendered, "[ok] rust\n");
}

#[test]
fn explicit_path_runner_is_syntax_only_without_project_scope() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("lib.rs");
    fs::write(
        &source,
        "pub fn public_api() {}\n#[cfg(test)] mod tests {}\n",
    )
    .expect("write source");

    let report = run_rust_lang_harness(&[source]).expect("run explicit path harness");

    assert!(report.is_clean());
    assert!(report.project_scope.is_none());
    assert!(report.findings.is_empty());
}

#[test]
fn explicit_path_runner_reports_unreadable_source_as_syntax_error() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("invalid_utf8.rs");
    fs::write(&source, [0xff]).expect("write invalid utf8");

    let report = run_rust_lang_harness(&[source]).expect("run explicit path harness");
    let rendered = render_rust_project_harness(&report);

    assert_eq!(report.file_count(), 1);
    assert_eq!(report.parsed_count(), 0);
    assert!(
        report
            .modules
            .first()
            .and_then(|module| module.parse_error.as_deref())
            .is_some_and(|error| error.contains("failed to read Rust source")),
        "{rendered}"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "RUST-SYN-R001"),
        "{rendered}"
    );
}

#[test]
fn advice_renderer_selects_info_findings() {
    let config = default_rust_harness_config();
    assert!(
        config
            .blocking_severities
            .contains(&RustDiagnosticSeverity::Warning)
    );

    let paths = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")];
    let report = run_rust_lang_harness(&paths).expect("run harness over lib.rs");
    let rendered = render_rust_project_harness_advice(&report);

    assert!(rendered.is_empty(), "{rendered}");
}

#[test]
fn default_renderer_keeps_info_advice_visible_without_blocking() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_advice_only_project(root, "advice-only");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    let rendered = normalize_temp_root(&render_rust_project_harness(&report), root);

    assert!(report.is_clean(), "{rendered}");
    assert!(rendered.contains("RUST-AGENT-DOCS-MODULE-001"));
    assert!(rendered.contains("RUST-AGENT-DOCS-PUBLIC-002"));
    assert!(rendered.contains("Help:"));
    assert!(rendered.contains("Contract:"));
    assert!(!rendered.contains("[ok]"), "{rendered}");
    assert!(!rendered.contains("[advice]"), "{rendered}");
    assert!(
        !rendered.contains("No blocking issues found."),
        "{rendered}"
    );
    insta::assert_snapshot!("public_api_default_advice_output", rendered);
}

#[test]
fn cargo_test_assertion_promotes_agent_advice_to_repair_feedback() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_advice_only_project(root, "advice-cargo-test");

    let panic = std::panic::catch_unwind(|| {
        assert_rust_project_harness_cargo_test_clean(root);
    })
    .expect_err("agent advice should fail cargo-test assertion");
    let normalized = normalize_temp_root(&panic_message(panic), root);

    assert!(
        normalized.contains("RUST-AGENT-DOCS-MODULE-001"),
        "{normalized}"
    );
    assert!(
        normalized.contains("RUST-AGENT-DOCS-PUBLIC-002"),
        "{normalized}"
    );
    assert!(normalized.contains("Help:"), "{normalized}");
    assert!(normalized.contains("Contract:"), "{normalized}");
    assert!(!normalized.contains("[advice]"), "{normalized}");
    assert!(
        !normalized.contains("No blocking issues found."),
        "{normalized}"
    );
}

#[test]
fn cargo_test_assertion_respects_configured_agent_pack_suppression() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_advice_only_project(root, "advice-cargo-test-waived");
    let config = default_rust_harness_config().with_disabled_rule_pack(RustRulePack::AgentPolicy);

    assert_rust_project_harness_cargo_test_clean_with_config(root, &config);
}

#[test]
fn agent_snapshot_renderer_exposes_reasoning_tree_shape() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"snapshot-shape\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\nmod leaf;\n",
    )
    .expect("write domain");
    fs::write(root.join("src/domain/leaf.rs"), "//! Domain leaf.\n").expect("write leaf");

    let rendered = normalize_temp_root(
        &render_rust_project_harness_agent_snapshot(root).expect("render snapshot"),
        root,
    );

    assert!(rendered.starts_with("Modules:"), "{rendered}");
    assert!(!rendered.contains("[agent:snapshot]"), "{rendered}");
    assert!(!rendered.contains("SourceRoots:"), "{rendered}");
    assert!(!rendered.contains("PackageEntrypoints:"), "{rendered}");
    assert!(!rendered.contains("shadowed=0"), "{rendered}");
    assert!(!rendered.contains("orphaned=0"), "{rendered}");
    assert!(
        rendered.contains("src/lib.rs [root, facade] owner=src -> mod:src/domain/mod.rs"),
        "{rendered}"
    );
    assert!(!rendered.contains("FindingGroups:"), "{rendered}");
    insta::assert_snapshot!("public_api_agent_snapshot_shape", rendered);
}

#[test]
fn json_renderer_preserves_structured_report_fields() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"json-output\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "mod owned;\npub use owned::public_api;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "pub fn public_api() {}\n").expect("write owned module");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");
    let json = render_rust_project_harness_json(&report).expect("render json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");

    assert_eq!(
        value["findings"][0]["rule_id"],
        "RUST-AGENT-DOCS-MODULE-001"
    );
    assert!(value["findings"][0]["summary"].as_str().is_some());
    assert!(value["findings"][0]["requirement"].as_str().is_some());
    assert!(value["project_scope"].is_object());
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}

fn write_advice_only_project(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "mod owned;\npub use owned::public_api;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "pub fn public_api() {}\n").expect("write owned module");
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    "<non-string panic>".to_owned()
}
