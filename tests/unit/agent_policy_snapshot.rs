use std::fs;
use std::path::Path;

use asp_rust::{render_asp_rust, run_asp_rust_for_scope};
use tempfile::TempDir;

#[path = "agent_policy_snapshot/primitive_api.rs"]
mod primitive_api;

#[path = "agent_policy_snapshot/error_boundary.rs"]
mod error_boundary;

#[path = "agent_policy_snapshot/test_support_reexport.rs"]
mod test_support_reexport;

#[path = "agent_policy_snapshot/algorithm_shape.rs"]
mod algorithm_shape;

#[test]
fn agent_policy_mod_exports_have_explicit_test_coverage() {
    let policy_mod = include_str!("../../src/rules/agent_policy/mod.rs");
    let exported_modules = policy_mod
        .lines()
        .filter_map(agent_policy_module_name)
        .collect::<Vec<_>>();
    let covered_modules = [
        "algorithm_shape",
        "api_shape",
        "data_shape",
        "dependency_graph",
        "doc_boundary",
        "native_abi",
        "pack",
        "process_command",
        "scenario_requirements",
        "source_surface",
        "tokio_runtime",
    ];

    assert!(
        !exported_modules.is_empty(),
        "agent policy mod.rs must expose policy modules"
    );
    for module in &exported_modules {
        assert!(
            covered_modules.contains(module),
            "agent policy module `{module}` must be assigned to an explicit unit-test lane"
        );
    }
    for module in covered_modules {
        assert!(
            exported_modules.contains(&module),
            "coverage map contains stale agent policy module `{module}`"
        );
    }
}

#[test]
fn path_policy_agent_has_process_command_lane() {
    let agent_tests = include_str!("path_policy/agent.rs");
    assert!(
        agent_tests.contains("#[path = \"agent/process_command.rs\"]")
            && agent_tests.contains("mod process_command;"),
        "path_policy/agent.rs must register a process_command lane"
    );
}

fn agent_policy_module_name(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let module = trimmed
        .strip_prefix("mod ")
        .or_else(|| trimmed.strip_prefix("pub(crate) mod "))?
        .strip_suffix(';')?;
    Some(module)
}

#[test]
fn agent_r001_public_module_intent_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r001-intent");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "pub fn public_api() {}\n").expect("write lib");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-DOCS-MODULE-001",
        1,
        "agent_r001_public_module_intent",
    );
}

#[test]
fn agent_r002_public_item_doc_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r002-doc");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod owned;\n").expect("write lib");
    fs::write(
        root.join("src/owned.rs"),
        "//! Owned module.\npub struct MissingDoc;\n",
    )
    .expect("write owned");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-DOCS-PUBLIC-002",
        1,
        "agent_r002_public_item_doc",
    );
}

#[test]
fn agent_r003_repeated_namespace_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r003-namespace");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain branch.\nmod domain;\n",
    )
    .expect("write branch");
    fs::write(
        root.join("src/domain/domain.rs"),
        "//! Repeated domain namespace.\nfn local() {}\n",
    )
    .expect("write repeated");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-SOURCE-NAMESPACE-003",
        1,
        "agent_r003_repeated_namespace",
    );
}

#[test]
fn agent_r004_public_name_conflict_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r004-conflict");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha.rs"),
        "//! Alpha owner.\n/// Alpha handle.\npub struct Handle;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta.rs"),
        "//! Beta owner.\n/// Beta handle.\npub struct Handle;\n",
    )
    .expect("write beta");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-API-NAME-004",
        2,
        "agent_r004_public_name_conflict",
    );
}

#[test]
fn agent_r005_facade_reexports_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r005-reexports");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), facade_reexports()).expect("write lib");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-API-FACADE-005",
        1,
        "agent_r005_facade_reexports",
    );
}

#[test]
fn agent_r006_generic_public_module_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r006-public-module");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n/// Shared utility bucket.\npub mod utils;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/utils.rs"), "//! Utility bucket.\n").expect("write utils");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-SOURCE-MODULE-006",
        1,
        "agent_r006_generic_public_module",
    );
}

#[test]
fn agent_r007_generic_module_path_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r007-path");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod helpers;\n").expect("write lib");
    fs::write(root.join("src/helpers.rs"), "//! Helper bucket.\n").expect("write helpers");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-SOURCE-PATH-007",
        1,
        "agent_r007_generic_module_path",
    );
}

#[test]
fn agent_r008_branch_intent_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r008-branch");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(root.join("src/domain.rs"), "mod parse;\nmod render;\n").expect("write domain");
    fs::write(root.join("src/domain/parse.rs"), "//! Parse leaf.\n").expect("write parse");
    fs::write(root.join("src/domain/render.rs"), "//! Render leaf.\n").expect("write render");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-DOCS-BRANCH-008",
        1,
        "agent_r008_branch_intent",
    );
}

#[test]
fn agent_r009_owner_dependency_cycle_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r009-cycle");
    write_owner_cycle_fixture(root);

    assert_agent_snapshot(
        root,
        "RUST-AGENT-OWNER-GRAPH-009",
        1,
        "agent_r009_owner_dependency_cycle",
    );
}

#[test]
fn agent_r010_cross_owner_leaf_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r010-leaf");
    write_cross_owner_leaf_fixture(root);

    assert_agent_snapshot(
        root,
        "RUST-AGENT-OWNER-BOUNDARY-010",
        1,
        "agent_r010_cross_owner_leaf",
    );
}

#[test]
fn agent_r011_owner_fan_out_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-r011-fan-out");
    write_owner_fan_out_fixture(root);

    assert_agent_snapshot(
        root,
        "RUST-AGENT-DOCS-OWNER-FANOUT-011",
        1,
        "agent_r011_owner_fan_out",
    );
}

#[test]
fn rust_agent_tokio_runtime_boundary_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "rust-agent-tokio-runtime-boundary");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n\
         pub struct TokioAgentRuntime;\n\
         impl TokioAgentRuntime {\n\
             pub fn new() -> Self { Self }\n\
         }\n\
         pub fn uses_facade() {\n\
             let _runtime = TokioAgentRuntime::new();\n\
         }\n\
         pub fn block_on() {\n\
             let _runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();\n\
         }\n",
    )
    .expect("write lib");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-TOKIO-RUNTIME-002",
        1,
        "rust_agent_tokio_runtime_boundary",
    );
}

#[test]
fn rust_agent_native_abi_contract_snapshot() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "rust-agent-native-abi-contract");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n\
         pub mod native_abi;\n\
         pub mod owned_abi;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/native_abi.rs"),
        "//! Native ABI owner missing its contract constants.\n\
         #[repr(C)]\n\
         pub struct NativeUtf8 {\n\
             pub ptr: *const u8,\n\
             pub len: usize,\n\
         }\n",
    )
    .expect("write native abi");
    fs::write(
        root.join("src/owned_abi.rs"),
        "//! Native ABI owner with its contract constants.\n\
         pub const OWNED_ABI_VERSION: u32 = 1;\n\
         pub const OWNED_ABI_ID: &str = \"owned.v1\";\n\
         pub const OWNED_HEADER_PATH: &str = \"include/owned.h\";\n\
         pub const OWNED_HEADER_SOURCE: &str = \"typedef struct OwnedUtf8 OwnedUtf8;\";\n\
         #[repr(C)]\n\
         pub struct OwnedUtf8 {\n\
             pub ptr: *const u8,\n\
             pub len: usize,\n\
         }\n",
    )
    .expect("write owned abi");

    assert_agent_snapshot(
        root,
        "RUST-AGENT-NATIVE-ABI-001",
        1,
        "rust_agent_native_abi_contract",
    );
}

fn assert_agent_snapshot(
    root: &Path,
    rule_id: &str,
    expected_count: usize,
    snapshot_name: &'static str,
) {
    let mut report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");
    report.findings.retain(|finding| finding.rule_id == rule_id);
    assert_eq!(
        report.findings.len(),
        expected_count,
        "expected {expected_count} {rule_id} finding(s), got {:?}",
        report.findings
    );
    let rendered = normalize_temp_root(&render_asp_rust(&report), root);
    insta::assert_snapshot!(snapshot_name, rendered);
}

fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}

fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

fn facade_reexports() -> String {
    let mut source = String::from("//! Test crate.\n");
    for index in 0..29 {
        source.push_str(&format!("pub use owner_{index}::Thing{index};\n"));
    }
    source
}

fn write_owner_cycle_fixture(root: &Path) {
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha dir");
    fs::create_dir_all(root.join("src/beta")).expect("create beta dir");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha.rs"),
        "//! Alpha owner.\nuse crate::beta::Beta;\nmod core;\npub struct Alpha;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta.rs"),
        "//! Beta owner.\nuse crate::alpha::Alpha;\nmod core;\npub struct Beta;\n",
    )
    .expect("write beta");
    fs::write(root.join("src/alpha/core.rs"), "//! Alpha core.\n").expect("write alpha core");
    fs::write(root.join("src/beta/core.rs"), "//! Beta core.\n").expect("write beta core");
}

fn write_cross_owner_leaf_fixture(root: &Path) {
    fs::create_dir_all(root.join("src/domain")).expect("create domain dir");
    fs::create_dir_all(root.join("src/ui")).expect("create ui dir");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod domain;\nmod ui;\n",
    )
    .expect("write lib");
    fs::write(root.join("src/domain.rs"), "//! Domain owner.\nmod leaf;\n").expect("write domain");
    fs::write(
        root.join("src/domain/leaf.rs"),
        "//! Domain leaf.\npub struct Leaf;\n",
    )
    .expect("write leaf");
    fs::write(
        root.join("src/ui.rs"),
        "//! Ui owner.\nuse crate::domain::leaf::Leaf;\nmod view;\n",
    )
    .expect("write ui");
    fs::write(root.join("src/ui/view.rs"), "//! Ui view.\n").expect("write view");
}

fn write_owner_fan_out_fixture(root: &Path) {
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha dir");
    fs::create_dir_all(root.join("src/beta")).expect("create beta dir");
    fs::create_dir_all(root.join("src/gamma")).expect("create gamma dir");
    fs::create_dir_all(root.join("src/orchestrator")).expect("create orchestrator dir");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\nmod gamma;\nmod orchestrator;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha.rs"),
        "//! Alpha owner.\nmod core;\npub struct Alpha;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta.rs"),
        "//! Beta owner.\nmod core;\npub struct Beta;\n",
    )
    .expect("write beta");
    fs::write(
        root.join("src/gamma.rs"),
        "//! Gamma owner.\nmod core;\npub struct Gamma;\n",
    )
    .expect("write gamma");
    fs::write(
        root.join("src/orchestrator.rs"),
        "use crate::alpha::Alpha;\nuse crate::beta::Beta;\nuse crate::gamma::Gamma;\nmod task;\n",
    )
    .expect("write orchestrator");
    fs::write(root.join("src/alpha/core.rs"), "//! Alpha core.\n").expect("write alpha core");
    fs::write(root.join("src/beta/core.rs"), "//! Beta core.\n").expect("write beta core");
    fs::write(root.join("src/gamma/core.rs"), "//! Gamma core.\n").expect("write gamma core");
    fs::write(root.join("src/orchestrator/task.rs"), "//! Task.\n").expect("write task");
}
