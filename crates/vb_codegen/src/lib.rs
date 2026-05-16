pub use vb_core::CompiledWorkflow;

pub use crate::codegen::CodegenError;
pub use crate::codegen::CodegenResult;

#[cfg(kani)]
mod kani_generated_runtime;

pub mod codegen;

pub use codegen::{
    emit_action_match_dispatch, emit_constants, emit_drive_function, emit_expr_function,
    emit_finish, emit_ids, emit_list_store_contract, emit_resource_contract, emit_rust_workflow,
    emit_step_function, emit_value_store_contract, format_generated_rust, compile_check_generated_rust,
    compare_generated_to_ir, emit_trybuild_fixture, validate_generated_subset,
};

pub(crate) use codegen::emit_action_boundary;

pub(crate) use codegen::{
    emit_unsupported_step, write_header, write_next_or_error,
};

#[cfg(not(miri))]
mod tests;
