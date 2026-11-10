//! Bounded Repeat bodies with lexical bindings.

use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_14::{emit_body_result, emit_scoped_typed_body};
use crate::mod_compile_lowering::{CanonicalScope, SlotCompiler, body_width, checked_step_offset};
use vb_core::workflow::{NodeContinuation, NodeOutput, node};
use vb_core::{CompiledNode, CompiledNodeKind, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) fn lower_canonical_repeat(
    index: usize,
    id: StepIdx,
    max_attempts: u16,
    body: &[crate::StepAst],
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, ExprType), CompileErrors> {
    if max_attempts == 0 {
        return Err(field_error(
            index,
            "repeat.max_attempts",
            "a value of at least 1 (got 0)",
        ));
    }
    if body.is_empty() {
        return Err(field_error(index, "steps", "non-empty body"));
    }
    let layout = RepeatLayout::new(id, body).map_err(|e| CompileErrors(vec![e]))?;
    let attempt_slot = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node(
        id,
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body: layout.body,
            done: layout.done,
        },
        Some(attempt_slot),
        None,
    ));
    let last = emit_scoped_typed_body(body, layout.body, layout.result, scope, builder, "repeat")?;
    let result = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    emit_body_result(
        layout.result,
        last.map(|value| value.0),
        result,
        layout.check,
        builder,
    )?;
    push_repeat_continuations(layout, attempt_slot, result, next, builder);
    Ok((result, last.map_or(ExprType::Null, |value| value.1)))
}

struct RepeatLayout {
    body: StepIdx,
    result: StepIdx,
    check: StepIdx,
    done: StepIdx,
}

impl RepeatLayout {
    fn new(id: StepIdx, steps: &[crate::StepAst]) -> Result<Self, CompileError> {
        let width = body_width(steps, 0)?;
        let width =
            u16::try_from(width).map_err(|_| CompileError::StepIndexOutOfRange { value: width })?;
        let body = checked_step_offset(id, 1, "repeat", "body")?;
        let result = checked_step_offset(body, width, "repeat", "result")?;
        let check = checked_step_offset(result, 1, "repeat", "check")?;
        let done = checked_step_offset(check, 1, "repeat", "done")?;
        Ok(Self {
            body,
            result,
            check,
            done,
        })
    }
}

fn push_repeat_continuations(
    layout: RepeatLayout,
    attempt_slot: SlotIdx,
    result: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) {
    builder.push_node(node(
        layout.check,
        CompiledNodeKind::RepeatCheck {
            attempt_slot,
            done: layout.done,
        },
        Some(attempt_slot),
        Some(layout.body),
    ));
    builder.push_node(node(
        layout.done,
        CompiledNodeKind::RepeatFinish { result },
        Some(result),
        next,
    ));
}

fn field_error(step: usize, field: &'static str, expected: &'static str) -> CompileErrors {
    CompileErrors(vec![CompileError::StepFieldShape {
        step,
        field,
        expected,
    }])
}

#[cfg(test)]
#[path = "canonical_repeat_tests.rs"]
mod tests;
