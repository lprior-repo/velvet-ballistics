//! vb_codegen stub — real implementation lives in velvet-optional repo.
//! This crate exists only to satisfy workspace compilation; the generated
//! workflow codegen feature is deferred per the master build contract.

/// Placeholder error for deferred codegen feature.
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    #[error("codegen deferred — see velvet-optional repo")]
    Deferred,
}

/// Placeholder: always returns Err(Deferred). Generic to accept any workflow type.
pub fn emit_rust_workflow<T>(_workflow: &T) -> Result<String, CodegenError> {
    Err(CodegenError::Deferred)
}

/// Placeholder: always returns Err(Deferred). Generic to accept any plan type.
pub fn compare_generated_to_ir<T>(_source: &str, _plan: &T) -> Result<(), CodegenError> {
    Err(CodegenError::Deferred)
}
