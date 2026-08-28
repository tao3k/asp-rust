//! Internal parser substrate for Rust source and Cargo project facts.

#[cfg(any(feature = "search", test))]
mod cargo_cfg;
mod cargo_dependency_facts;
mod cargo_manifest;
mod cargo_test_targets;
mod location;
mod module_tree;
pub(crate) mod native_syntax;
mod parsed_module;
pub(crate) use parsed_module::parse_rust_source_syntax;
mod path_resolution;
mod reasoning_tree;
mod source_metrics;
mod source_path;
#[cfg(any(feature = "cli", feature = "search", test))]
pub(crate) mod syntax_abi;
mod use_tree;

#[cfg(any(feature = "search", test))]
pub(crate) use cargo_cfg::parse_cargo_cfg_facts;
pub(crate) use cargo_dependency_facts::{
    CargoDependencyFacts, CargoDependencyKind, parse_cargo_dependency_facts,
};
pub(crate) use cargo_manifest::{
    CargoBenchTargetFacts, CargoManifestFacts, parse_cargo_manifest, parse_required_cargo_manifest,
};
#[cfg(feature = "cli")]
pub(crate) use cargo_manifest::{cargo_package_root_for_path, cargo_project_root_for_path};
#[cfg(feature = "cli")]
pub(crate) use cargo_manifest::{
    cargo_workspace_member_roots_from_candidates, parse_cargo_project_facts,
    parse_cargo_workspace_member_roots, workspace_member_pattern_matches,
};
pub(crate) use cargo_test_targets::parse_cargo_test_targets;
pub(crate) use location::{file_location, path_line_location, source_line, span_location};
pub(crate) use module_tree::RustModuleChildEdge;
#[cfg(test)]
pub(crate) use module_tree::RustModuleChildEdgeKind;
pub(crate) use native_syntax::{
    RustFunctionControlFlowSyntax, RustNativeSyntaxFacts, RustPublicEnumTupleVariantFieldSyntax,
    RustPublicEnumVariantFieldSyntax, RustPublicStructFieldSyntax, RustPublicTypeAliasSyntax,
    RustTopLevelItemSyntax,
};
#[cfg(feature = "cli")]
pub(crate) use parsed_module::parse_rust_source;
pub(crate) use parsed_module::{ParsedRustModule, parse_rust_file};
pub(crate) use path_resolution::resolve_rust_path_attr;
pub(crate) use reasoning_tree::{
    RustReasoningImportFacts, RustReasoningModuleFacts, RustReasoningOwnerBranchFacts,
    RustReasoningOwnerBranchRole, RustReasoningOwnerDependencyFacts, RustReasoningTreeFacts,
    rust_reasoning_tree_facts,
};
pub(crate) use source_metrics::RustSourceMetrics;
pub(crate) use source_path::{RustSourcePathFacts, rust_source_path_facts};
pub(crate) use use_tree::{
    RustUseDeepRelativeImportSyntax, RustUseGlobScopeKind, RustUseImportRootKind,
    RustUseImportSyntax, RustUseStatementContext, RustUseStatementSyntax, RustUseVisibilityKind,
    rust_use_statement_syntax,
};
