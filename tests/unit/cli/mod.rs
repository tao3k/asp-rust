pub(crate) mod support;

mod ast_patch;
mod ast_patch_scenarios;
mod basics;
mod behavior;
mod determinism;
mod dev_command_log;
mod evidence;
mod failure_frontier;
mod flow_drill {
    include!("flow_drill.rs");

    #[test]
    fn cli_rust_registry_advertises_ast_patch_apply_and_real_project_evidence_schema() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = temp.path();
        super::support::write_search_fixture(root);

        let registry = super::support::run_cli([
            "agent".as_ref(),
            "doctor".as_ref(),
            "--json".as_ref(),
            root.as_os_str(),
        ]);
        assert!(registry.status.success(), "{registry:?}");
        let registry_json =
            serde_json::from_slice::<serde_json::Value>(&registry.stdout).expect("registry json");
        let language = registry_json["languages"][0].as_object().expect("language");

        let methods = language["methods"].as_array().expect("methods");
        for method in ["ast-patch/dry-run", "ast-patch/apply"] {
            assert!(
                methods
                    .iter()
                    .any(|candidate| candidate.as_str() == Some(method)),
                "missing method {method}: {methods:?}"
            );
        }

        let schemas = language["schemas"].as_array().expect("schemas");
        assert!(
            schemas.iter().any(|schema| {
                schema["schemaId"].as_str()
                    == Some("agent.semantic-protocols.rust-ast-patch-real-project-evidence")
                    && schema["path"].as_str()
                        == Some("schemas/rust-ast-patch-real-project-evidence.v1.schema.json")
            }),
            "missing real-project evidence schema: {schemas:?}"
        );

        let method_descriptors = language["methodDescriptors"]
            .as_array()
            .expect("methodDescriptors");
        for (method, mutation_available) in
            [("ast-patch/dry-run", false), ("ast-patch/apply", true)]
        {
            let descriptor = method_descriptors
                .iter()
                .find(|descriptor| descriptor["method"].as_str() == Some(method))
                .unwrap_or_else(|| panic!("missing descriptor {method}"));
            assert_eq!(descriptor["command"], "ast-patch");
            assert_eq!(descriptor["mutationAvailable"], mutation_available);
            assert!(
                descriptor["outputSchemaIds"]
                    .as_array()
                    .expect("outputSchemaIds")
                    .iter()
                    .any(|schema| {
                        schema.as_str()
                            == Some("agent.semantic-protocols.semantic-ast-patch-receipt")
                    }),
                "missing ast-patch receipt output schema: {descriptor:?}"
            );
        }
    }
}
mod projection;
mod proof;
#[cfg(feature = "search")]
mod query;
mod receipt;
mod registry_codeql;
mod review;
mod schema;
mod schema_isolation;
#[cfg(feature = "search")]
mod search;
#[cfg(feature = "search")]
mod search_api_callables;
#[cfg(feature = "search")]
mod search_json_type_surfaces;
#[cfg(feature = "search")]
mod search_lab;
mod search_owner_items;
#[cfg(feature = "search")]
mod search_policy;
#[cfg(feature = "search")]
mod search_query;
mod semantic_syntax_refs;
