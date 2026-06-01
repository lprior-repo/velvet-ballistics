#![forbid(unsafe_code)]
//! Stack-based expression bytecode evaluator.
//!
//! This module is organized into:
//! - `core`: main program evaluation entry points
//! - `ops`: binary and unary operation implementations
//! - `helpers`: helper function dispatch and implementations
//! - `stack`: stack push/pop operations
//! - `type_enforcers`: type checking utilities

pub mod core;
pub mod helpers;
pub mod ops;
pub mod stack;
pub mod type_enforcers;

// Re-exports for backwards compatibility
pub use crate::lexer::{BinaryOp, UnaryOp};
pub use crate::parser::ExprHelper;
pub use crate::{ExprError, ExprResult};
pub use vb_core::limits::MAX_EXPRESSION_STACK;

// Main entry points re-exported from core
pub use core::{eval_expr_program, eval_expr_program_with_store};

// Binary/unary ops re-exported from ops
pub use ops::{eval_binary_op, eval_unary_op};

// Helper eval re-exported from helpers
pub use helpers::{eval_helper, eval_helper_with_store};

#[path = "../eval_tests.rs"]
mod tests;
