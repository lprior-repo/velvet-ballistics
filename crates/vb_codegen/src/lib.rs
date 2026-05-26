//! vb_codegen stub — real implementation lives in velvet-optional repo.
//! This crate exists only to satisfy workspace compilation; the generated
//! workflow codegen feature is deferred per the master build contract.

/// Placeholder error for deferred codegen feature.
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    #[error("codegen deferred — see velvet-optional repo")]
    Deferred,

    #[error("unsupported IR feature: {feature}")]
    UnsupportedIr { feature: String },
}

/// Placeholder: always returns Err(Deferred). Generic to accept any workflow type.
pub fn emit_rust_workflow<T>(_workflow: &T) -> Result<String, CodegenError> {
    Err(CodegenError::Deferred)
}

/// Placeholder: always returns Err(Deferred). Generic to accept any plan type.
pub fn compare_generated_to_ir<T>(_source: &str, _plan: &T) -> Result<(), CodegenError> {
    Err(CodegenError::Deferred)
}

/// Placeholder: validates the generated Rust subset of a compiled workflow.
/// Always returns Err(Deferred) — real implementation lives in velvet-optional.
pub fn validate_generated_subset<T>(_workflow: &T) -> Result<(), CodegenError> {
    Err(CodegenError::Deferred)
}

/// Placeholder stub module for generated IR parity checks.
pub mod parity {
    use super::CodegenError;

    /// Placeholder: always returns Err(Deferred).
    pub fn ast_ir_parity<I, A>(_ir_thaw: &I, _ast: &A) -> Result<(), CodegenError> {
        Err(CodegenError::Deferred)
    }
}
