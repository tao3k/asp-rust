use std::fs;
use std::path::Path;

use asp_rust::{
    default_asp_rust_config, render_asp_rust_agent_snapshot,
    render_asp_rust_agent_snapshot_with_config,
};
use tempfile::TempDir;

#[test]
fn agent_reasoning_tree_snapshot_groups_owner_branches() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-reasoning-snapshot");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::create_dir_all(root.join("src/alt")).expect("create alternate source tree");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nuse crate::domain::Thing;\nuse std::fmt;\nmod domain;\n#[path = \"alt/custom.rs\"]\nmod custom;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\nuse self::leaf::Leaf;\nuse super::sibling::Thing;\nmod leaf;\npub use leaf::Thing;\n",
    )
    .expect("write domain");
    fs::write(
        root.join("src/domain/leaf.rs"),
        "//! Domain leaf.\n/// Domain leaf.\npub struct Leaf;\n/// Domain thing.\npub struct Thing;\n",
    )
    .expect("write leaf");
    fs::write(root.join("src/alt/custom.rs"), "//! Custom path child.\n").expect("write custom");

    let rendered = render_asp_rust_agent_snapshot(root).expect("render snapshot");

    insta::assert_snapshot!(
        "agent_reasoning_tree_snapshot",
        normalize_temp_root(&rendered, root)
    );
}

#[test]
fn agent_reasoning_tree_snapshot_compacts_workspace_packages() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("write workspace manifest");
    let active = root.join("crates/active");
    let empty = root.join("crates/empty");
    fs::create_dir_all(active.join("src/domain")).expect("create active domain");
    fs::create_dir_all(&empty).expect("create empty crate");
    write_manifest(&active, "active");
    write_manifest(&empty, "empty");
    fs::write(
        active.join("src/lib.rs"),
        "//! Active crate.\nuse crate::domain::Thing;\nmod domain;\n",
    )
    .expect("write active lib");
    fs::write(
        active.join("src/domain/mod.rs"),
        "//! Domain owner.\nmod thing;\npub use thing::Thing;\n",
    )
    .expect("write active domain");
    fs::write(
        active.join("src/domain/thing.rs"),
        "//! Domain thing.\n/// Domain thing.\npub struct Thing;\n",
    )
    .expect("write active thing");

    let rendered = render_asp_rust_agent_snapshot(root).expect("render snapshot");
    let rendered = normalize_temp_root(&rendered, root);

    assert!(!rendered.contains("crates/empty"), "{rendered}");
    insta::assert_snapshot!("agent_reasoning_tree_workspace_snapshot", rendered);
}

#[test]
fn agent_reasoning_tree_snapshot_ignores_test_context_owner_dependencies() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-snapshot-test-context");
    fs::create_dir_all(root.join("src/alpha")).expect("create alpha");
    fs::create_dir_all(root.join("src/beta")).expect("create beta");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod alpha;\nmod beta;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/alpha/mod.rs"),
        "//! Alpha owner.\nmod core;\npub use core::Alpha;\n",
    )
    .expect("write alpha");
    fs::write(
        root.join("src/beta/mod.rs"),
        "//! Beta owner.\nmod core;\npub use core::Beta;\n",
    )
    .expect("write beta");
    fs::write(
        root.join("src/alpha/core.rs"),
        "//! Alpha core.\n#[cfg(test)]\nmod tests {\n    use crate::beta::Beta;\n}\n/// Alpha handle.\npub struct Alpha;\n",
    )
    .expect("write alpha core");
    fs::write(
        root.join("src/beta/core.rs"),
        "//! Beta core.\n#[cfg(test)]\nmod tests {\n    use crate::alpha::Alpha;\n}\n/// Beta handle.\npub struct Beta;\n",
    )
    .expect("write beta core");
    let config = default_asp_rust_config().with_disabled_rule("RUST-AGENT-PROJECT-003");

    let rendered =
        render_asp_rust_agent_snapshot_with_config(root, &config).expect("render snapshot");
    let rendered = normalize_temp_root(&rendered, root);

    assert!(!rendered.contains("deps="), "{rendered}");
    assert!(!rendered.contains("OwnerDependencies:"), "{rendered}");
    insta::assert_snapshot!("agent_reasoning_tree_ignores_test_context_deps", rendered);
}

#[test]
fn agent_reasoning_tree_snapshot_omits_empty_child_edge_placeholder() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-snapshot-binary");
    fs::create_dir_all(root.join("src/bin")).expect("create bin");
    fs::write(root.join("src/bin/tool.rs"), "fn main() {}\n").expect("write binary");

    let rendered = render_asp_rust_agent_snapshot(root).expect("render snapshot");
    let rendered = normalize_temp_root(&rendered, root);

    assert!(!rendered.contains("-> -"), "{rendered}");
    assert!(
        rendered.contains("src/bin/tool.rs [root, binary] owner=src/bin/tool"),
        "{rendered}"
    );
    insta::assert_snapshot!("agent_reasoning_tree_omits_empty_child_edges", rendered);
}

#[test]
fn agent_reasoning_tree_snapshot_caps_large_sections() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "agent-snapshot-large");
    fs::create_dir_all(root.join("src")).expect("create src");
    let mut lib = String::from("//! Test crate.\nmod target;\n");
    for index in 0..30 {
        lib.push_str(&format!("mod owner_{index};\n"));
        fs::write(
            root.join(format!("src/owner_{index}.rs")),
            "//! Owner leaf.\nuse crate::target::Target;\nfn use_target(_: Target) {}\n",
        )
        .expect("write owner");
    }
    fs::write(root.join("src/lib.rs"), lib).expect("write lib");
    fs::write(
        root.join("src/target.rs"),
        "//! Target leaf.\nstruct Target;\n",
    )
    .expect("write target");

    let rendered = render_asp_rust_agent_snapshot(root).expect("render snapshot");
    let rendered = normalize_temp_root(&rendered, root);

    assert!(rendered.contains("... +23 children"), "{rendered}");
    assert!(rendered.contains("src/target.rs <--crate--"), "{rendered}");
    assert!(!rendered.contains("owner deps"), "{rendered}");
    insta::assert_snapshot!("agent_reasoning_tree_large_sections_are_capped", rendered);
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
