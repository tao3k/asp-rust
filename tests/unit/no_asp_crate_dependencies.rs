//! Architecture gate for the independently distributable Rust language harness.

use std::path::Path;

const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "build-dependencies", "dev-dependencies"];
const SHARED_CONTRACT_CRATES: [&str; 2] = [
    "agent-semantic-content-identity",
    "agent-semantic-provider-transport",
];
const SHARED_CONTRACT_MODULES: [&str; 2] = [
    "agent_semantic_content_identity",
    "agent_semantic_provider_transport",
];

fn source_dependency_violations(root: &Path) -> Vec<String> {
    let mut pending = vec![root.to_path_buf()];
    let mut violations = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("read Rust harness source directory") {
            let path = entry.expect("read Rust harness source entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read Rust harness source");
            for (line_index, line) in source.lines().enumerate() {
                if line.contains("agent_semantic_")
                    && !SHARED_CONTRACT_MODULES
                        .iter()
                        .any(|module| line.contains(module))
                {
                    violations.push(format!(
                        "{}:{}:{}",
                        path.display(),
                        line_index + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    violations.sort();
    violations
}

#[test]
fn rust_harness_depends_only_on_shared_v1_contracts() {
    let harness_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for manifest_path in [
        harness_root.join("Cargo.toml"),
        harness_root.join("rs-harness/Cargo.toml"),
    ] {
        let manifest_text =
            std::fs::read_to_string(&manifest_path).expect("read Rust harness Cargo.toml");
        let manifest: toml::Value =
            toml::from_str(&manifest_text).expect("parse Rust harness Cargo.toml");
        violations.extend(
            manifest_dependency_violations(&manifest)
                .into_iter()
                .map(|violation| format!("{}:{violation}", manifest_path.display())),
        );
    }
    for source_root in [
        harness_root.join("src"),
        harness_root.join("rs-harness/src"),
    ] {
        violations.extend(source_dependency_violations(&source_root));
    }

    assert!(
        violations.is_empty(),
        "rust-lang-project-harness may depend only on shared v1 contracts: {}",
        violations.join(", ")
    );
}

fn manifest_dependency_violations(manifest: &toml::Value) -> Vec<String> {
    let mut violations = Vec::new();
    collect_dependency_violations(manifest, "", &mut violations);
    violations
}

fn collect_dependency_violations(
    value: &toml::Value,
    table_path: &str,
    violations: &mut Vec<String>,
) {
    let Some(table) = value.as_table() else {
        return;
    };
    for (key, nested) in table {
        let nested_path = if table_path.is_empty() {
            key.clone()
        } else {
            format!("{table_path}.{key}")
        };
        if DEPENDENCY_TABLES.contains(&key.as_str()) {
            violations.extend(dependency_violations(nested, &nested_path));
        } else {
            collect_dependency_violations(nested, &nested_path, violations);
        }
    }
}

fn dependency_violations(dependency_table: &toml::Value, table_path: &str) -> Vec<String> {
    dependency_table
        .as_table()
        .into_iter()
        .flat_map(|dependencies| dependencies.iter())
        .filter_map(|(dependency_name, dependency)| {
            let package_name = dependency
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(dependency_name);
            let path = dependency
                .get("path")
                .and_then(toml::Value::as_str)
                .unwrap_or_default();
            let is_asp_package = package_name.starts_with("agent-semantic-")
                && !SHARED_CONTRACT_CRATES.contains(&package_name);
            let is_asp_path = path.split(['/', '\\']).any(|component| {
                component.starts_with("agent-semantic-")
                    && !SHARED_CONTRACT_CRATES.contains(&component)
            });
            (is_asp_package || is_asp_path)
                .then(|| format!("{table_path}.{dependency_name} -> {package_name} ({path})"))
        })
        .collect()
}
