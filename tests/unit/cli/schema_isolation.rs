use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

fn visit_schema_refs(
    schema_root: &Path,
    document_path: &Path,
    value: &serde_json::Value,
    visited: &mut BTreeSet<PathBuf>,
) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(serde_json::Value::as_str)
                && !reference.starts_with('#')
            {
                let path = reference.split('#').next().unwrap_or(reference);
                let relative = Path::new(path);
                assert!(!relative.is_absolute(), "absolute schema ref: {reference}");
                assert!(
                    !relative
                        .components()
                        .any(|component| component == Component::ParentDir),
                    "schema ref escapes its package-local schema directory: {reference}"
                );
                let resolved = document_path.parent().unwrap().join(relative);
                let resolved = resolved.canonicalize().unwrap_or_else(|error| {
                    panic!("unresolved package-local schema ref {reference}: {error}")
                });
                assert!(
                    resolved.starts_with(schema_root),
                    "schema ref escaped package-local closure: {}",
                    resolved.display()
                );
                if visited.insert(resolved.clone()) {
                    let nested: serde_json::Value = serde_json::from_slice(
                        &std::fs::read(&resolved).expect("read referenced package-local schema"),
                    )
                    .expect("decode referenced package-local schema");
                    visit_schema_refs(schema_root, &resolved, &nested, visited);
                }
            }
            for nested in object.values() {
                visit_schema_refs(schema_root, document_path, nested, visited);
            }
        }
        serde_json::Value::Array(items) => {
            for nested in items {
                visit_schema_refs(schema_root, document_path, nested, visited);
            }
        }
        _ => {}
    }
}

#[test]
fn provider_workspace_install_schema_and_registration_are_package_local() {
    let package_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_root = package_root.join("schemas").canonicalize().unwrap();
    let descriptor_path = package_root.join("provider/asp-provider-workspace-install.json");
    let descriptor: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&descriptor_path).unwrap()).unwrap();

    let schema_reference = descriptor
        .get("$schema")
        .and_then(serde_json::Value::as_str)
        .expect("workspace install descriptor must declare its package-local schema");
    let descriptor_parent = descriptor_path.parent().unwrap();
    let schema_path = descriptor_parent
        .join(schema_reference)
        .canonicalize()
        .unwrap();
    assert!(
        schema_path.starts_with(&schema_root),
        "workspace install schema escaped package schema root: {}",
        schema_path.display()
    );

    let registration_reference = descriptor
        .get("providerRegistration")
        .and_then(serde_json::Value::as_str)
        .expect("workspace install descriptor must declare providerRegistration");
    let registration_path = descriptor_parent
        .join(registration_reference)
        .canonicalize()
        .unwrap();
    assert!(
        registration_path.starts_with(descriptor_parent),
        "provider registration escaped package provider directory: {}",
        registration_path.display()
    );
}

#[test]
fn provider_registration_schema_closure_is_package_local_and_complete() {
    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_root = package_root
        .join("schemas")
        .canonicalize()
        .expect("package-local schemas");
    let registration_schema = schema_root.join("provider-registration.schema.json");
    let route_schema = schema_root.join("provider-route.schema.json");
    assert!(registration_schema.is_file());
    assert!(route_schema.is_file());

    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&registration_schema).expect("read provider registration schema"),
    )
    .expect("decode provider registration schema");
    let mut visited = BTreeSet::from([registration_schema.clone()]);
    visit_schema_refs(&schema_root, &registration_schema, &value, &mut visited);
    assert!(visited.contains(&route_schema));
}

#[test]
fn rust_provider_registration_uses_only_the_package_local_schema_authority() {
    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registration_path = package_root.join("provider/asp-provider-registration.json");
    let registration: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&registration_path).expect("read Rust provider registration"),
    )
    .expect("decode Rust provider registration");
    let schema_ref = registration
        .get("$schema")
        .and_then(serde_json::Value::as_str)
        .expect("provider registration schema ref");
    assert_eq!(schema_ref, "../schemas/provider-registration.schema.json");
    let resolved = registration_path
        .parent()
        .unwrap()
        .join(schema_ref)
        .canonicalize()
        .expect("resolve package-local provider registration schema");
    assert!(resolved.starts_with(package_root.join("schemas")));
}

#[test]
fn rust_provider_registration_compiles_the_live_route_profile() {
    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registration: serde_json::Value = serde_json::from_slice(
        &std::fs::read(package_root.join("provider/asp-provider-registration.json"))
            .expect("read Rust provider registration"),
    )
    .expect("decode Rust provider registration");
    assert_eq!(registration["languageId"], "rust");
    assert_eq!(registration["providerId"], "asp-rust");
    let operations = registration["routes"]
        .as_array()
        .expect("live registration routes")
        .iter()
        .map(|route| {
            route["operation"]
                .as_str()
                .expect("route operation")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        operations,
        BTreeSet::from([
            "projection-batch".to_owned(),
            "query".to_owned(),
            "search".to_owned(),
            "search.owner".to_owned(),
        ])
    );
}
