use std::fs;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn downstream_normal_graph_excludes_asp_rust_dev_dependency() {
    let temp = TempDir::new().expect("temp dir");
    let library = temp.path().join("policy-library");
    let build_support = temp.path().join("workspace-build-support");
    let consumer = temp.path().join("consumer");
    fs::create_dir_all(library.join("src")).expect("create library");
    fs::create_dir_all(library.join("tests")).expect("create library tests");
    fs::create_dir_all(build_support.join("src")).expect("create build support");
    fs::create_dir_all(consumer.join("src")).expect("create consumer");

    let asp_rust_root = env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\");
    fs::write(
        library.join("Cargo.toml"),
        "[package]\nname = \"policy-library\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dev-dependencies]\nworkspace-build-support = { path = \"../workspace-build-support\" }\n",
    )
    .expect("write library manifest");
    fs::write(library.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").expect("write library");
    fs::write(
        build_support.join("Cargo.toml"),
        format!(
            "[package]\nname = \"workspace-build-support\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nasp-rust = {{ path = \"{asp_rust_root}\" }}\n"
        ),
    )
    .expect("write build support manifest");
    fs::write(
        build_support.join("src/lib.rs"),
        r#"
pub fn workspace_policy() -> asp_rust::AspRustWorkspacePolicy {
    asp_rust::AspRustWorkspacePolicy::new("fixture", asp_rust::default_asp_rust_config())
}

#[doc(hidden)]
pub use asp_rust;

#[macro_export]
macro_rules! asp_workspace_policy_gate {
    () => {
        $crate::asp_rust::asp_rust_workspace_dev_gate!(
            mode = deny,
            policy = $crate::workspace_policy()
        );
    };
}
"#,
    )
    .expect("write build support");
    fs::write(
        library.join("tests/asp_rust_policy.rs"),
        "workspace_build_support::asp_workspace_policy_gate!();\n",
    )
    .expect("write workspace policy test target");
    fs::write(
        consumer.join("Cargo.toml"),
        "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\npolicy-library = { path = \"../policy-library\" }\n",
    )
    .expect("write consumer manifest");
    fs::write(
        consumer.join("src/lib.rs"),
        "pub use policy_library::value;\n",
    )
    .expect("write consumer");

    let normal_tree = cargo_tree(&consumer, "normal,build");
    assert!(normal_tree.contains("policy-library"), "{normal_tree}");
    assert!(!normal_tree.contains("asp-rust"), "{normal_tree}");

    let dev_tree = cargo_tree(&library, "normal,dev");
    assert!(dev_tree.contains("workspace-build-support"), "{dev_tree}");
    assert!(dev_tree.contains("asp-rust"), "{dev_tree}");

    let compile_status = Command::new(env!("CARGO"))
        .args(["test", "--offline", "--no-run", "--manifest-path"])
        .arg(library.join("Cargo.toml"))
        .status()
        .expect("compile Build Support Dev Gate");
    assert!(
        compile_status.success(),
        "Build Support Dev Gate must compile"
    );
}

fn cargo_tree(root: &std::path::Path, edges: &str) -> String {
    let output = Command::new(env!("CARGO"))
        .args(["tree", "--offline", "--edges", edges, "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("run cargo tree");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo tree utf8")
}
