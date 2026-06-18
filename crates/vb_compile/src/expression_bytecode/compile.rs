//! Public API entry points for expression bytecode compilation.

use crate::CompileError;
use crate::expression::ParsedExpression;
use vb_core::{AccessorProgram, ExprProgram, WorkflowError};

use super::lower::lower_expr;
use super::resolver::{ExpressionReferenceResolver, RejectingReferenceResolver, StepSlotReferenceResolver, SlotAccessorReferenceResolver};

/// Lowers a parsed expression tree into bounded postfix expression bytecode.
///
/// String literals and source references require the later symbol/accessor tables,
/// so Phase 10 rejects them instead of smuggling runtime string lookup into IR.
pub fn compile_expr_to_bytecode(
    expression: &ParsedExpression,
    constants: &mut Vec<vb_core::ConstValue>,
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(expression, constants, &mut RejectingReferenceResolver)
}

/// Lowers an expression and appends slot-rooted accessor programs for direct
/// slot references and list-index nested path references.
///
/// Object field segments require a compiler-owned symbol table. Until that
/// table exists in `vb_compile`, they are rejected instead of guessing
/// `SymbolId`s.
pub fn compile_expr_to_bytecode_with_accessors(
    expression: &ParsedExpression,
    constants: &mut Vec<vb_core::ConstValue>,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(
        expression,
        constants,
        &mut SlotAccessorReferenceResolver { accessors },
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
    constants: &mut Vec<vb_core::ConstValue>,
    accessors: &mut Vec<AccessorProgram>,
    step_slots: &[(Box<str>, vb_core::SlotIdx)],
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(
        expression,
        constants,
        &mut StepSlotReferenceResolver {
            step_slots,
            accessors,
        },
    )
}

/// Lowers a parsed expression tree into bytecode using compiler-owned reference
/// resolution.
pub(crate) fn compile_expr_to_bytecode_with_resolver(
    expression: &ParsedExpression,
    constants: &mut Vec<vb_core::ConstValue>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<ExprProgram, CompileError> {
    let mut ops = Vec::new();
    lower_expr(expression, constants, &mut ops, resolver)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice())
        .map_err(|error| CompileError::Workflow(WorkflowError::Expression(error)))
}
