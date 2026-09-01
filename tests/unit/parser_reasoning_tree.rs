use std::collections::BTreeSet;
use std::fs;

use tempfile::TempDir;

use crate::AspRustScope;
use crate::parser::{
    RustModuleChildEdgeKind, RustReasoningOwnerBranchRole, RustUseImportRootKind,
    parse_cargo_cfg_facts, parse_cargo_dependency_facts, parse_cargo_manifest, parse_rust_file,
    rust_reasoning_tree_facts,
};

type DependencyEdge = (
    std::path::PathBuf,
    Vec<String>,
    std::path::PathBuf,
    Vec<String>,
    RustUseImportRootKind,
    usize,
    bool,
);

type DeepRelativeImportRepair = (String, Option<String>, usize, usize, bool);

fn relative_paths(base: &std::path::Path, paths: &[std::path::PathBuf]) -> BTreeSet<String> {
    paths.iter().map(|path| relative_path(base, path)).collect()
}

fn relative_path(base: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn cargo_manifest_parser_records_dependency_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"cargo-facts\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\naxum = \"0.1\"\narrow-flight = \"0.1\"\nflight = { package = \"arrow-flight\", version = \">=0.1, <0.3\", optional = true, features = [\"tls\", \"flight-sql\", \"tls\"] }\n\n[dev-dependencies]\ntokio = { version = \"1\", features = [\"rt\"] }\n\n[build-dependencies]\ncc = \"1\"\n\n[target.'cfg(windows)'.dependencies]\nwinapi = \"0.3\"\n",
    )
    .expect("write manifest");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let dependency_facts = parse_cargo_dependency_facts(root)
        .iter()
        .map(|dependency| {
            format!(
                "{}|{}|{}|{}|{:?}|{}|{}|{}",
                dependency.dependency_key,
                dependency.import_name,
                dependency.package_name,
                dependency.version_req.as_deref().unwrap_or("-"),
                dependency.kind,
                dependency.target.as_deref().unwrap_or("-"),
                dependency.optional,
                dependency.features.join("+")
            )
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(
        dependency_facts,
        [
            "arrow-flight|arrow_flight|arrow-flight|^0.1|Normal|-|false|",
            "axum|axum|axum|^0.1|Normal|-|false|",
            "cc|cc|cc|^1|Build|-|false|",
            "flight|flight|arrow-flight|>=0.1, <0.3|Normal|-|true|flight-sql+tls",
            "tokio|tokio|tokio|^1|Dev|-|false|rt",
            "winapi|winapi|winapi|^0.3|Normal|cfg(windows)|false|",
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
    );
}

#[test]
fn cargo_manifest_parser_records_cfg_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cargo-cfg-facts\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [features]\n\
         json = []\n\n\
         [lints.rust]\n\
         unexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(loom)'] }\n\n\
         [workspace.lints.rust]\n\
         unexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(tokio_unstable)'] }\n\n\
         [target.'cfg(all(tokio_unstable, target_os = \"linux\"))'.dependencies]\n\
         mio = \"1\"\n",
    )
    .expect("write manifest");

    let cfg_facts = parse_cargo_cfg_facts(root)
        .iter()
        .map(|cfg| format!("{}|{}|{}", cfg.cfg, cfg.declared_in, cfg.expression))
        .collect::<BTreeSet<_>>();

    assert_eq!(
        cfg_facts,
        [
            "feature:json|features|cfg(feature=\"json\")",
            "loom|lints.rust.unexpected_cfgs|cfg(loom)",
            "target_os|target.dependencies|cfg(all(tokio_unstable,target_os=\"linux\"))",
            "tokio_unstable|target.dependencies|cfg(all(tokio_unstable,target_os=\"linux\"))",
            "tokio_unstable|workspace.lints.rust.unexpected_cfgs|cfg(tokio_unstable)",
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
    );
}

#[test]
fn cargo_manifest_parser_records_workspace_inherited_dependency_facts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/api\"]\n\n[workspace.dependencies]\nflight = { package = \"arrow-flight\", version = \">=0.1, <0.3\", features = [\"flight-sql\"] }\n",
    )
    .expect("write workspace manifest");
    let package_root = root.join("crates/api");
    fs::create_dir_all(package_root.join("src")).expect("create package source");
    fs::write(
        package_root.join("Cargo.toml"),
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nflight = { workspace = true }\n",
    )
    .expect("write package manifest");
    fs::write(package_root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");

    let dependency_facts = parse_cargo_dependency_facts(&package_root)
        .iter()
        .map(|dependency| {
            format!(
                "{}|{}|{}|{}|{:?}|{}|{}|{}",
                dependency.dependency_key,
                dependency.import_name,
                dependency.package_name,
                dependency.version_req.as_deref().unwrap_or("-"),
                dependency.kind,
                dependency.target.as_deref().unwrap_or("-"),
                dependency.optional,
                dependency.features.join("+")
            )
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(
        dependency_facts,
        ["flight|flight|arrow-flight|>=0.1, <0.3|Normal|-|false|flight-sql"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn cargo_manifest_parser_uses_completed_manifest_targets_and_workspace_paths() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\", \"crates/helper\"]\n\n[workspace.dependencies]\nhelper = { path = \"crates/helper\", version = \"0.1\" }\n",
    )
    .expect("write workspace manifest");

    let helper_root = root.join("crates/helper");
    fs::create_dir_all(helper_root.join("src")).expect("create helper source");
    fs::write(
        helper_root.join("Cargo.toml"),
        "[package]\nname = \"helper\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write helper manifest");
    fs::write(helper_root.join("src/lib.rs"), "//! Helper crate.\n").expect("write helper lib");

    let app_root = root.join("crates/app");
    fs::create_dir_all(app_root.join("src")).expect("create app source");
    fs::create_dir_all(app_root.join("examples")).expect("create examples");
    fs::create_dir_all(app_root.join("tests")).expect("create tests");
    fs::create_dir_all(app_root.join("benches")).expect("create benches");
    fs::write(
        app_root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nhelper = { workspace = true }\n",
    )
    .expect("write app manifest");
    fs::write(app_root.join("src/lib.rs"), "//! App library.\n").expect("write app lib");
    fs::write(app_root.join("src/main.rs"), "fn main() {}\n").expect("write app bin");
    fs::write(app_root.join("examples/demo.rs"), "fn main() {}\n").expect("write example");
    fs::write(
        app_root.join("tests/integration.rs"),
        "#[test]\nfn integration() {}\n",
    )
    .expect("write integration test");
    fs::write(app_root.join("benches/load.rs"), "fn main() {}\n").expect("write bench");

    let manifest = parse_cargo_manifest(&app_root);

    assert_eq!(
        relative_paths(&app_root, &manifest.source_target_files),
        ["src/lib.rs", "src/main.rs"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        manifest
            .example_targets
            .iter()
            .map(|target| format!("{}|{}", target.name, relative_path(&app_root, &target.path)))
            .collect::<BTreeSet<_>>(),
        ["demo|examples/demo.rs"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        relative_paths(&app_root, &manifest.test_target_files),
        ["tests/integration.rs"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        manifest
            .bench_targets
            .iter()
            .map(|target| format!(
                "{}|{}|{}",
                target.name,
                relative_path(&app_root, &target.path),
                target.harness
            ))
            .collect::<BTreeSet<_>>(),
        ["load|benches/load.rs|true"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(manifest.path_dependency_roots, vec![helper_root]);
}

#[test]
fn reasoning_tree_interprets_modules_owners_and_child_edges() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let src = root.join("src");
    fs::create_dir_all(src.join("domain")).expect("create source tree");
    fs::create_dir_all(src.join("alt")).expect("create alternate source tree");
    fs::write(
        src.join("lib.rs"),
    "//! Crate facade.\npub use crate::domain::Thing;\nuse std::fmt;\npub fn visible_api() {}\n#[cfg(test)]\nmod tests {\n    use proptest::prelude::*;\n}\nmod domain;\n#[path = \"alt/custom.rs\"]\nmod custom;\ninclude!(\"shard.rs\");\n",
    )
    .expect("write lib");
    fs::write(
        src.join("domain.rs"),
        "//! Domain branch.\nuse self::leaf::Leaf;\nuse super::sibling::Thing;\nmod leaf;\n",
    )
    .expect("write domain");
    fs::write(src.join("domain/leaf.rs"), "//! Domain leaf.\n").expect("write leaf");
    fs::write(src.join("alt/custom.rs"), "//! Custom path child.\n").expect("write custom");
    fs::write(src.join("shard.rs"), "//! Included shard.\n").expect("write shard");

    let modules = [
        parse_rust_file(&src.join("lib.rs")),
        parse_rust_file(&src.join("domain.rs")),
        parse_rust_file(&src.join("domain/leaf.rs")),
        parse_rust_file(&src.join("alt/custom.rs")),
        parse_rust_file(&src.join("shard.rs")),
    ];
    let scope = AspRustScope {
        project_root: root.to_path_buf(),
        source_paths: vec![src.clone()],
        test_paths: Vec::new(),
        package_paths: vec![root.join("build.rs")],
        fallback_paths: vec![root.to_path_buf()],
    };

    let reasoning_tree = rust_reasoning_tree_facts(&scope, &modules);
    assert_eq!(reasoning_tree.package_root, root);
    assert_eq!(reasoning_tree.source_roots, vec![src.clone()]);
    assert_eq!(
        reasoning_tree.package_entrypoints,
        vec![root.join("build.rs")]
    );

    let lib = reasoning_tree
        .module(&src.join("lib.rs"))
        .expect("lib facts");
    assert_eq!(lib.public_api_summary.public_items, 1);
    assert_eq!(lib.public_api_summary.public_exports, 1);
    assert_eq!(lib.public_api_summary.public_functions, 1);
    assert_eq!(lib.import_summary.test_context_imports, 1);
    assert_eq!(
        lib.import_summary.production_external_imports,
        vec![vec!["std".to_string(), "fmt".to_string()]],
    );
    assert!(lib.is_source_module);
    assert!(lib.is_module_tree_root);
    assert!(lib.source_path.is_crate_facade);
    assert_eq!(lib.import_summary.crate_imports, 1);
    assert_eq!(lib.import_summary.external_imports, 2);
    assert_eq!(
        lib.import_summary.local_owner_imports,
        vec![vec!["src".to_string(), "domain".to_string()]]
    );
    assert_eq!(
        dependency_edges(lib),
        vec![(
            src.join("lib.rs"),
            vec!["src".to_string()],
            src.join("domain.rs"),
            vec!["src".to_string(), "domain".to_string()],
            RustUseImportRootKind::Crate,
            2,
            false,
        )]
    );
    assert_eq!(
        child_edges(lib),
        vec![
            (RustModuleChildEdgeKind::Mod, src.join("domain.rs")),
            (
                RustModuleChildEdgeKind::PathAttrMod,
                src.join("alt/custom.rs")
            ),
            (
                RustModuleChildEdgeKind::IncludeLiteral,
                src.join("shard.rs")
            ),
        ]
    );

    let branch = reasoning_tree
        .module(&src.join("domain.rs"))
        .expect("branch facts");
    assert_eq!(
        branch.source_path.namespace_components,
        vec!["src".to_string(), "domain".to_string()]
    );
    assert_eq!(branch.import_summary.self_imports, 1);
    assert_eq!(branch.import_summary.parent_imports, 1);
    assert_eq!(
        branch.import_summary.local_owner_imports,
        vec![vec![
            "src".to_string(),
            "domain".to_string(),
            "leaf".to_string()
        ]]
    );
    assert_eq!(
        dependency_edges(branch),
        vec![(
            src.join("domain.rs"),
            vec!["src".to_string(), "domain".to_string()],
            src.join("domain/leaf.rs"),
            vec!["src".to_string(), "domain".to_string(), "leaf".to_string()],
            RustUseImportRootKind::SelfScope,
            2,
            false,
        )]
    );
    assert_eq!(
        tree_dependency_edges(&reasoning_tree),
        vec![
            (
                src.join("lib.rs"),
                vec!["src".to_string()],
                src.join("domain.rs"),
                vec!["src".to_string(), "domain".to_string()],
                RustUseImportRootKind::Crate,
                2,
                false,
            ),
            (
                src.join("domain.rs"),
                vec!["src".to_string(), "domain".to_string()],
                src.join("domain/leaf.rs"),
                vec!["src".to_string(), "domain".to_string(), "leaf".to_string()],
                RustUseImportRootKind::SelfScope,
                2,
                false,
            ),
        ]
    );
    assert_eq!(
        child_edges(branch),
        vec![(RustModuleChildEdgeKind::Mod, src.join("domain/leaf.rs"))]
    );

    assert_eq!(reasoning_tree.owner_branches.len(), 2);
    assert_eq!(reasoning_tree.owner_branches[0].path, src.join("lib.rs"));
    assert_eq!(
        reasoning_tree.owner_branches[0].roles,
        vec![
            RustReasoningOwnerBranchRole::Root,
            RustReasoningOwnerBranchRole::Facade,
        ]
    );
    assert_eq!(
        reasoning_tree.owner_branches[0].owner_namespace,
        vec!["src".to_string()]
    );
    assert_eq!(
        owner_branch_child_edges(&reasoning_tree.owner_branches[0]),
        vec![
            (RustModuleChildEdgeKind::Mod, src.join("domain.rs")),
            (
                RustModuleChildEdgeKind::PathAttrMod,
                src.join("alt/custom.rs")
            ),
            (
                RustModuleChildEdgeKind::IncludeLiteral,
                src.join("shard.rs")
            ),
        ]
    );
    assert_eq!(
        reasoning_tree.owner_branches[0]
            .import_summary
            .crate_imports,
        1
    );
    assert_eq!(
        reasoning_tree.owner_branches[0]
            .import_summary
            .external_imports,
        2
    );
    assert_eq!(
        reasoning_tree.owner_branches[0]
            .import_summary
            .local_owner_imports,
        vec![vec!["src".to_string(), "domain".to_string()]]
    );
    assert_eq!(reasoning_tree.owner_branches[1].path, src.join("domain.rs"));
    assert_eq!(
        reasoning_tree.owner_branches[1].roles,
        vec![RustReasoningOwnerBranchRole::Branch]
    );
    assert_eq!(
        reasoning_tree.owner_branches[1].owner_namespace,
        vec!["src".to_string(), "domain".to_string()]
    );
    assert_eq!(
        reasoning_tree.owner_branches[1].import_summary.self_imports,
        1
    );
    assert_eq!(
        reasoning_tree.owner_branches[1]
            .import_summary
            .parent_imports,
        1
    );
    assert_eq!(
        reasoning_tree.owner_branches[1]
            .import_summary
            .local_owner_imports,
        vec![vec![
            "src".to_string(),
            "domain".to_string(),
            "leaf".to_string()
        ]]
    );

    assert!(reasoning_tree.shadowed_module_sources.is_empty());
    assert!(reasoning_tree.unreachable_source_files.is_empty());
}

#[test]
fn reasoning_tree_deduplicates_owner_dependencies_by_context_and_keeps_earliest_line() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let src = root.join("src");
    fs::create_dir_all(&src).expect("create source tree");
    fs::write(
        src.join("lib.rs"),
        "//! Crate facade.\nuse crate::domain::Thing;\nuse crate::domain::Other;\nmod domain;\n#[cfg(test)]\nmod tests {\n    use crate::domain::Thing;\n}\n",
    )
    .expect("write lib");
    fs::write(src.join("domain.rs"), "//! Domain branch.\n").expect("write domain");

    let modules = [
        parse_rust_file(&src.join("lib.rs")),
        parse_rust_file(&src.join("domain.rs")),
    ];
    let scope = AspRustScope {
        project_root: root.to_path_buf(),
        source_paths: vec![src.clone()],
        test_paths: Vec::new(),
        package_paths: Vec::new(),
        fallback_paths: vec![root.to_path_buf()],
    };

    let reasoning_tree = rust_reasoning_tree_facts(&scope, &modules);
    let lib = reasoning_tree
        .module(&src.join("lib.rs"))
        .expect("lib facts");

    assert_eq!(
        dependency_edges(lib),
        vec![
            (
                src.join("lib.rs"),
                vec!["src".to_string()],
                src.join("domain.rs"),
                vec!["src".to_string(), "domain".to_string()],
                RustUseImportRootKind::Crate,
                2,
                false,
            ),
            (
                src.join("lib.rs"),
                vec!["src".to_string()],
                src.join("domain.rs"),
                vec!["src".to_string(), "domain".to_string()],
                RustUseImportRootKind::Crate,
                7,
                true,
            ),
        ]
    );
    assert_eq!(
        dependency_edges(lib),
        tree_dependency_edges(&reasoning_tree)
    );
}

#[test]
fn reasoning_tree_derives_crate_repairs_for_deep_relative_imports() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let src = root.join("src");
    fs::create_dir_all(src.join("gateway/studio")).expect("create source tree");
    fs::write(src.join("lib.rs"), "//! Crate facade.\nmod gateway;\n").expect("write lib");
    fs::write(src.join("gateway.rs"), "//! Gateway branch.\n").expect("write gateway");
    fs::write(
        src.join("gateway/studio/support.rs"),
        "use super::super::{MarkdownLintIssue, MarkdownLintReport};\n\
         #[cfg(test)]\n\
         mod tests {\n\
             use super::super::super::TestOnly;\n\
         }\n",
    )
    .expect("write support");

    let modules = [
        parse_rust_file(&src.join("lib.rs")),
        parse_rust_file(&src.join("gateway.rs")),
        parse_rust_file(&src.join("gateway/studio/support.rs")),
    ];
    let scope = AspRustScope {
        project_root: root.to_path_buf(),
        source_paths: vec![src.clone()],
        test_paths: Vec::new(),
        package_paths: Vec::new(),
        fallback_paths: vec![root.to_path_buf()],
    };

    let reasoning_tree = rust_reasoning_tree_facts(&scope, &modules);
    let support = reasoning_tree
        .module(&src.join("gateway/studio/support.rs"))
        .expect("support facts");

    assert_eq!(
        deep_relative_import_repairs(support),
        vec![
            (
                "super::super::MarkdownLintIssue".to_string(),
                Some("crate::gateway::MarkdownLintIssue".to_string()),
                2,
                1,
                false,
            ),
            (
                "super::super::MarkdownLintReport".to_string(),
                Some("crate::gateway::MarkdownLintReport".to_string()),
                2,
                1,
                false,
            ),
            (
                "super::super::super::TestOnly".to_string(),
                Some("crate::gateway::TestOnly".to_string()),
                3,
                4,
                true,
            ),
        ]
    );
}

#[test]
fn reasoning_tree_marks_test_root_modules_as_test_sources() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let src = root.join("src");
    let tests = root.join("tests");
    fs::create_dir_all(&src).expect("create source tree");
    fs::create_dir_all(&tests).expect("create tests tree");
    fs::write(src.join("lib.rs"), "//! Crate facade.\n").expect("write lib");
    fs::write(tests.join("integration.rs"), "use crate::prelude::*;\n").expect("write integration");

    let modules = [
        parse_rust_file(&src.join("lib.rs")),
        parse_rust_file(&tests.join("integration.rs")),
    ];
    let scope = AspRustScope {
        project_root: root.to_path_buf(),
        source_paths: vec![src],
        test_paths: vec![tests.clone()],
        package_paths: Vec::new(),
        fallback_paths: vec![root.to_path_buf()],
    };

    let reasoning_tree = rust_reasoning_tree_facts(&scope, &modules);
    let test_module = reasoning_tree
        .module(&tests.join("integration.rs"))
        .expect("test module facts");

    assert!(test_module.source_path.is_test_source);
    assert!(!test_module.is_source_module);
    assert!(
        !reasoning_tree
            .owner_branches
            .iter()
            .any(|branch| branch.path.ends_with("tests/integration.rs"))
    );
}

fn deep_relative_import_repairs(
    module: &crate::parser::RustReasoningModuleFacts,
) -> Vec<DeepRelativeImportRepair> {
    module
        .import_summary
        .deep_relative_import_facts
        .iter()
        .map(|deep_relative_import| {
            (
                deep_relative_import.rendered_path(),
                deep_relative_import.rendered_crate_path(),
                deep_relative_import.parent_hops,
                deep_relative_import.line,
                deep_relative_import.is_test_context,
            )
        })
        .collect()
}

fn child_edges(
    module: &crate::parser::RustReasoningModuleFacts,
) -> Vec<(RustModuleChildEdgeKind, std::path::PathBuf)> {
    module
        .declared_child_edges
        .iter()
        .map(|edge| (edge.kind, edge.child_path.clone()))
        .collect()
}

fn owner_branch_child_edges(
    branch: &crate::parser::RustReasoningOwnerBranchFacts,
) -> Vec<(RustModuleChildEdgeKind, std::path::PathBuf)> {
    branch
        .declared_child_edges
        .iter()
        .map(|edge| (edge.kind, edge.child_path.clone()))
        .collect()
}

fn dependency_edges(module: &crate::parser::RustReasoningModuleFacts) -> Vec<DependencyEdge> {
    module
        .import_summary
        .local_owner_dependencies
        .iter()
        .map(|dependency| {
            (
                dependency.source_path.clone(),
                dependency.source_namespace.clone(),
                dependency.target_path.clone(),
                dependency.target_namespace.clone(),
                dependency.via_root,
                dependency.line,
                dependency.is_test_context,
            )
        })
        .collect()
}

fn tree_dependency_edges(tree: &crate::parser::RustReasoningTreeFacts) -> Vec<DependencyEdge> {
    tree.owner_dependencies
        .iter()
        .map(|dependency| {
            (
                dependency.source_path.clone(),
                dependency.source_namespace.clone(),
                dependency.target_path.clone(),
                dependency.target_namespace.clone(),
                dependency.via_root,
                dependency.line,
                dependency.is_test_context,
            )
        })
        .collect()
}
