#![forbid(unsafe_code)]
//! Bounded generated-workflow compatibility surface.

mod emit;
mod error;
pub mod parity;
mod validate;

pub use emit::{compare_generated_to_ir, emit_rust_workflow};
pub use error::CodegenError;
pub use validate::validate_generated_subset;
