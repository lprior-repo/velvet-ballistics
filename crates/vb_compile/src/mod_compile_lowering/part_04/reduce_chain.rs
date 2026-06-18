//! Reduce body chain emitter.
//!
//! `emit_reduce_body_steps` emits a chain of body steps for Reduce with
//! proper linking between consecutive steps. It is the multi-step
//! equivalent of `emit_single_body_set`, which only handles single-step
//! bodies.

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
use super::super::part_01::{body_width, canonical_body_step_width};
use super::super::part_02::lower_canonical_for_each;
use super::super::part_05::{
    canonical_primitive_name, lower_set, optional_slot_from_text, parse_i64_field, slot_from_text,
};
use super::super::part_07::{SlotCompiler, WaitKind, lower_ask, lower_wait};
use super::super::part_12::{checked_step_offset, integer_error_value};
use super::body_dispatch::body_constant_index;
use super::compound::lower_canonical_aggregate;

/// Emits a chain of body steps for a Reduce primitive.
///
/// Unlike `emit_single_body_set` which handles exactly one step, this
/// function handles zero or more steps and chains them with next-links.
///
/// # Specification
/// - `body` must be non-empty, otherwise `Err(StepFieldShape)`
/// - Each step gets a sequential StepIdx starting from `body_step`
/// - Steps are linked: step[i].next = step[i+1], last step's next = `next` parameter
/// - Supported primitives: Set, Do, ForEach, Reduce
/// - Unsupported primitives return `Err(UnsupportedStepPrimitive)`
///
/// # Offset Arithmetic
/// Each step's StepIdx is computed as `body_step + cumulative_offset` where
/// the cumulative offset advances by the step's canonical width.
pub(crate) fn emit_reduce_body_steps(
    body: &[vb_yaml::ast::StepAst],
    body_step: StepIdx,
    diagnostic_step: usize,
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if body.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: "steps",
            expected: "at least one body step",
        }]));
    }
    let step_count = body.len();
    let mut cumulative_offset = 0u16;
    for (i, step) in body.iter().enumerate() {
        let step_id = checked_step_offset(body_step, cumulative_offset, "reduce", "body")
            .map_err(|e| CompileErrors(vec![e]))?;
        let step_width =
            canonical_body_step_width(&step.primitive).map_err(|e| CompileErrors(vec![e]))?;
        let step_width_u16 = u16::try_from(step_width).map_err(|_| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: diagnostic_step,
            }])
        })?;
        let next_index = i.checked_add(1).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: diagnostic_step,
            }])
        })?;
        let step_next = if next_index < step_count {
            let next_offset = cumulative_offset
                .checked_add(step_width_u16)
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::StepIndexOutOfRange {
                        value: diagnostic_step,
                    }])
                })?;
            Some(
                checked_step_offset(body_step, next_offset, "reduce", "body")
                    .map_err(|e| CompileErrors(vec![e]))?,
            )
        } else {
            next
        };
        match &step.primitive {
            vb_yaml::ast::StepPrimitive::Set { value, .. } => {
                let constant = body_constant_index(builder, value, diagnostic_step, false)?;
                builder.record_slot(slot);
                builder.push_node(lower_set(step_id, slot, constant, step_next));
            }
            vb_yaml::ast::StepPrimitive::Do { action, input } => {
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
                builder.push_node(CompiledNode {
                    id: step_id,
                    output: None,
                    next: step_next,
                    error_slot: None,
                    on_error: None,
                    kind: CompiledNodeKind::Do {
                        action: vb_core::ActionId::new(action_id),
                        input: input_slot,
                    },
                });
            }
            vb_yaml::ast::StepPrimitive::ForEach {
                input,
                at_once,
                body: foreach_body,
                ..
            } => lower_canonical_for_each(
                diagnostic_step,
                step_id,
                input,
                *at_once,
                foreach_body,
                builder,
            )?,
            vb_yaml::ast::StepPrimitive::Reduce {
                input,
                initial,
                body: reduce_inner_body,
                ..
            } => lower_canonical_aggregate(
                diagnostic_step,
                step_id,
                input,
                initial,
                reduce_inner_body,
                step_next,
                builder,
            )?,
            other => {
                return Err(CompileErrors(vec![
                    CompileError::UnsupportedStepPrimitive {
                        step: diagnostic_step,
                        primitive: canonical_primitive_name(other),
                    },
                ]));
            }
        }
        cumulative_offset = cumulative_offset
            .checked_add(step_width_u16)
            .ok_or_else(|| {
                CompileErrors(vec![CompileError::StepIndexOutOfRange {
                    value: diagnostic_step,
                }])
            })?;
    }
    Ok(())
}
