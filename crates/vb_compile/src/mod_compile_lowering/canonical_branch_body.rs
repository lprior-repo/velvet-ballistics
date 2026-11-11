//! Recursive lexical bodies. Bindings never escape their branch or loop body.

use super::super::part_01::ForEachInputs;
use super::*;
use vb_core::SlotIdx;
use vb_expr::expr_typecheck::ExprType;

#[path = "branch_bindings.rs"]
mod branch_bindings;
use branch_bindings::BranchBindings;

/// Context shared by every body level of one top-level step.
///
/// `inputs` resolves nested `for_each` sources against declared inputs,
/// `step_names` resolves nested `choose.otherwise` labels against top-level
/// steps, and `index` is the outer diagnostic step index used by every level.
#[derive(Clone, Copy)]
pub(in crate::mod_compile_lowering) struct BodyCtx<'a> {
    pub(in crate::mod_compile_lowering) inputs: &'a ForEachInputs,
    pub(in crate::mod_compile_lowering) step_names: &'a [Box<str>],
    pub(in crate::mod_compile_lowering) index: usize,
}

pub(in crate::mod_compile_lowering) fn emit_scoped_branch_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    ctx: BodyCtx<'_>,
    builder: &mut SlotCompiler,
) -> Result<usize, CompileErrors> {
    let _ = emit_scoped_flat_body(
        body,
        start,
        common_next,
        scope,
        ctx,
        builder,
        "choose",
        None,
    )?;
    Ok(body.len())
}

/// Emit one node per body step and return the final body's value slot.
///
/// Container steps (`for_each`, `collect`, `repeat`, `choose`) lower
/// recursively through the same entry points as top-level steps, so layout
/// widths and emitted nodes agree by construction. A nested container
/// evaluates to its inner body's result slot, except `choose`, which binds
/// no value: its marker slot stays unwritten and downstream reads of it fail
/// closed.
#[allow(clippy::too_many_arguments)]
pub(in crate::mod_compile_lowering) fn emit_scoped_flat_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    ctx: BodyCtx<'_>,
    builder: &mut SlotCompiler,
    primitive: &'static str,
    final_target: Option<SlotIdx>,
) -> Result<(Option<SlotIdx>, ExprType), CompileErrors> {
    let mut local = BranchBindings::new(scope, primitive == "for_each", ctx);
    let mut result = None;
    let mut result_type = ExprType::Unknown;
    let mut cursor = start;
    for (index, step) in body.iter().enumerate() {
        reject_body_controls(step, index)?;
        let width =
            canonical_body_step_width(&step.primitive).map_err(|e| CompileErrors(vec![e]))?;
        let last = index.checked_add(1).ok_or_else(|| index_error(index))? == body.len();
        let next = if last {
            common_next
        } else {
            advance_cursor(cursor, width, primitive)?
        };
        let target = if last { final_target } else { None };
        let (slot, ty) = local.lower(step, cursor, next, builder, target)?;
        result = Some(slot);
        result_type = ty;
        cursor = advance_cursor(cursor, width, primitive)?;
    }
    Ok((result, result_type))
}

fn advance_cursor(
    cursor: StepIdx,
    width: usize,
    primitive: &'static str,
) -> Result<StepIdx, CompileErrors> {
    let offset = u16::try_from(width).map_err(|_| index_error(width))?;
    checked_step_offset(cursor, offset, primitive, "body").map_err(|e| CompileErrors(vec![e]))
}

fn index_error(value: usize) -> CompileErrors {
    CompileErrors(vec![CompileError::StepIndexOutOfRange { value }])
}

fn alloc_body_slot(builder: &mut SlotCompiler) -> Result<SlotIdx, CompileErrors> {
    builder.allocate_slot().map_err(|e| CompileErrors(vec![e]))
}

fn reject_body_controls(step: &crate::StepAst, index: usize) -> Result<(), CompileErrors> {
    let fields = [
        (step.name.is_some(), "name"),
        (step.condition.is_some(), "if"),
        (step.with.is_some(), "with"),
        (step.retry.is_some(), "try_again"),
        (step.on_error.is_some(), "on_error"),
        (step.then.is_some(), "then"),
    ];
    if let Some((_, field)) = fields.iter().find(|(present, _)| *present) {
        return Err(CompileErrors(vec![
            CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from(*field),
            },
        ]));
    }
    Ok(())
}
