#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::return_self_not_must_use)]
//! Generated Rust workflow mode for velvet-ballastics maxperf builds.
//!
//! Compiles `CompiledWorkflow` IR into native Rust source that passes the same
//! lint gates as first-party code and preserves identical observable semantics.

use thiserror::Error;

mod codegen;
mod emit;
mod validate;

pub use codegen::emit_rust_workflow;
pub use validate::validate_generated_subset;

// Re-export public emit functions for backwards compatibility
pub use emit::{
    emit_action_boundary, emit_action_match_dispatch, emit_drive_function, emit_expr_function,
    emit_finish, emit_ids, emit_resource_contract, emit_step_function, emit_trybuild_fixture,
    compile_check_generated_rust, format_generated_rust,
};

// Re-export comparison utilities
pub use codegen::compare_generated_to_ir;

/// Codegen failures with stable typed diagnostics.
#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("unsupported generated Rust IR feature: {feature}")]
    UnsupportedIr { feature: &'static str },
    #[error("codegen output exceeds buffer capacity")]
    FormatBufferOverflow,
    #[error("rustfmt failed: {detail}")]
    RustfmtFailed { detail: String },
    #[error("compile check failed: {detail}")]
    CompileCheckFailed { detail: String },
    #[error("semantic equivalence violation: {detail}")]
    SemanticMismatch { detail: String },
    #[error("codegen IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("trybuild fixture error: {detail}")]
    TrybuildFixture { detail: String },
}

/// Result alias for codegen operations.
pub type CodegenResult<T> = Result<T, CodegenError>;
