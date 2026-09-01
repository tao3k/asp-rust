//! Native Rust syntax facts shared by harness policies.

mod async_queue_boundary;
mod async_task_lifecycle;
mod collect;
mod control_flow;
mod data_shape;
mod facts;
mod invocation_facts;
mod item_facts;
pub(crate) mod item_projection;
mod module_facts;
mod native_abi_boundary;
mod path_facts;
pub(crate) mod process_boundary;
#[cfg(feature = "search")]
pub(crate) mod projection_code;
mod public_api_callable;
mod select_cancellation_safety;
pub(crate) mod signature;
mod sync_lock_boundary;
mod timeout_cancellation_safety;
pub(crate) mod tokio_runtime_boundary;

pub(crate) use collect::rust_native_syntax_facts;
pub(crate) use control_flow::RustFunctionControlFlowSyntax;
pub(crate) use data_shape::{
    RustPublicEnumTupleVariantFieldSyntax, RustPublicEnumVariantFieldSyntax,
    RustPublicStructFieldSyntax, RustPublicTypeAliasSyntax,
};
pub(crate) use facts::{RustNativeSyntaxFacts, RustTopLevelItemSyntax};
