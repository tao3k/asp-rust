//! Architecture gate for the independently distributable ASP Rust provider.

use std::path::Path;

const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "build-dependencies", "dev-dependencies"];

fn source_dependency_violations(root: &Path) -> Vec<String> {
    let mut pending = vec![root.to_path_buf()];
    let mut violations = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("read ASP Rust source directory") {
            let path = entry.expect("read ASP Rust source entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read ASP Rust source");
            for (line_index, line) in source.lines().enumerate() {
                if line.contains("agent_semantic_") {
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
fn asp_rust_has_no_parent_monorepo_dependencies() {
    let asp_rust_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for manifest_path in [
        asp_rust_root.join("Cargo.toml"),
        asp_rust_root.join("build-support/Cargo.toml"),
    ] {
        let manifest_text =
            std::fs::read_to_string(&manifest_path).expect("read ASP Rust Cargo.toml");
        let manifest: toml::Value =
            toml::from_str(&manifest_text).expect("parse ASP Rust Cargo.toml");
        violations.extend(
            manifest_dependency_violations(&manifest)
                .into_iter()
                .map(|violation| format!("{}:{violation}", manifest_path.display())),
        );
    }
    for source_root in [
        asp_rust_root.join("src"),
        asp_rust_root.join("build-support/src"),
    ] {
        violations.extend(source_dependency_violations(&source_root));
    }

    assert!(
        violations.is_empty(),
        "asp-rust must not depend on parent agent-semantic crates: {}",
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
            let is_asp_package = package_name.starts_with("agent-semantic-");
            let is_asp_path = path
                .split(['/', '\\'])
                .any(|component| component.starts_with("agent-semantic-"));
            (is_asp_package || is_asp_path)
                .then(|| format!("{table_path}.{dependency_name} -> {package_name} ({path})"))
        })
        .collect()
}
