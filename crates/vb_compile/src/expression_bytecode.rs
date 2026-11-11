#![forbid(unsafe_code)]
//! Cold expression bytecode lowering.

use crate::CompileError;
use crate::expression::ParsedExpression;
use vb_core::{AccessorProgram, ConstValue, ExprOp, ExprProgram, SlotIdx, SymbolId, WorkflowError};

#[path = "expression_bytecode_slot_refs.rs"]
#[cfg(test)]
mod slot_refs;

#[path = "expression_bytecode_step_refs.rs"]
mod step_refs;

#[path = "expression_bytecode_lowering.rs"]
mod lowering;

/// Lowers a parsed expression tree into bounded postfix expression bytecode.
///
/// String literals and source references require the later symbol/accessor tables,
/// so Phase 10 rejects them instead of smuggling runtime string lookup into IR.
#[cfg(test)]
fn compile_expr_to_bytecode(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(
        expression,
        constants,
        &mut slot_refs::RejectingReferenceResolver,
    )
}

/// Lowers an expression and appends slot-rooted accessor programs for direct
/// slot references and list-index nested path references.
///
/// Object field segments require a compiler-owned symbol table. Until that
/// table exists in `vb_compile`, they are rejected instead of guessing
/// `SymbolId`s.
#[cfg(test)]
fn compile_expr_to_bytecode_with_accessors(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(
        expression,
        constants,
        &mut slot_refs::SlotAccessorReferenceResolver { accessors },
    )
}

/// Lowers an expression with step reference resolution support.
///
/// This function extends `compile_expr_to_bytecode_with_accessors` to also
/// resolve `$step.<id>` and `$steps.<id>` references using the provided
/// step name to slot mapping.
///
/// For bare step references like `$steps.build_result`, returns `LoadSlot(slot)`
/// where slot is the output slot of the named step.
///
/// For step references with field accessors like `$steps.build.result`, creates
/// an AccessorProgram with the step's output slot as root and the field as path.
pub(crate) fn compile_expr_to_bytecode_with_step_slots(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    accessors: &mut Vec<AccessorProgram>,
    step_slots: &[(Box<str>, SlotIdx)],
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(
        expression,
        constants,
        &mut step_refs::StepSlotReferenceResolver {
            step_slots,
            accessors,
        },
    )
}

/// Lowers a parsed expression tree into bytecode using compiler-owned reference
/// resolution.
pub(crate) fn compile_expr_to_bytecode_with_resolver(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<ExprProgram, CompileError> {
    let mut ops = Vec::new();
    lowering::lower_expr(expression, constants, &mut ops, resolver)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice())
        .map_err(|error| CompileError::Workflow(WorkflowError::Expression(error)))
}

/// Compiler reference resolver used by expression bytecode lowering.
pub(crate) trait ExpressionReferenceResolver {
    /// Returns the bytecode operation for a source reference.
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError>;

    fn intern_text(&mut self, _text: &str) -> Result<SymbolId, CompileError> {
        Err(CompileError::ExpressionLoweringUnsupported {
            feature: "text constants".into(),
        })
    }
}

fn split_reference_tail(tail: &str) -> (&str, Option<&str>) {
    match tail.split_once('.') {
        Some((slot, path)) => (slot, Some(path)),
        None => (tail, None),
    }
}

#[cfg(test)]
#[path = "expression_bytecode_tests.rs"]
mod tests;
