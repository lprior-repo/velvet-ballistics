#![forbid(unsafe_code)]
//! Step compilation entry point and primitive dispatch.
//!
//! Converts parsed YAML workflow steps into compiled node arrays.

use saphyr::Yaml;
use vb_core::{CompiledNode, CompiledNodeKind, ConstIdx, ConstValue, SlotBranch, SlotIdx, StepIdx,
               WorkflowParts};

mod compile_builder;
mod compile_step_helpers;
mod compile_step_primitives;

pub use compile_builder::WorkflowBuilder;
pub use compile_step_helpers::{
    alloc_workflow_slot, checked_step_offset, mapped_branch_target,
    non_string_key_error, optional_slot_field, reject_last_non_finish,
    reject_non_mapping_step_body, reject_unknown_primitive_fields,
    reject_unsupported_for_each_fields, required_action, required_branch_target,
    required_branch_targets, required_next_step, required_slot, required_step_field,
    required_u16_field, required_u32_field, slot_idx_for_step, slot_value,
    source_ir_start,
};
pub use compile_step_primitives::{ChooseCondition, StepPrimitive, StepSpec, is_reserved_name,
                                  step_spec};

use super::slot_compiler::SlotCompiler;
use super::lower::{
    lower_ask, lower_collect, lower_do, lower_finish, lower_for_each, lower_reduce,
    lower_repeat, lower_set, lower_together, lower_wait,
};

/// Compiles a single YAML workflow step into compiled node(s).
pub fn compile_step(
    step: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    let node = match primitive {
        StepPrimitive::Run | StepPrimitive::Do => compile_run(
            body,
            index,
            last_step,
            id,
            next,
            primitive.as_str(),
            builder,
        )?,
        StepPrimitive::Set | StepPrimitive::Save => compile_save(
            body,
            index,
            last_step,
            id,
            next,
            primitive.as_str(),
            builder,
        )?,
        StepPrimitive::Choose => {
            compile_choose(body, index, last_step, id, source_ir_starts, builder)?
        }
        StepPrimitive::ForEach => return compile_for_each(body, index, last_step, id, builder),
        StepPrimitive::Together => {
            return compile_together(body, index, last_step, id, source_ir_starts, builder);
        }
        StepPrimitive::Collect => {
            return compile_collect(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Reduce => {
            return compile_reduce(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Repeat => {
            return compile_repeat(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Wait => compile_wait(body, index, last_step, id, next, builder)?,
        StepPrimitive::Ask => return compile_ask(body, index, last_step, id, next, builder),
        StepPrimitive::Finish => return compile_finish(body, index, last_step, id, builder),
    };
    Ok(vec![node])
}

// ============================================================================
// Run and Save compilation
// ============================================================================

fn compile_run(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
    primitive: &'static str,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    let action = required_action(body, index, primitive)?;
    let input = required_slot(body, index, "input")?;
    let output = slot_idx_for_step(index)?;
    builder.record_slot(input);
    builder.record_slot(output);
    Ok(lower_do(
        id,
        action,
        input,
        Some(output),
        Some(required_next_step(next, index)?),
        &mut SlotCompiler::new(),
    ))
}

fn compile_save(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
    primitive: &'static str,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_non_mapping_step_body(body, index, primitive, "an object")?;
    let output = slot_idx_for_step(index)?;
    let constant = save_slot_value(body, index, primitive)?;
    let constant = builder.push_constant(constant)?;
    builder.record_slot(output);
    set_const_node(id, output, constant, required_next_step(next, index)?)
}

#[allow(clippy::unnecessary_wraps)]
fn set_const_node(
    id: StepIdx,
    output: SlotIdx,
    value: ConstIdx,
    next: StepIdx,
    on_error: None,
    error_slot: None,
) -> Result<CompiledNode, CompileError> {
    Ok(CompiledNode {
        id,
        output: Some(output),
        next: Some(next),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst { value },
    })
}

fn save_slot_value(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<ConstValue, CompileError> {
    let Some(mapping) = body.as_mapping() else {
        return Err(CompileError::StepFieldShape {
            step,
            field: primitive,
            expected: "an object",
        });
    };
    if mapping.len() != 1 {
        return Err(CompileError::UnsupportedConstantValue { step });
    }
    match mapping.iter().next() {
        Some((key, value)) if key.as_str() == Some("value") => slot_value(value, step),
        Some((key, _)) if key.as_str().is_none() => Err(non_string_key_error()),
        Some(_) | None => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

// ============================================================================
// Choose compilation
// ============================================================================

fn compile_choose(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "choose", &["condition", "on_true", "on_false"])?;
    let condition = required_choose_condition(body, index)?;
    let on_true = mapped_branch_target(body, index, "on_true", source_ir_starts)?;
    let on_false = mapped_branch_target(body, index, "on_false", source_ir_starts)?;
    match condition {
        ChooseCondition::Slot(condition) => {
            compile_slot_choose(id, condition, on_true, on_false, builder)
        }
        ChooseCondition::Literal(value) => {
            compile_literal_choose(index, id, value, on_true, on_false, builder)
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn compile_slot_choose(
    id: StepIdx,
    condition: SlotIdx,
    on_true: StepIdx,
    on_false: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    builder.record_slot(condition);
    Ok(CompiledNode {
        id,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches: vec![SlotBranch {
                condition,
                target: on_true,
            }]
            .into_boxed_slice(),
            otherwise: Some(on_false),
        },
    })
}

fn compile_literal_choose(
    index: usize,
    id: StepIdx,
    value: bool,
    on_true: StepIdx,
    on_false: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    let output = slot_idx_for_step(index)?;
    let constant = builder.push_constant(ConstValue::Bool(value))?;
    builder.record_slot(output);
    Ok(CompiledNode {
        id,
        output: Some(output),
        next: Some(if value { on_true } else { on_false }),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst { value: constant },
    })
}

// ============================================================================
// Iterator compilation (for_each, together, collect, reduce, repeat)
// ============================================================================

fn compile_for_each(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unsupported_for_each_fields(body, index)?;
    reject_unknown_primitive_fields(body, index, "for_each", &["input", "item", "limit"])?;
    let input = required_slot(body, index, "input")?;
    let item = required_slot(body, index, "item")?;
    let limit = required_u32_field(body, index, "for_each", "limit")?;
    let body_step = checked_step_offset(id, 1, "for_each", "body")?;
    let done = checked_step_offset(id, 2, "for_each", "done")?;
    builder.record_slot(input);
    builder.record_slot(item);
    lower_for_each(
        id,
        input,
        item,
        limit,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )
}

fn compile_together(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "together", &["branches"])?;
    let branch_sources = required_branch_targets(body, index, "branches")?;
    let mut branches = Vec::with_capacity(branch_sources.len());
    for source in branch_sources {
        branches.push(source_ir_start(source_ir_starts, source.as_usize())?);
    }
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "together",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }
    })?;
    let accumulator = alloc_workflow_slot(builder)?;
    let join = checked_step_offset(id, 1, "together", "join")?;
    Ok(vec![
        CompiledNode {
            id,
            output: Some(accumulator),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: branches.into_boxed_slice(),
                join,
            },
        },
        CompiledNode {
            id: join,
            output: Some(accumulator),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count,
                accumulator,
            },
        },
    ])
}

fn compile_collect(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "collect", &["source", "limit", "page_size"])?;
    let source = required_slot(body, index, "source")?;
    let limit = required_u32_field(body, index, "collect", "limit")?;
    let page_size = required_u32_field(body, index, "collect", "page_size")?;
    let body_step = checked_step_offset(id, 1, "collect", "body")?;
    let done = checked_step_offset(id, 2, "collect", "done")?;
    builder.record_slot(source);
    let mut nodes = lower_collect(
        id,
        source,
        limit,
        page_size,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )?;
    // CollectFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

fn compile_reduce(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "reduce", &["input", "accumulator", "initial"])?;
    let input = required_slot(body, index, "input")?;
    let accumulator = required_slot(body, index, "accumulator")?;
    let initial = slot_value(required_step_field(body, index, "initial")?, index)?;
    let initial = builder.push_constant(initial)?;
    let body_step = checked_step_offset(id, 1, "reduce", "body")?;
    let done = checked_step_offset(id, 2, "reduce", "done")?;
    builder.record_slot(input);
    builder.record_slot(accumulator);
    let mut nodes = lower_reduce(
        id,
        input,
        accumulator,
        initial,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )?;
    // ReduceFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

fn compile_repeat(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
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

// ============================================================================
// Wait compilation
// ============================================================================

fn compile_wait(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
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
            lower_wait(id, deadline, None, false, &mut SlotCompiler::new())
        }
        (None, Some(event_slot), timeout_slot) => {
            builder.record_slot(event_slot);
            if let Some(slot) = timeout_slot {
                builder.record_slot(slot);
            }
            lower_wait(id, event_slot, timeout_slot, true, &mut SlotCompiler::new())
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

// ============================================================================
// Ask compilation
// ============================================================================

fn compile_ask(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
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

// ============================================================================
// Finish compilation
// ============================================================================

fn compile_finish(
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

fn compile_finish_result(
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
            on_error: None,
            error_slot: None,
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
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst { value },
        },
        CompiledNode {
            id: finish_id,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish { result: output },
        },
    ])
}

fn finish_result_slot(result: &Yaml<'_>, index: usize) -> Result<Option<SlotIdx>, CompileError> {
    let Some(value) = result.as_integer() else {
        return Ok(None);
    };
    if !finish_integer_is_slot(value, index) {
        return Ok(None);
    }
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(Some(SlotIdx::new(value)))
}

fn finish_integer_is_slot(value: i64, index: usize) -> bool {
    match usize::try_from(value) {
        Ok(slot) => slot <= index,
        Err(_) => false,
    }
}
