//! Scoped lexical bodies. Bindings never escape their branch or loop body.

use super::*;
#[cfg(test)]
use std::collections::HashMap;
use vb_core::SlotIdx;
use vb_expr::expr_typecheck::ExprType;

#[path = "branch_bindings.rs"]
mod bindings;
#[path = "generic_body_values.rs"]
mod body_values;
use bindings::BranchBindings;

pub(super) fn emit_scoped_branch_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<usize, CompileErrors> {
    let _last_output = emit_scoped_flat_body(body, start, common_next, scope, builder, "choose")?;
    body_width(body, 0).map_err(|e| CompileErrors(vec![e]))
}

/// Emit a lexical body and return its last value-producing slot.
pub(in crate::mod_compile_lowering) fn emit_scoped_flat_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
    primitive: &'static str,
) -> Result<Option<SlotIdx>, CompileErrors> {
    body_values::strip_type(emit_scoped_typed_body(
        body,
        start,
        common_next,
        scope,
        builder,
        primitive,
    ))
}

/// Emit a lexical body and return its last value-producing slot and inferred type.
pub(in crate::mod_compile_lowering) fn emit_scoped_typed_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
    primitive: &'static str,
) -> Result<Option<(SlotIdx, ExprType)>, CompileErrors> {
    emit_typed_body(body, start, common_next, scope, builder, primitive, None)
}

pub(super) fn emit_typed_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
    primitive: &'static str,
    tracked: Option<SlotIdx>,
) -> Result<Option<(SlotIdx, ExprType)>, CompileErrors> {
    let depth = builder.body_depth;
    let prospective = depth
        .checked_add(1)
        .ok_or_else(|| index_error(usize::from(depth)))?;
    if prospective > 64 {
        return Err(CompileErrors(vec![
            CompileError::PrimitiveLoweringLimitExceeded {
                primitive,
                field: "body nesting depth",
                value: usize::from(prospective),
                limit: 64,
            },
        ]));
    }
    builder.body_depth = prospective;
    let result =
        emit_typed_body_inner(body, start, common_next, scope, builder, primitive, tracked);
    builder.body_depth = depth;
    result
}

fn emit_typed_body_inner(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
    primitive: &'static str,
    tracked: Option<SlotIdx>,
) -> Result<Option<(SlotIdx, ExprType)>, CompileErrors> {
    let mut local = BranchBindings::new(scope, primitive == "for_each");
    let mut layout =
        canonical_layout(body, start.as_usize()).map_err(|e| CompileErrors(vec![e]))?;
    if tracked.is_some() {
        let mut cursor = start.as_usize();
        for (step, entry) in body.iter().zip(&mut layout) {
            entry.start = step_idx(cursor).map_err(|e| CompileErrors(vec![e]))?;
            entry.width = body_values::checked_sum(
                entry.width,
                usize::from(super::iteration_body::produces_value(step)),
            )
            .ok_or_else(|| index_error(cursor))?;
            cursor =
                body_values::checked_sum(cursor, entry.width).ok_or_else(|| index_error(cursor))?;
        }
    }
    let mut names = vec![Box::from(""); start.as_usize()];
    names.extend(canonical_step_names(body, &layout).map_err(|e| CompileErrors(vec![e]))?);
    let mut result = None;
    let mut offset = 0usize;
    for (index, step) in body.iter().enumerate() {
        reject_body_controls(step, index)?;
        let id = body_id(start, offset, primitive)?;
        offset = body_values::checked_sum(
            offset,
            layout_width(&layout, index).map_err(|e| CompileErrors(vec![e]))?,
        )
        .ok_or_else(|| index_error(offset))?;
        let next_index = body_values::checked_sum(index, 1).ok_or_else(|| index_error(index))?;
        let next = if next_index == body.len() {
            common_next
        } else {
            body_id(start, offset, primitive)?
        };
        let control = super::part_02::canonical_controls::prepare(
            step,
            id,
            Some(next),
            &names,
            local.view(),
            builder,
        )?;
        let success = control.next.ok_or_else(|| index_error(id.as_usize()))?;
        let capture = if tracked.is_some() && super::iteration_body::produces_value(step) {
            let width =
                canonical_step_width_with_condition(step).map_err(|e| CompileErrors(vec![e]))?;
            Some(body_id(id, width, primitive)?)
        } else {
            None
        };
        let candidate = local
            .lower(
                &control.step,
                control.body,
                capture.unwrap_or(success),
                &names,
                builder,
            )?
            .map(|slot| (slot, local.type_of(slot).unwrap_or(ExprType::Unknown)));
        if let (Some(capture), Some(output)) = (capture, tracked) {
            let source = candidate
                .map(|value| value.0)
                .ok_or_else(|| index_error(capture.as_usize()))?;
            builder.push_node(vb_core::workflow::node(
                capture,
                CompiledNodeKind::Copy { source },
                Some(output),
                Some(success),
            ));
        }
        result = body_values::keep_last(result, candidate);
        super::part_02::canonical_controls::complete(&control, builder);
    }
    Ok(tracked.map(|slot| (slot, ExprType::Unknown)).or(result))
}

pub(in crate::mod_compile_lowering) fn emit_body_result(
    id: StepIdx,
    result: Option<SlotIdx>,
    output: SlotIdx,
    next: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let kind = match result {
        Some(source) => CompiledNodeKind::Copy { source },
        None => CompiledNodeKind::SetConst {
            value: builder
                .push_constant(ConstValue::Null)
                .map_err(|e| CompileErrors(vec![e]))?,
        },
    };
    builder.push_node(vb_core::workflow::node(id, kind, Some(output), Some(next)));
    Ok(())
}

fn body_id(
    start: StepIdx,
    index: usize,
    primitive: &'static str,
) -> Result<StepIdx, CompileErrors> {
    let offset = u16::try_from(index).map_err(|_| index_error(index))?;
    checked_step_offset(start, offset, primitive, "body").map_err(|e| CompileErrors(vec![e]))
}

fn index_error(value: usize) -> CompileErrors {
    CompileErrors(vec![CompileError::StepIndexOutOfRange { value }])
}

fn reject_body_controls(step: &crate::StepAst, index: usize) -> Result<(), CompileErrors> {
    let fields = [(
        step.retry.is_some() && !matches!(step.primitive, crate::StepPrimitive::Do { .. }),
        "try_again",
    )];
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

#[cfg(test)]
#[path = "generic_body_tests.rs"]
mod generic_body_tests;
