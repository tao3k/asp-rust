use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn policy_rules_consume_parser_facts_not_syn_directly() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    for path in rust_files_under(&root.join("src/rules")) {
        let source = fs::read_to_string(&path).expect("read rule source");
        if source.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("use syn") || line.contains("syn::")
        }) {
            offenders.push(relative_path(&root, &path));
        }
    }

    assert!(
        offenders.is_empty(),
        "policy rule modules must consume src/parser facts, not syn directly: {offenders:?}"
    );
}

#[test]
fn native_use_scope_facts_stay_in_parser_layer() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_source =
        fs::read_to_string(root.join("src/parser/use_tree.rs")).expect("read use tree parser");
    assert!(parser_source.contains("syn::UseTree::Glob"));
    assert!(parser_source.contains("RustUseGlobScopeKind"));
    assert!(parser_source.contains("RustUseDeepRelativeImportSyntax"));
    assert!(parser_source.contains("fn is_deep_relative_import"));
    let reasoning_tree_source =
        fs::read_to_string(root.join("src/parser/reasoning_tree.rs")).expect("read parser facts");
    assert!(reasoning_tree_source.contains("RustReasoningDeepRelativeImportFacts"));
    assert!(reasoning_tree_source.contains("fn record_deep_relative_import_facts"));
    assert!(reasoning_tree_source.contains("fn crate_relative_deep_relative_import_segments"));
    assert!(reasoning_tree_source.contains("fn parent_import_candidate_namespace"));

    let rule_source = fs::read_to_string(root.join("src/rules/modularity/source_shape.rs"))
        .expect("read modularity source-shape rules");
    assert!(rule_source.contains("RustUseGlobScopeKind::CrateOwner"));
    assert!(rule_source.contains("deep_relative_import_facts"));
    assert!(rule_source.contains("rendered_crate_path"));

    for forbidden in [
        "UseTree::Glob",
        "has_super_super",
        "contains_deep_relative_import",
        "crate_relative_import_segments",
    ] {
        assert!(
            !rule_source.contains(forbidden),
            "modularity rules must consume parser-owned use scope facts, not `{forbidden}`"
        );
    }
}

#[test]
fn syn_parse_file_entrypoint_stays_inside_parser_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_entrypoints = rust_files_under(&root.join("src"))
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .expect("read Rust source")
                .contains("syn::parse_file")
        })
        .map(|path| relative_path(&root, &path))
        .collect::<Vec<_>>();

    assert_eq!(parser_entrypoints, vec!["src/parser/parsed_module.rs"]);
}

#[test]
fn parsed_module_does_not_expose_syn_file_to_policy_rules() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parsed_module =
        fs::read_to_string(root.join("src/parser/parsed_module.rs")).expect("read parsed module");

    assert!(!parsed_module.contains("pub syntax: Option<syn::File>"));
}

#[test]
fn cargo_manifest_parser_lives_under_parser_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_source = fs::read_to_string(root.join("src/parser/cargo_manifest.rs"))
        .expect("read cargo manifest parser");
    let dependency_facts_source =
        fs::read_to_string(root.join("src/parser/cargo_dependency_facts.rs"))
            .expect("read cargo dependency facts parser");
    let cfg_facts_source = fs::read_to_string(root.join("src/parser/cargo_cfg.rs"))
        .expect("read cargo cfg facts parser");
    assert!(parser_source.contains("use cargo_toml::"));
    assert!(parser_source.contains("Manifest::from_path"));
    assert!(dependency_facts_source.contains("parse_cargo_dependency_facts"));
    assert!(dependency_facts_source.contains("Manifest::from_path"));
    assert!(!parser_source.contains("Command::new"));
    assert!(!parser_source.contains("cargo metadata"));
    assert!(!dependency_facts_source.contains("Command::new"));
    assert!(!dependency_facts_source.contains("cargo metadata"));
    assert!(!cfg_facts_source.contains("Command::new"));
    assert!(!cfg_facts_source.contains("cargo metadata"));
    assert!(!cfg_facts_source.contains("Manifest::from_path"));

    let mut offenders = Vec::new();
    for path in rust_files_under(&root.join("src")) {
        let relative = relative_path(&root, &path);
        if matches!(
            relative.as_str(),
            "src/parser/cargo_manifest.rs"
                | "src/parser/cargo_dependency_facts.rs"
                | "src/parser/cargo_cfg.rs"
        ) {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read Rust source");
        if source.contains("cargo_toml::") || source.contains("Manifest::from_path") {
            offenders.push(relative);
        }
    }

    assert!(
        offenders.is_empty(),
        "Cargo manifest parsing must stay under src/parser cargo owners: {offenders:?}"
    );
}

#[test]
fn verification_profile_index_does_not_hardcode_project_semantics() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile_index_sources = rust_files_under(&root.join("src/verification/profile_index"));
    let forbidden = [
        "path_signals",
        "path_segment_has_any",
        "namespace_has_any",
        "duckdb",
        "redis",
        "lance",
        "axum",
        "tokio",
        "rayon",
        "sha2",
        "arrow-flight",
        "arrow_flight",
    ];
    let mut offenders = Vec::new();
    for path in profile_index_sources {
        let relative = relative_path(&root, &path);
        let source = fs::read_to_string(&path).expect("read profile index source");
        for term in forbidden {
            if source.contains(term) {
                offenders.push(format!("{relative}:{term}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "verification profile index must infer from parser facts and config signals, not hardcoded project semantics: {offenders:?}"
    );
}

#[test]
fn module_tree_reachability_lives_under_parser_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_source = fs::read_to_string(root.join("src/parser/module_tree.rs"))
        .expect("read module tree parser");
    assert!(parser_source.contains("pub(crate) fn rust_module_tree_facts"));

    let forbidden_rule_fragments = [
        "external_child_module_edges",
        "child_module_base_dir",
        "is_module_tree_root",
        "include_target",
    ];
    let mut offenders = Vec::new();
    for path in rust_files_under(&root.join("src/rules")) {
        let source = fs::read_to_string(&path).expect("read rule source");
        if forbidden_rule_fragments
            .iter()
            .any(|fragment| source.contains(fragment))
        {
            offenders.push(relative_path(&root, &path));
        }
    }

    assert!(
        offenders.is_empty(),
        "module-tree reachability must be parser facts, not rule-local path reconstruction: {offenders:?}"
    );
}

#[test]
fn cargo_test_target_parsing_lives_under_parser_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_source = fs::read_to_string(root.join("src/parser/cargo_test_targets.rs"))
        .expect("read cargo test target parser");
    assert!(parser_source.contains("pub(crate) fn parse_cargo_test_targets"));
    assert!(parser_source.contains("parse_rust_file(&path)"));

    let rule_source = fs::read_to_string(root.join("src/rules/project_policy/test_targets.rs"))
        .expect("read test target rules");
    for forbidden in [
        "parse_rust_file(",
        "collect_test_target_files",
        "fs::read_dir",
    ] {
        assert!(
            !rule_source.contains(forbidden),
            "Cargo test target rules must consume parser-owned target modules, not `{forbidden}`"
        );
    }
}

#[test]
fn rust_path_attribute_resolution_lives_under_parser_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_source = fs::read_to_string(root.join("src/parser/native_syntax/module_facts.rs"))
        .expect("read native syntax parser");
    assert!(parser_source.contains("resolved_path_attr"));

    let path_resolution = fs::read_to_string(root.join("src/parser/path_resolution.rs"))
        .expect("read path resolution parser helper");
    assert!(path_resolution.contains("pub(crate) fn resolve_rust_path_attr"));

    let mut offenders = Vec::new();
    for path in rust_files_under(&root.join("src/rules")) {
        let source = fs::read_to_string(&path).expect("read rule source");
        if source.contains("resolve_path_attr")
            || source.contains("resolve_rust_path_attr")
            || source.contains("Component::ParentDir")
        {
            offenders.push(relative_path(&root, &path));
        }
    }

    assert!(
        offenders.is_empty(),
        "Rust #[path] resolution must be parser facts, not rule-local path normalization: {offenders:?}"
    );
}

#[test]
fn rust_source_path_facts_live_under_parser_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parser_source =
        fs::read_to_string(root.join("src/parser/source_path.rs")).expect("read source path facts");
    assert!(parser_source.contains("pub(crate) fn rust_source_path_facts"));
    assert!(parser_source.contains("repeated_namespace_segments"));
    assert!(parser_source.contains("is_binary_entrypoint"));

    let forbidden_rule_fragments = [
        "fn relative_namespace_components",
        "fn repeated_segments",
        "fn offending_branch",
        "fn is_binary_entrypoint_file",
        "fn is_build_script_entrypoint_file",
    ];
    let mut offenders = Vec::new();
    for path in rust_files_under(&root.join("src/rules")) {
        let source = fs::read_to_string(&path).expect("read rule source");
        if forbidden_rule_fragments
            .iter()
            .any(|fragment| source.contains(fragment))
        {
            offenders.push(relative_path(&root, &path));
        }
    }

    assert!(
        offenders.is_empty(),
        "Rust source path and namespace facts must live under src/parser/source_path.rs: {offenders:?}"
    );
}

fn rust_files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files);
    files.sort();
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read directory") {
        let path = entry.expect("read directory entry").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("path under repository root")
        .to_string_lossy()
        .replace('\\', "/")
}
