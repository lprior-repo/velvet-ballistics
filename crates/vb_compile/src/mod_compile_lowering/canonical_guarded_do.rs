#![forbid(unsafe_code)]
//! Guarded Do lowering: conditional action execution with Choose node.

use super::CanonicalScope;
use super::canonical_do::lower_canonical_do;
use super::canonical_scalar::compile_canonical_expression;
use crate::CompileError;
use crate::mod_compile_errors::CompileErrors;
use crate::mod_compile_lowering::SlotCompiler;
use vb_core::{CompiledNode, CompiledNodeKind, ExprBranch, ExprIdx, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

/// Lowers a top-level Do step, optionally guarded by a condition.
///
/// When a condition is present, emits a Choose node at the given id that
/// branches to the real Do node at id+1. The Do node uses the original
/// authored step index for slot computation.
pub(super) fn lower_top_level_do(
    step: &crate::StepAst,
    index: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    let (action, input) = extract_do_fields(step, index)?;

    if let Some(condition_expr) = &step.condition {
        let (expr_idx, inferred) = compile_canonical_expression(condition_expr, scope, builder)?;
        validate_condition_type(inferred)?;

        let do_id = compute_next_step_id(id)?;
        emit_choose_node(id, expr_idx, do_id, next, builder);

        return lower_canonical_do(do_id, index, action, input, next, scope, builder);
    }

    lower_canonical_do(id, index, action, input, next, scope, builder)
}

fn validate_condition_type(inferred: ExprType) -> Result<(), CompileErrors> {
    if inferred != ExprType::Bool {
        return Err(CompileErrors(vec![CompileError::TypeMismatch {
            field: "if",
            expected: "boolean",
            found: expr_type_name(&inferred),
        }]));
    }
    Ok(())
}

fn emit_choose_node(
    id: StepIdx,
    condition: ExprIdx,
    target: StepIdx,
    otherwise: Option<StepIdx>,
    builder: &mut SlotCompiler,
) {
    builder.push_node(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Choose {
            branches: Box::from([ExprBranch { condition, target }]),
            otherwise,
        },
    });
}

fn extract_do_fields(step: &crate::StepAst, index: usize) -> Result<(&str, &str), CompileErrors> {
    match &step.primitive {
        crate::StepPrimitive::Do { action, input } => Ok((action, input)),
        _ => Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "do",
            expected: "Do primitive",
        }])),
    }
}

fn compute_next_step_id(id: StepIdx) -> Result<StepIdx, CompileErrors> {
    id.get().checked_add(1).map(StepIdx::new).ok_or_else(|| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: id.as_usize(),
        }])
    })
}

fn expr_type_name(t: &ExprType) -> &'static str {
    match t {
        ExprType::Bool => "boolean",
        ExprType::I64 => "integer",
        ExprType::F64 => "float",
        ExprType::Text => "string",
        ExprType::Null => "null",
        ExprType::List => "list",
        ExprType::Object => "object",
        ExprType::Unknown => "unknown",
        _ => "unknown",
    }
}
