//! Cold expression bytecode lowering.
//!
//! Entry points compile a `ParsedExpression` tree into bounded postfix
//! `ExprProgram` bytecode, optionally resolving `$slot`/`$steps` references
//! via the supplied `ExpressionReferenceResolver`.

#![forbid(unsafe_code)]

mod compile;
mod lower;
mod opcode;
mod reference;
mod resolver;

pub(crate) use compile::compile_expr_to_bytecode_with_step_slots;
pub use compile::{compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors};
pub(crate) use opcode::{binary_op, helper_op, validate_helper_arity};

#[cfg(test)]
#[path = "../expression_bytecode_tests.rs"]
mod tests;
