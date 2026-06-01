#![forbid(unsafe_code)]
//! Type taint validation entry point.

use crate::ast::WorkflowAst;
use crate::compile::type_taint::facts::Facts;
use crate::compile::type_taint::steps::validate_steps;
use crate::CompileErrors;

/// Validates a workflow AST for type correctness and taint flow.
pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileErrors> {
    let mut facts = Facts::new(ast);
    validate_steps(ast, &mut facts)
}
