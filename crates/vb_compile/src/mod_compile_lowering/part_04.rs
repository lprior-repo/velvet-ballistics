#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};

#[path = "canonical_suspension_fields.rs"]
mod canonical_suspension_fields;
use canonical_suspension_fields::{resolve_optional_suspension_slot, resolve_suspension_slot};
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
    builder.record_slot(input);
    let initial_input = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let next_item = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let iterator_slot = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let accumulator = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let initial = parse_i64_field(initial, index, "aggregate.initial")?;
    let initial = builder
        .push_constant(ConstValue::I64(initial))
        .map_err(|e| CompileErrors(vec![e]))?;
    let start =
        checked_step_offset(id, 1, "aggregate", "start").map_err(|e| CompileErrors(vec![e]))?;
    let body_step =
        checked_step_offset(id, 2, "aggregate", "body").map_err(|e| CompileErrors(vec![e]))?;
    let next_step =
        checked_step_offset(id, 3, "aggregate", "next").map_err(|e| CompileErrors(vec![e]))?;
    let done =
        checked_step_offset(id, 4, "aggregate", "done").map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(CompiledNode {
        id,
        output: Some(initial_input),
        next: Some(start),
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Copy { source: input },
    });
    builder.push_node(CompiledNode {
        id: start,
        output: Some(iterator_slot),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceStart {
            input: initial_input,
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
        output: Some(next_item),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body: body_step,
            done,
        },
    });
    builder.push_node(CompiledNode {
        id: done,
        output: Some(accumulator),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ReduceFinish { accumulator },
    });
    Ok(())
}

#[path = "canonical_repeat.rs"]
mod canonical_repeat;
pub(super) use canonical_repeat::lower_canonical_repeat;

pub(super) fn lower_canonical_wait(
    index: usize,
    id: StepIdx,
    event: Option<&str>,
    timeout: Option<&str>,
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut node = match (event, timeout) {
        (Some(event_text), timeout_text) => {
            let event = resolve_suspension_slot(event_text, index, "wait.event", scope)?;
            let timeout =
                resolve_optional_suspension_slot(timeout_text, index, "wait.timeout", scope)?;
            lower_wait(id, WaitKind::Event { event, timeout }, builder)
        }
        (None, Some(timeout_text)) => {
            let deadline = resolve_suspension_slot(timeout_text, index, "wait.timeout", scope)?;
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
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let prompt = resolve_suspension_slot(prompt, index, "ask.prompt", scope)?;
    let timeout = resolve_optional_suspension_slot(timeout, index, "ask.timeout", scope)?;
    builder.record_slot(prompt);
    if let Some(timeout_slot) = timeout {
        builder.record_slot(timeout_slot);
    }
    let answer = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
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
            builder.record_slot(slot);
            // Construct Do node directly to avoid double-borrow of builder
            builder.push_node(CompiledNode {
                id,
                output: Some(slot),
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

#[path = "body_constants.rs"]
mod body_constants;
pub(super) use body_constants::body_constant_index;
