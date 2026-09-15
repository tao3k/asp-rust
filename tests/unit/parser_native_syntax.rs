use std::fs;

use tempfile::TempDir;

use crate::parser::{RustUseGlobScopeKind, RustUseVisibilityKind, parse_rust_file};

#[test]
fn native_syntax_facts_record_cfg_and_module_declaring_macros() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("lib.rs");
    fs::write(
        &source,
        "#[cfg(feature = \"fs\")]\ncompile_error!(\"fs is not supported here\");\nasp_rust_cargo_test_gate!();\nasp_rust_cargo_test_gate!(advice = allow, config = { default_asp_rust_config() });\nfn main() {\n    asp_rust::assert_asp_rust_cargo_check_clean_from_env_with_config(&config());\n}\ncfg_feature! {\n    pub(crate) mod optional;\n}\ncfg_macros! {\n    pub use crate::optional::Thing;\n}\ncfg_nested! {\n    cfg_inner! {\n        pub(crate) use crate::optional::Other;\n    }\n}\ncfg_bad! {\n    pub fn leaked() {}\n}\nmacro_rules! declare_mod { () => { mod generated; } }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let compile_error = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("compile_error"))
        .expect("compile_error macro fact");
    assert!(compile_error.has_cfg_attr);
    assert!(!compile_error.macro_declares_module);
    assert!(!compile_error.macro_body_is_facade_boundary);
    let compile_error_invocation = module
        .syntax_facts
        .macro_invocations
        .iter()
        .find(|invocation| invocation.terminal_name == "compile_error")
        .expect("compile_error invocation");
    assert!(compile_error_invocation.argument_token_count > 0);
    let empty_gate_invocation = module
        .syntax_facts
        .macro_invocations
        .iter()
        .find(|invocation| invocation.terminal_name == "asp_rust_cargo_test_gate")
        .expect("empty gate invocation");
    assert_eq!(empty_gate_invocation.argument_token_count, 0);
    assert!(empty_gate_invocation.argument_top_level_idents.is_empty());
    let configured_gate_invocation = module
        .syntax_facts
        .macro_invocations
        .iter()
        .find(|invocation| {
            invocation.terminal_name == "asp_rust_cargo_test_gate"
                && invocation.argument_token_count > 0
        })
        .expect("configured gate invocation");
    assert!(
        configured_gate_invocation
            .argument_top_level_idents
            .iter()
            .any(|ident| ident == "config"),
        "{configured_gate_invocation:?}"
    );
    let configured_build_gate_call = module
        .syntax_facts
        .function_calls
        .iter()
        .find(|invocation| {
            invocation.terminal_name == "assert_asp_rust_cargo_check_clean_from_env_with_config"
        })
        .expect("configured build gate call");
    assert_eq!(configured_build_gate_call.argument_token_count, 1);

    let cfg_feature = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("cfg_feature"))
        .expect("cfg_feature macro fact");
    assert!(!cfg_feature.has_cfg_attr);
    assert!(cfg_feature.macro_declares_module);
    assert!(cfg_feature.macro_body_is_facade_boundary);

    let cfg_macros = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("cfg_macros"))
        .expect("cfg_macros macro fact");
    assert!(!cfg_macros.macro_declares_module);
    assert!(cfg_macros.macro_body_is_facade_boundary);

    let cfg_nested = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("cfg_nested"))
        .expect("cfg_nested macro fact");
    assert!(!cfg_nested.macro_declares_module);
    assert!(cfg_nested.macro_body_is_facade_boundary);

    let cfg_bad = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("cfg_bad"))
        .expect("cfg_bad macro fact");
    assert!(!cfg_bad.macro_body_is_facade_boundary);

    let macro_rules = module
        .syntax_facts
        .top_level_items
        .iter()
        .find(|item| item.macro_name.as_deref() == Some("macro_rules"))
        .expect("macro_rules macro fact");
    assert!(macro_rules.macro_declares_module);
    assert!(!macro_rules.macro_body_is_facade_boundary);
}

#[test]
fn native_syntax_facts_record_glob_scope_kinds() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("lib.rs");
    fs::write(
        &source,
        "use crate::gateway::studio::studio_repo_sync_api_tests::*;\n\
         use super::*;\n\
         use external::prelude::*;\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let glob_scopes = module
        .syntax_facts
        .use_statements
        .iter()
        .flat_map(|use_statement| &use_statement.glob_imports)
        .map(|glob| (glob.rendered_path(), glob.scope_kind))
        .collect::<Vec<_>>();
    assert_eq!(
        glob_scopes,
        vec![
            (
                "crate::gateway::studio::studio_repo_sync_api_tests::*".to_string(),
                RustUseGlobScopeKind::CrateOwner,
            ),
            ("super::*".to_string(), RustUseGlobScopeKind::ParentScope,),
            (
                "external::prelude::*".to_string(),
                RustUseGlobScopeKind::External,
            ),
        ]
    );
}

#[test]
fn native_syntax_facts_record_deep_relative_scope_imports() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("domain.rs");
    fs::write(
        &source,
        "pub(super) use super::super::{assert_workspace_patch_intent_artifact, local_gxi};\n\
         pub use super::{super::MissingOwner, sibling::Thing};\n\
         use self::leaf::Leaf;\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let deep_relative_imports = module
        .syntax_facts
        .use_statements
        .iter()
        .flat_map(|use_statement| &use_statement.deep_relative_imports)
        .map(|import| {
            (
                import.rendered_path(),
                import.parent_hops,
                import.line,
                import.visibility.clone(),
                import.is_reexport,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        deep_relative_imports,
        vec![
            (
                "super::super::assert_workspace_patch_intent_artifact".to_string(),
                2,
                1,
                RustUseVisibilityKind::Super,
                true,
            ),
            (
                "super::super::local_gxi".to_string(),
                2,
                1,
                RustUseVisibilityKind::Super,
                true,
            ),
            (
                "super::super::MissingOwner".to_string(),
                2,
                2,
                RustUseVisibilityKind::Public,
                true,
            ),
        ]
    );
}

#[test]
fn native_syntax_facts_record_reexports_and_path_references() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("support.rs");
    fs::write(
        &source,
        "pub(super) use crate::domain::{Original as Alias, Plain};\n\
         fn helper(value: Alias) -> Plain { Plain::from(value) }\n",
    )
    .expect("write source");

    let module = parse_rust_file(&source);

    let use_statement = module
        .syntax_facts
        .use_statements
        .first()
        .expect("use statement");
    assert_eq!(use_statement.visibility, RustUseVisibilityKind::Super);
    assert_eq!(
        use_statement
            .reexports
            .iter()
            .map(|reexport| {
                (
                    reexport.rendered_source_path(),
                    reexport.exposed_name.as_str(),
                    reexport.visibility.clone(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "crate::domain::Original".to_string(),
                "Alias",
                RustUseVisibilityKind::Super,
            ),
            (
                "crate::domain::Plain".to_string(),
                "Plain",
                RustUseVisibilityKind::Super,
            ),
        ]
    );
    let references = module
        .syntax_facts
        .path_references
        .iter()
        .map(|reference| reference.terminal_name.as_str())
        .collect::<Vec<_>>();
    assert!(references.contains(&"Alias"), "{references:?}");
    assert!(references.contains(&"Plain"), "{references:?}");
}
