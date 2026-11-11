//! Bounded Repeat bodies with lexical bindings and nested composition.
//!
//! A `repeat` body may contain `set` / `do` leaves as well as nested
//! `for_each` / `collect` / `repeat` / `choose` containers, lowered
//! recursively. The body evaluates to its last step's slot, which the finish
//! node forwards as the repeat value.

use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_01::ForEachInputs;
use crate::mod_compile_lowering::part_14::{BodyCtx, emit_scoped_flat_body};
use crate::mod_compile_lowering::{CanonicalScope, SlotCompiler, body_width, checked_step_offset};
use vb_core::workflow::{NodeContinuation, NodeOutput, node};
use vb_core::{CompiledNode, CompiledNodeKind, SlotIdx, StepIdx};

pub(in crate::mod_compile_lowering) struct RepeatLowering<'a> {
    pub(in crate::mod_compile_lowering) max_attempts: u16,
    pub(in crate::mod_compile_lowering) body: &'a [crate::StepAst],
    pub(in crate::mod_compile_lowering) next: Option<StepIdx>,
    pub(in crate::mod_compile_lowering) scope: CanonicalScope<'a>,
    pub(in crate::mod_compile_lowering) inputs: &'a ForEachInputs,
    pub(in crate::mod_compile_lowering) step_names: &'a [Box<str>],
}

pub(in crate::mod_compile_lowering) fn lower_canonical_repeat(
    index: usize,
    id: StepIdx,
    config: RepeatLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<SlotIdx, CompileErrors> {
    if config.max_attempts == 0 {
        return Err(field_error(
            index,
            "repeat.max_attempts",
            "a value of at least 1 (got 0)",
        ));
    }
    if config.body.is_empty() {
        return Err(field_error(index, "steps", "at least one set or do step"));
    }
    let layout = RepeatLayout::new(id, config.body).map_err(|e| CompileErrors(vec![e]))?;
    let attempt_slot = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node(
        id,
        CompiledNodeKind::RepeatStart {
            max_attempts: config.max_attempts.into(),
            body: layout.body,
            done: layout.done,
        },
        Some(attempt_slot),
        None,
    ));
    let ctx = BodyCtx {
        inputs: config.inputs,
        step_names: config.step_names,
        index,
    };
    let (result, _) = emit_scoped_flat_body(
        config.body,
        layout.body,
        layout.check,
        config.scope,
        ctx,
        builder,
        "repeat",
        None,
    )?;
    let result =
        result.ok_or_else(|| field_error(index, "steps", "at least one set or do step"))?;
    push_repeat_continuations(layout, attempt_slot, result, config.next, builder);
    Ok(result)
}

struct RepeatLayout {
    body: StepIdx,
    check: StepIdx,
    done: StepIdx,
}

impl RepeatLayout {
    fn new(id: StepIdx, steps: &[crate::StepAst]) -> Result<Self, CompileError> {
        // The same checked recursive width function is used by the outer
        // layout, so nested containers reserve exactly the nodes emission
        // produces.
        let width = body_width(steps, 0)?;
        let width =
            u16::try_from(width).map_err(|_| CompileError::StepIndexOutOfRange { value: width })?;
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

#[cfg(test)]
#[path = "canonical_repeat_edge_tests.rs"]
mod edge_tests;

#[cfg(test)]
#[path = "canonical_repeat_lexical_tests.rs"]
mod lexical_tests;
