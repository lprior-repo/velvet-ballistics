#![forbid(unsafe_code)]
//! Cold typed workflow AST for the compiler boundary.
//!
//! YAML remains confined to `vb-compiler`; runtime crates consume only lowered
//! native Rust IR. This module preserves source-language intent before IR
//! lowering erases names and diagnostic metadata.

pub(crate) mod marks;
mod parse;
mod types;

pub(crate) use parse::parse_workflow_ast;
pub use types::{
    AstExpression, AstMapEntry, AstValue, StepAst, StepKindAst, StepPrimitiveAst, TriggerAst,
    WorkflowAst,
};

#[cfg(test)]
mod tests;
