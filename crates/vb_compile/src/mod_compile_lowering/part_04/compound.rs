//! Compound step primitive lowering: aggregate, repeat, wait, ask.
//!
//! These functions lower the canonical compound step primitives that each
//! emit a fixed sequence of compiled nodes at deterministic step offsets.
//!
//! - `lower_canonical_aggregate` (Reduce) — emits ReduceStart, body steps,
//!   ReduceNext, ReduceFinish (4+ nodes).
//! - `lower_canonical_repeat` (Repeat) — emits RepeatStart, body steps via
//!   `emit_single_body_set`, RepeatAttempt, RepeatFinish (3+ nodes).
//! - `lower_canonical_wait` (Wait) — emits WaitUntil or WaitEvent (1 node).
//! - `lower_canonical_ask` (Ask) — emits Ask + AskResume (2 nodes).

#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};
use saphyr::Yaml;
use std::collections::HashMap;
use vb_core::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
    ExprIdx, ExprProgram, ResourceContract, SlotBranch, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowError, WorkflowParts,
};

/// Helpers imported directly because they are `pub(super)` in their source modules
/// and cannot be re-exported through the module tree.
use super::super::part_01::body_width;
use super::super::part_02::lower_canonical_for_each;
use super::super::part_05::parse_i64_field;
use super::super::part_05::slot_from_text;
use super::super::part_05::optional_slot_from_text;
use super::super::part_07::{lower_wait, lower_ask, WaitKind, SlotCompiler};
use super::super::part_12::checked_step_offset;
use super::reduce_chain::emit_reduce_body_steps;

// ── Reduce (aggregate) ──────────────────────────────────────────────────

pub(crate) fn lower_canonical_aggregate(
    index: usize,
    id: StepIdx,
    input: &str,
    initial: &str,
    body: &[vb_yaml::ast::StepAst],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let input = slot_from_text(input, index, "aggregate.input")?;
    let accumulator = SlotIdx::new(1);
    let initial = parse_i64_field(initial, index, "reduce.initial")?;
    let initial = builder
        .push_constant(ConstValue::I64(initial))
        .map_err(|e| CompileErrors(vec![e]))?;
    let body_step =
        checked_step_offset(id, 1, "reduce", "body").map_err(|e| CompileErrors(vec![e]))?;
    let body_total_width = body_width(body, 0).map_err(|e| CompileErrors(vec![e]))?;
    let body_total_width_u16 = u16::try_from(body_total_width)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }]))?;
    let next_step = checked_step_offset(body_step, body_total_width_u16, "reduce", "next")
        .map_err(|e| CompileErrors(vec![e]))?;
    let done =
        checked_step_offset(next_step, 1, "reduce", "done").map_err(|e| CompileErrors(vec![e]))?;
    builder.record_slot(input);
    builder.record_slot(accumulator);
    builder.push_node(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body: body_step,
            done,
        },
    });
    emit_reduce_body_steps(
        body,
        body_step,
        index,
        accumulator,
        Some(next_step),
        builder,
    )?;
    builder.push_node(CompiledNode {
        id: next_step,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceNext {
            iterator_slot: accumulator,
            accumulator,
            body: body_step,
            done,
        },
    });
    builder.push_node(CompiledNode {
        id: done,
        output: None,
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceFinish { accumulator },
    });
    Ok(())
}

// ── Repeat ──────────────────────────────────────────────────────────────

pub(crate) fn lower_canonical_repeat(
    index: usize,
    id: StepIdx,
    max_attempts: u16,
    body: &[vb_yaml::ast::StepAst],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if max_attempts == 0 {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "repeat.max_attempts",
            expected: "non-empty primitive field",
        }]));
    }
    let body_step =
        checked_step_offset(id, 1, "repeat", "body").map_err(|e| CompileErrors(vec![e]))?;
    let attempt =
        checked_step_offset(id, 2, "repeat", "attempt").map_err(|e| CompileErrors(vec![e]))?;
    let done = checked_step_offset(id, 3, "repeat", "done").map_err(|e| CompileErrors(vec![e]))?;
    let attempt_slot = SlotIdx::new(1);
    builder.record_slot(attempt_slot);
    builder.push_node(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatStart {
            max_attempts,
            body: body_step,
            done,
        },
    });
    emit_single_body_set(
        body,
        body_step,
        index,
        SlotIdx::new(1),
        Some(attempt),
        builder,
        false,
    )?;
    builder.push_node(CompiledNode {
        id: attempt,
        output: Some(attempt_slot),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body: body_step,
            done,
        },
    });
    builder.push_node(CompiledNode {
        id: done,
        output: None,
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::RepeatFinish {
            result: attempt_slot,
        },
    });
    Ok(())
}

// ── Wait ────────────────────────────────────────────────────────────────

pub(crate) fn lower_canonical_wait(
    index: usize,
    id: StepIdx,
    event: Option<&str>,
    timeout: Option<&str>,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut node = match (event, timeout) {
        (Some(event_text), timeout_text) => {
            let event = slot_from_text(event_text, index, "wait.event")?;
            let timeout = optional_slot_from_text(timeout_text, index, "wait.timeout")?;
            lower_wait(id, WaitKind::Event { event, timeout }, builder)
        }
        (None, Some(timeout_text)) => {
            let deadline = slot_from_text(timeout_text, index, "wait.timeout")?;
            lower_wait(id, WaitKind::Until { deadline }, builder)
        }
        (None, None) => {
            return Err(CompileErrors(vec![CompileError::StepFieldShape {
                step: index,
                field: "wait",
                expected: "event or timeout",
            }]));
        }
    };
    node.next = next;
    builder.push_node(node);
    Ok(())
}

// ── Ask ─────────────────────────────────────────────────────────────────

pub(crate) fn lower_canonical_ask(
    index: usize,
    id: StepIdx,
    prompt: &str,
    timeout: Option<&str>,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let prompt = slot_from_text(prompt, index, "ask.prompt")?;
    let timeout = optional_slot_from_text(timeout, index, "ask.timeout")?;
    let answer = SlotIdx::new(2);
    let mut nodes =
        lower_ask(id, prompt, answer, timeout, builder).map_err(|e| CompileErrors(vec![e]))?;
    let resume = nodes
        .get(1)
        .map(|node| node.id)
        .ok_or_else(|| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }]))?;
    if let Some(node) = nodes.get_mut(0) {
        node.next = Some(resume);
    }
    if let Some(node) = nodes.get_mut(1) {
        node.next = next;
    }
    for node in nodes {
        builder.push_node(node);
    }
    Ok(())
}
