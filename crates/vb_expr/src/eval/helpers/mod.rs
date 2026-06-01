#![forbid(unsafe_code)]
//! Helper function dispatch and implementations.
//!
//! Organized into:
//! - `dispatch`: helper operation dispatch from bytecode and public API
//! - `impls`: store-aware helper implementations

pub mod dispatch;
pub mod impls;

// Re-export for convenience
pub use dispatch::{eval_helper, eval_helper_op_with_store, eval_helper_with_store};
