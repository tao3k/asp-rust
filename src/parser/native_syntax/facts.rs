//! Native Rust syntax fact data types.

use std::path::PathBuf;

use super::control_flow::RustFunctionControlFlowSyntax;
use super::data_shape::{
    RustPublicEnumTupleVariantFieldSyntax, RustPublicEnumVariantFieldSyntax,
    RustPublicStructFieldSyntax, RustPublicTypeAliasSyntax, RustPublicTypeGenericBoundSyntax,
};
use super::item_projection::RustItemProjectionNodeSyntax;
use super::native_abi_boundary::RustNativeAbiSurfaceSyntax;
use super::process_boundary::RustProcessCommandExecutionSyntax;
use super::signature::{
    RustFunctionDynamicJsonApiSyntax, RustFunctionParamSyntax, RustFunctionReturnSyntax,
    RustFunctionTupleApiSyntax,
};
use super::tokio_runtime_boundary::RustTokioRuntimeOperationSyntax;
use crate::parser::RustUseStatementSyntax;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustPublicApiCallableSyntax {
    pub line: usize,
    pub kind: &'static str,
    pub name: String,
    pub has_doc: bool,
    pub is_public: bool,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustNativeSyntaxFacts {
    pub has_module_doc: bool,
    pub top_level_items: Vec<RustTopLevelItemSyntax>,
    pub public_api_callables: Vec<RustPublicApiCallableSyntax>,
    pub cfg_test_modules: Vec<RustModuleDeclarationSyntax>,
    pub test_function_count: usize,
    pub use_statements: Vec<RustUseStatementSyntax>,
    pub path_references: Vec<RustPathReferenceSyntax>,
    pub public_struct_fields: Vec<RustPublicStructFieldSyntax>,
    pub public_enum_variant_fields: Vec<RustPublicEnumVariantFieldSyntax>,
    pub public_enum_tuple_variant_fields: Vec<RustPublicEnumTupleVariantFieldSyntax>,
    pub public_type_generic_bounds: Vec<RustPublicTypeGenericBoundSyntax>,
    pub public_type_aliases: Vec<RustPublicTypeAliasSyntax>,
    pub public_function_params: Vec<RustFunctionParamSyntax>,
    pub public_function_returns: Vec<RustFunctionReturnSyntax>,
    pub public_tuple_api_surfaces: Vec<RustFunctionTupleApiSyntax>,
    pub public_dynamic_json_api_surfaces: Vec<RustFunctionDynamicJsonApiSyntax>,
    pub native_abi_surfaces: Vec<RustNativeAbiSurfaceSyntax>,
    pub process_command_executions: Vec<RustProcessCommandExecutionSyntax>,
    pub tokio_runtime_operations: Vec<RustTokioRuntimeOperationSyntax>,
    pub all_function_control_flows: Vec<RustFunctionControlFlowSyntax>,
    pub public_function_control_flows: Vec<RustFunctionControlFlowSyntax>,
    pub macro_invocations: Vec<RustInvocationSyntax>,
    pub function_calls: Vec<RustInvocationSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustTopLevelItemSyntax {
    pub line: usize,
    pub end_line: usize,
    pub kind: &'static str,
    pub name: Option<String>,
    pub impl_target_name: Option<String>,
    pub trait_owner_name: Option<String>,
    pub has_doc: bool,
    pub is_public: bool,
    pub is_public_use: bool,
    pub is_use: bool,
    pub is_extern_crate: bool,
    pub is_macro: bool,
    pub has_proc_macro_export_attr: bool,
    pub has_cfg_attr: bool,
    pub is_implementation_item: bool,
    pub function_name: Option<String>,
    pub macro_name: Option<String>,
    pub macro_declares_module: bool,
    pub macro_body_is_facade_boundary: bool,
    pub include_target: Option<String>,
    pub module: Option<RustModuleDeclarationSyntax>,
    pub cfg_predicates: Vec<String>,
    pub projection_responsibilities: Vec<&'static str>,
    pub projection_nodes: Vec<RustItemProjectionNodeSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustModuleDeclarationSyntax {
    pub line: usize,
    pub ident: String,
    pub path_attr: Option<String>,
    pub resolved_path_attr: Option<PathBuf>,
    pub is_inline: bool,
    pub is_cfg_test: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustPathReferenceSyntax {
    pub line: usize,
    pub segments: Vec<String>,
    pub terminal_name: String,
    pub is_test_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustInvocationSyntax {
    pub line: usize,
    pub terminal_name: String,
    pub argument_token_count: usize,
    pub argument_top_level_idents: Vec<String>,
}

impl RustNativeSyntaxFacts {
    pub(crate) fn contains_function_call_named(&self, names: &[&str]) -> bool {
        self.function_calls
            .iter()
            .any(|invocation| names.contains(&invocation.terminal_name.as_str()))
    }
}
