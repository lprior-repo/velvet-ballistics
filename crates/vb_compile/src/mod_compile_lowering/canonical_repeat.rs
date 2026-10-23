//! Bounded flat Repeat bodies with lexical Set/Do bindings.

use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_14::emit_scoped_flat_body;
use crate::mod_compile_lowering::{CanonicalScope, SlotCompiler, body_width, checked_step_offset};
use vb_core::{CompiledNode, CompiledNodeKind, SlotIdx, StepIdx};

pub(in crate::mod_compile_lowering) fn lower_canonical_repeat(
    index: usize,
    id: StepIdx,
    max_attempts: u16,
    body: &[crate::StepAst],
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if max_attempts == 0 {
        return Err(field_error(index, "repeat.max_attempts", "non-empty primitive field"));
    }
    if body.is_empty() {
        return Err(field_error(index, "steps", "at least one set or do step"));
    }
    let layout = RepeatLayout::new(id, body).map_err(|e| CompileErrors(vec![e]))?;
    let attempt_slot = builder.allocate_slot().map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(CompiledNode {
        id,
        output: Some(attempt_slot),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatStart {
            max_attempts,
            body: layout.body,
            done: layout.done,
        },
    });
    let result = emit_scoped_flat_body(body, layout.body, layout.check, scope, builder, "repeat")?
        .ok_or_else(|| field_error(index, "steps", "at least one set or do step"))?;
    push_repeat_continuations(layout, attempt_slot, result, next, builder);
    Ok(())
}

struct RepeatLayout {
    body: StepIdx,
    check: StepIdx,
    done: StepIdx,
}

impl RepeatLayout {
    fn new(id: StepIdx, steps: &[crate::StepAst]) -> Result<Self, CompileError> {
        // The same checked flat width function is used by the outer layout.
        // Nested primitives remain explicitly unsupported rather than gaining
        // activation semantics merely because their source body has a length.
        let width = body_width(steps, 0)?;
        let width = u16::try_from(width)
            .map_err(|_| CompileError::StepIndexOutOfRange { value: width })?;
        let body = checked_step_offset(id, 1, "repeat", "body")?;
        let check = checked_step_offset(body, width, "repeat", "check")?;
        let done = checked_step_offset(check, 1, "repeat", "done")?;
        Ok(Self { body, check, done })
    }
}

fn push_repeat_continuations(
    layout: RepeatLayout,
    attempt_slot: SlotIdx,
    result: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) {
    builder.push_node(CompiledNode {
        id: layout.check,
        output: Some(attempt_slot),
        next: Some(layout.body),
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatCheck {
            attempt_slot,
            done: layout.done,
        },
    });
    builder.push_node(CompiledNode {
        id: layout.done,
        output: Some(result),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatFinish { result },
    });
}

fn field_error(step: usize, field: &'static str, expected: &'static str) -> CompileErrors {
    CompileErrors(vec![CompileError::StepFieldShape { step, field, expected }])
}

#[cfg(test)]
#[path = "canonical_repeat_tests.rs"]
mod tests;
