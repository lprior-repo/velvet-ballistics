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

pub(super) fn lower_canonical_aggregate(
    index: usize,
    id: StepIdx,
    input: &str,
    initial: &str,
    body: &[crate::StepAst],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let input = slot_from_text(input, index, "aggregate.input")?;
    let accumulator = SlotIdx::new(1);
    let initial = parse_i64_field(initial, index, "aggregate.initial")?;
    let initial = builder
        .push_constant(ConstValue::I64(initial))
        .map_err(|e| CompileErrors(vec![e]))?;
    let body_step =
        checked_step_offset(id, 1, "aggregate", "body").map_err(|e| CompileErrors(vec![e]))?;
    let next_step =
        checked_step_offset(id, 2, "aggregate", "next").map_err(|e| CompileErrors(vec![e]))?;
    let done =
        checked_step_offset(id, 3, "aggregate", "done").map_err(|e| CompileErrors(vec![e]))?;
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
    emit_single_body_set(
        body,
        body_step,
        index,
        accumulator,
        Some(next_step),
        builder,
        false,
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

pub(super) fn lower_canonical_repeat(
    index: usize,
    id: StepIdx,
    max_attempts: u16,
    body: &[crate::StepAst],
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

pub(super) fn lower_canonical_wait(
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

pub(super) fn lower_canonical_ask(
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

pub(super) fn emit_single_body_set(
    body: &[crate::StepAst],
    id: StepIdx,
    diagnostic_step: usize,
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
    reuse_first_constant: bool,
) -> Result<(), CompileErrors> {
    if body.len() != 1 {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: "steps",
            expected: "exactly one set step",
        }]));
    }
    let step = body.first().ok_or_else(|| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: "steps",
            expected: "one set step",
        }])
    })?;
    match &step.primitive {
        crate::StepPrimitive::Set { value, .. } => {
            let constant =
                body_constant_index(builder, value, diagnostic_step, reuse_first_constant)?;
            builder.record_slot(slot);
            builder.push_node(lower_set(id, slot, constant, next));
            Ok(())
        }
        crate::StepPrimitive::Do { action, input } => {
            // Parse action as integer (ActionId) - action field contains numeric ID
            let action_value = action.parse::<i64>().map_err(|_| {
                CompileErrors(vec![CompileError::StepFieldShape {
                    step: diagnostic_step,
                    field: "do.action",
                    expected: "integer action id",
                }])
            })?;
            let action_id = u16::try_from(action_value).map_err(|_| {
                CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "do",
                    field: "action",
                    value: integer_error_value(action_value),
                    limit: usize::from(u16::MAX),
                }])
            })?;
            // Parse input as SlotIdx
            let input_value = input.parse::<i64>().map_err(|_| {
                CompileErrors(vec![CompileError::StepFieldShape {
                    step: diagnostic_step,
                    field: "do.input",
                    expected: "integer slot index",
                }])
            })?;
            let input_idx = u16::try_from(input_value).map_err(|_| {
                CompileErrors(vec![CompileError::SlotIndexOutOfRange {
                    value: input_value,
                }])
            })?;
            let input_slot = SlotIdx::new(input_idx);
            builder.record_slot(input_slot);
            // Construct Do node directly to avoid double-borrow of builder
            builder.push_node(CompiledNode {
                id,
                output: None,
                next,
                error_slot: None,
                on_error: None,
                kind: CompiledNodeKind::Do {
                    action: vb_core::ActionId::new(action_id),
                    input: input_slot,
                },
            });
            Ok(())
        }
        other => Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: diagnostic_step,
                primitive: canonical_primitive_name(other),
            },
        ])),
    }
}

pub(super) fn body_constant_index(
    builder: &mut SlotCompiler,
    value: &str,
    step: usize,
    reuse_first_constant: bool,
) -> Result<ConstIdx, CompileErrors> {
    if reuse_first_constant && !builder.constants.is_empty() {
        return Ok(ConstIdx::new(0));
    }
    let constant = parse_i64_field(value, step, "set.value")?;
    builder
        .push_constant(ConstValue::I64(constant))
        .map_err(|e| CompileErrors(vec![e]))
}
