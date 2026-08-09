use std::fs;

use rust_lang_project_harness::{render_rust_project_harness, run_rust_project_harness_for_scope};
use tempfile::TempDir;

use super::support::{
    create_member_crate, has_module_path, has_rule, has_rule_for_path_suffix, normalize_temp_root,
    write_manifest,
};

#[test]
fn project_runner_discovers_only_manifest_registered_member_crates() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/gated\", \"crates/inline\"]\nresolver = \"2\"\n",
    )
    .expect("write workspace manifest");
    let gated = root.join("crates/gated");
    let inline = root.join("crates/inline");
    create_member_crate(&gated, "gated");
    create_member_crate(&inline, "inline");
    fs::write(
        gated.join("src/lib.rs"),
        "//! Gated crate.\nxiuxian_testing::crate_test_policy_source_harness!(\"../tests/unit/lib_policy.rs\");\nmod owned;\n",
    )
    .expect("write gated lib");
    fs::write(gated.join("src/owned.rs"), "//! Owned module.\n").expect("write owned");
    fs::create_dir_all(gated.join("tests/unit")).expect("create gated tests");
    fs::write(
        gated.join("tests/unit/lib_policy.rs"),
        "xiuxian_testing::crate_testing_gate!();\n",
    )
    .expect("write lib policy");
    fs::write(
        gated.join("tests/unit_test.rs"),
        "xiuxian_testing::crate_test_policy_harness!();\n#[path = \"unit/lib_policy.rs\"]\nmod lib_policy;\n",
    )
    .expect("write unit target");
    fs::write(
        inline.join("src/lib.rs"),
        "//! Inline crate.\n#[cfg(test)] mod tests { #[test] fn it_works() {} }\n",
    )
    .expect("write inline lib");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::ProjectWorkspace,
    )
    .expect("run project harness");

    assert_eq!(report.workspace_member_scopes.len(), 2);
    assert!(has_module_path(&report, "crates/gated/src/lib.rs"));
    assert!(has_module_path(&report, "crates/inline/src/lib.rs"));
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-003"));
    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-009"),
        "{:?}",
        report.findings
    );

    let mut focused_report = report;
    focused_report
        .findings
        .retain(|finding| finding.rule_id == "RUST-AGENT-PROJECT-003");
    let rendered = normalize_temp_root(&render_rust_project_harness(&focused_report), root);
    insta::assert_snapshot!("workspace_member_inline_source_test", rendered);
}

#[test]
fn workspace_manifest_member_globs_are_scanned_as_member_crates() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("write workspace manifest");
    let member = root.join("crates/member");
    create_member_crate(&member, "member");
    let nested = member.join("nested");
    create_member_crate(&nested, "nested");
    fs::write(
        member.join("src/lib.rs"),
        "//! Member crate.\n#[cfg(test)] mod tests { #[test] fn it_works() {} }\n",
    )
    .expect("write member lib");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::ProjectWorkspace,
    )
    .expect("run project harness");

    assert_eq!(report.workspace_member_scopes.len(), 1);
    assert!(has_module_path(&report, "crates/member/src/lib.rs"));
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-003"));
}

#[test]
fn workspace_member_test_target_policy_accepts_crate_local_suite_mounts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/member\"]\n",
    )
    .expect("write workspace manifest");
    let member = root.join("crates/member");
    create_member_crate(&member, "member");
    fs::create_dir_all(member.join("tests/unit")).expect("create member tests");
    fs::write(
        member.join("tests/unit_test.rs"),
        "#[path = \"unit/helper.rs\"]\nmod helper;\n",
    )
    .expect("write member unit target");
    fs::write(member.join("tests/unit/helper.rs"), "fn helper() {}\n").expect("write helper");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::ProjectWorkspace,
    )
    .expect("run project harness");

    assert!(
        !has_rule_for_path_suffix(
            &report,
            "RUST-AGENT-PROJECT-008",
            "crates/member/tests/unit_test.rs"
        ),
        "{:?}",
        report.findings
    );
}

#[test]
fn include_literal_source_shards_are_reachable_from_the_module_tree() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "include-shards");
    fs::create_dir_all(root.join("src/ops")).expect("create ops dir");
    fs::write(root.join("src/lib.rs"), "//! Include crate.\nmod ops;\n").expect("write lib");
    fs::write(
        root.join("src/ops.rs"),
        "//! Ops branch.\ninclude!(\"ops/core.rs\");\n",
    )
    .expect("write ops");
    fs::write(root.join("src/ops/core.rs"), "fn core() {}\n").expect("write core shard");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::ProjectWorkspace,
    )
    .expect("run project harness");

    assert!(
        !has_rule_for_path_suffix(&report, "RUST-MOD-R009", "src/ops/core.rs"),
        "{:?}",
        report.findings
    );
}
