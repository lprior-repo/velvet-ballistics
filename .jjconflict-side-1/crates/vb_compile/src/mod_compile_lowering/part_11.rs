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

#[allow(dead_code)]
pub(super) fn compile_repeat(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "repeat", &["max_attempts"])?;
    let max_attempts = required_u16_field(body, index, "repeat", "max_attempts")?;
    let body_step = checked_step_offset(id, 1, "repeat", "body")?;
    let done = checked_step_offset(id, 2, "repeat", "done")?;
    let attempt_slot = slot_idx_for_step(id.as_usize().checked_add(1).ok_or({
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "repeat",
            field: "attempt_slot",
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        }
    })?)?;
    builder.record_slot(attempt_slot);
    let mut nodes = lower_repeat(id, max_attempts, body_step, done, &mut SlotCompiler::new())?;
    // RepeatFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

#[allow(dead_code)]
pub(super) fn compile_wait(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["until", "event", "timeout"])?;
    let until = optional_slot_field(body, index, "until")?;
    let event = optional_slot_field(body, index, "event")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    let mut node = match (until, event, timeout) {
        (Some(deadline), None, None) => {
            builder.record_slot(deadline);
            lower_wait(id, WaitKind::Until { deadline }, &mut SlotCompiler::new())
        }
        (None, Some(event_slot), timeout_slot) => {
            builder.record_slot(event_slot);
            if let Some(slot) = timeout_slot {
                builder.record_slot(slot);
            }
            lower_wait(
                id,
                WaitKind::Event {
                    event: event_slot,
                    timeout: timeout_slot,
                },
                &mut SlotCompiler::new(),
            )
        }
        _ => {
            return Err(CompileError::StepFieldShape {
                step: index,
                field: "wait",
                expected: "until without timeout or event with optional timeout",
            });
        }
    };
    node.next = next;
    Ok(node)
}

#[allow(dead_code)]
pub(super) fn compile_ask(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "answer", "timeout"])?;
    let prompt = required_slot(body, index, "prompt")?;
    let answer = required_slot(body, index, "answer")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    builder.record_slot(prompt);
    builder.record_slot(answer);
    if let Some(slot) = timeout {
        builder.record_slot(slot);
    }
    let mut nodes = lower_ask(id, prompt, answer, timeout, &mut SlotCompiler::new())?;
    // Ask (index 0) chains to AskResume for structural reachability.
    if let (Some(_ask_node), Some(resume_node)) = (nodes.first(), nodes.get(1)) {
        let resume_id = resume_node.id;
        if let Some(ask_node) = nodes.first_mut() {
            ask_node.next = Some(resume_id);
        }
    }
    // AskResume (index 1) chains to the next step.
    if let Some(resume) = nodes.get_mut(1) {
        resume.next = next;
    }
    Ok(nodes)
}

#[allow(dead_code)]
pub(super) fn compile_finish(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    if index != last_step {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        });
    }
    reject_unknown_primitive_fields(body, index, "finish", &["result"])?;
    let result = required_step_field(body, index, "result")?;
    compile_finish_result(result, index, id, builder)
}

#[allow(dead_code)]
pub(super) fn compile_finish_result(
    result: &Yaml<'_>,
    index: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    if let Some(slot) = finish_result_slot(result, index)? {
        builder.record_slot(slot);
        return Ok(vec![CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish { result: slot },
        }]);
    }
    let value = slot_value(result, index)?;
    let value = builder.push_constant(value)?;
    let output = slot_idx_for_step(index)?;
    builder.record_slot(output);
    let finish_id = id.checked_add(1).ok_or(CompileError::StepIndexOutOfRange {
        value: id.as_usize(),
    })?;
    Ok(vec![
        CompiledNode {
            id,
            output: Some(output),
            next: Some(finish_id),
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::SetConst { value },
        },
        CompiledNode {
            id: finish_id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish { result: output },
        },
    ])
}

#[allow(dead_code)]
pub(super) fn finish_result_slot(
    result: &Yaml<'_>,
    index: usize,
) -> Result<Option<SlotIdx>, CompileError> {
    let Some(value) = result.as_integer() else {
        return Ok(None);
    };
    if !finish_integer_is_slot(value, index) {
        return Ok(None);
    }
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(Some(SlotIdx::new(value)))
}

#[allow(dead_code)]
pub(super) fn finish_integer_is_slot(value: i64, index: usize) -> bool {
    match usize::try_from(value) {
        Ok(slot) => slot <= index,
        Err(_) => false,
    }
}

pub(super) fn reject_last_non_finish(index: usize, last_step: usize) -> Result<(), CompileError> {
    if index == last_step {
        Err(CompileError::LastStepMustFinish)
    } else {
        Ok(())
    }
}

pub(super) fn step_idx(value: usize) -> Result<StepIdx, CompileError> {
    let value = u16::try_from(value).map_err(|_| CompileError::StepIndexOutOfRange { value })?;
    Ok(StepIdx::new(value))
}

pub(super) fn slot_idx_for_step(value: usize) -> Result<SlotIdx, CompileError> {
    let value = u16::try_from(value).map_err(|_| CompileError::StepIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}

pub(super) fn required_step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}
