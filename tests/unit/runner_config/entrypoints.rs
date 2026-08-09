use std::fs;

use rust_lang_project_harness::{
    RustHarnessConfig, run_rust_project_harness_for_scope,
    run_rust_project_harness_with_config_for_scope,
};
use tempfile::TempDir;

use super::support::{has_module_path, has_package_path, has_rule, write_manifest};

#[test]
fn default_project_runner_covers_cargo_package_rust_targets() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "default-package");
    fs::write(root.join("build.rs"), "fn main() {}\n").expect("write build script");
    fs::create_dir(root.join("examples")).expect("create examples");
    fs::write(root.join("examples/demo.rs"), "fn main() {}\n").expect("write example");
    fs::create_dir(root.join("benches")).expect("create benches");
    fs::write(root.join("benches/throughput.rs"), "fn bench_helper() {}\n").expect("write bench");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owned;\n").expect("write lib");
    fs::write(
        root.join("src/owned.rs"),
        "//! Owned module.\nfn private_api() {}\n",
    )
    .expect("write owned module");
    fs::create_dir_all(root.join("tests/unit")).expect("create unit tests");
    fs::write(root.join("tests/unit/helper.rs"), "fn helper() {}\n").expect("write helper");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(report.is_clean(), "{:?}", report.findings);
    assert_eq!(report.file_count(), 6);
    assert!(has_module_path(&report, "build.rs"));
    assert!(has_module_path(&report, "examples/demo.rs"));
    assert!(has_module_path(&report, "benches/throughput.rs"));
    assert!(has_module_path(&report, "src/lib.rs"));
    assert!(has_module_path(&report, "src/owned.rs"));
    assert!(has_module_path(&report, "tests/unit/helper.rs"));
    assert!(has_package_path(&report, "build.rs"));
    assert!(has_package_path(&report, "examples"));
    assert!(has_package_path(&report, "benches"));
}

#[test]
fn custom_source_roots_are_policy_source_roots() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "custom-source");
    fs::create_dir_all(root.join("crates/core")).expect("create custom source root");
    fs::write(
        root.join("crates/core/lib.rs"),
        "pub fn public_api() {}\n#[cfg(test)] mod tests { #[test] fn it_works() {} }\n",
    )
    .expect("write custom source");

    let config = RustHarnessConfig {
        include_tests: false,
        ..RustHarnessConfig::default()
    }
    .with_source_path("crates/core", "custom source root fixture");
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(has_rule(&report, "RUST-AGENT-PROJECT-003"));
    assert!(has_rule(&report, "RUST-AGENT-DOCS-MODULE-001"));
    assert!(has_rule(&report, "RUST-AGENT-DOCS-PUBLIC-002"));
}

#[test]
fn include_tests_false_skips_test_root_parsing_not_test_layout_policy() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "skip-tests");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nfn private_api() {}\n",
    )
    .expect("write lib");
    fs::create_dir_all(root.join("tests/unit")).expect("create unit tests");
    fs::write(root.join("tests/unit/broken.rs"), "fn broken( {\n").expect("write broken test");
    fs::write(
        root.join("tests/custom_gate.rs"),
        "rust_lang_project_harness::rust_project_harness_gate!();\n",
    )
    .expect("write custom gate");

    let default_report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run default project harness");
    assert!(has_rule(&default_report, "RUST-SYN-R001"));

    let config = RustHarnessConfig {
        include_tests: false,
        ..RustHarnessConfig::default()
    }
    .with_tests_excluded("fixture intentionally skips test-root parsing");
    let report = run_rust_project_harness_with_config_for_scope(
        root,
        &config,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(!has_rule(&report, "RUST-SYN-R001"));
    assert!(has_rule(&report, "RUST-AGENT-PROJECT-001"));
    assert_eq!(report.file_count(), 1);
    assert!(
        report
            .project_resolution
            .as_ref()
            .is_some_and(|scope| scope.test_paths.is_empty())
    );
}

#[test]
fn root_test_target_policy_rejects_top_level_test_implementation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "root-test-implementation");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir(root.join("tests")).expect("create tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "rust_lang_project_harness::rust_project_harness_gate!();\n#[test]\nfn inline_test() {}\n",
    )
    .expect("write root test target");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(has_rule(&report, "RUST-AGENT-PROJECT-007"));
}

#[test]
fn root_test_target_policy_accepts_thin_aggregate() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "root-test-aggregate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir_all(root.join("tests/unit")).expect("create unit tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "rust_lang_project_harness::rust_project_harness_gate!();\n#[path = \"unit/helper.rs\"]\nmod helper;\n",
    )
    .expect("write root test target");
    fs::write(root.join("tests/unit/helper.rs"), "fn helper() {}\n").expect("write helper");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-007"),
        "{:?}",
        report.findings
    );
}

#[test]
fn root_test_target_policy_rejects_implicit_module_mounts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "root-test-implicit-mount");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir_all(root.join("tests/unit")).expect("create unit tests");
    fs::write(
        root.join("tests/unit_test.rs"),
        "rust_lang_project_harness::rust_project_harness_gate!();\nmod helper;\n",
    )
    .expect("write root test target");
    fs::write(root.join("tests/unit/helper.rs"), "fn helper() {}\n").expect("write helper");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(has_rule(&report, "RUST-AGENT-PROJECT-008"));
}

#[test]
fn root_test_target_policy_accepts_documented_suite_mounts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "root-test-custom-suite");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::create_dir_all(root.join("tests/contract")).expect("create contract tests");
    fs::write(
        root.join("tests/rust-project-harness-rules.toml"),
        "[tests]\nallowed_directories = [\n  { name = \"contract\", explanation = \"contract suite mounted by root target\" },\n]\n",
    )
    .expect("write policy config");
    fs::write(
        root.join("tests/unit_test.rs"),
        "rust_lang_project_harness::rust_project_harness_gate!();\n#[path = \"contract/helper.rs\"]\nmod helper;\n",
    )
    .expect("write root test target");
    fs::write(root.join("tests/contract/helper.rs"), "fn helper() {}\n").expect("write helper");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(
        !has_rule(&report, "RUST-AGENT-PROJECT-008"),
        "{:?}",
        report.findings
    );
}

#[test]
fn crate_facade_policy_rejects_implementation_in_lib_rs() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "facade-implementation");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "mod owned;\nmacro_rules! local_macro { () => {} }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/owned.rs"), "//! Owned module.\n").expect("write owned module");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(has_rule(&report, "RUST-MOD-R004"));
}

#[test]
fn crate_facade_policy_accepts_proc_macro_exports() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "proc-macro-facade");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod owned;\n#[proc_macro]\npub fn export(input: proc_macro::TokenStream) -> proc_macro::TokenStream { owned::expand(input) }\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/owned.rs"),
        "//! Owned module.\npub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream { input }\n",
    )
    .expect("write owned module");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R004"), "{:?}", report.findings);
}

#[test]
fn binary_entrypoint_policy_rejects_top_level_implementation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "binary-entrypoint-implementation");
    fs::create_dir_all(root.join("src/bin")).expect("create bin dir");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(
        root.join("src/bin/tool.rs"),
        "//! Tool entrypoint.\nstruct CliOptions;\nfn main() {}\n",
    )
    .expect("write bin");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(has_rule(&report, "RUST-MOD-R005"));
}

#[test]
fn binary_entrypoint_policy_accepts_thin_entrypoint() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "binary-entrypoint-thin");
    fs::create_dir_all(root.join("src/bin")).expect("create bin dir");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n/// Run the tool.\npub fn run_tool() {}\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/bin/tool.rs"),
        "//! Tool entrypoint.\nuse binary_entrypoint_thin::run_tool;\nfn main() { run_tool(); }\n",
    )
    .expect("write bin");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(!has_rule(&report, "RUST-MOD-R005"), "{:?}", report.findings);
}

#[test]
fn build_script_policy_rejects_top_level_implementation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "build-script-implementation");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
    fs::write(
        root.join("build.rs"),
        "use std::path::Path;\nfn helper() {}\nfn main() {}\n",
    )
    .expect("write build script");

    let report = run_rust_project_harness_for_scope(
        root,
        rust_lang_project_harness::RustHarnessRunScope::Package,
    )
    .expect("run project harness");

    assert!(has_rule(&report, "RUST-MOD-R006"));
}
