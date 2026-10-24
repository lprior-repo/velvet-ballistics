use super::part_01::ForEachInputs;
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_core::{SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

#[cfg(test)]
pub(crate) use super::part_14::lower_canonical_choose;

#[path = "canonical_scope.rs"]
mod canonical_scope;
pub(super) use canonical_scope::{CanonicalScope, validate_output_alias};

#[path = "canonical_references.rs"]
mod canonical_references;
pub(super) use canonical_references::resolve_slot_reference;

#[path = "canonical_types.rs"]
mod canonical_types;

#[path = "canonical_scalar.rs"]
mod canonical_scalar;
pub(super) use canonical_scalar::{compile_canonical_expression, lower_canonical_set_expression};

#[path = "canonical_do.rs"]
mod canonical_do;

#[path = "canonical_guarded_do.rs"]
mod canonical_guarded_do;
use canonical_guarded_do::lower_top_level_do;

#[path = "bounded_for_each.rs"]
mod bounded_for_each;
use bounded_for_each::{ForEachLowering, lower_bounded_for_each};

#[path = "bounded_collect.rs"]
pub(super) mod bounded_collect;
use bounded_collect::{CollectLowering, lower_bounded_collect};

#[path = "for_each_strict.rs"]
mod for_each_strict;
pub(super) use for_each_strict::validate_for_each_expression;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_canonical_step(
    step: &crate::StepAst,
    index: usize,
    last: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    outputs: &mut HashMap<String, SlotIdx>,
    step_outputs: &mut HashMap<String, SlotIdx>,
    step_names: &mut Vec<Box<str>>,
    slot_types: &mut HashMap<SlotIdx, ExprType>,
    for_each_inputs: &ForEachInputs,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let result = match &step.primitive {
        crate::StepPrimitive::Set { output, value } => {
            validate_output_alias(output).map_err(|e| CompileErrors(vec![e]))?;
            let slot = slot_idx_for_step(index).map_err(|e| CompileErrors(vec![e]))?;
            if outputs.contains_key(output) {
                return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                    name: Box::from(output.as_str()),
                }]));
            }
            let inferred = lower_canonical_set_expression(
                id,
                slot,
                value,
                next,
                CanonicalScope {
                    outputs,
                    step_outputs,
                    slot_types,
                },
                builder,
            )?;
            outputs.insert(output.to_owned(), slot);
            step_outputs.insert(step.id.to_string(), slot);
            builder.record_slot(slot);
            slot_types.insert(slot, inferred);
            Ok(())
        }
        crate::StepPrimitive::Do { .. } => {
            let output_slot = lower_top_level_do(
                step,
                index,
                id,
                next,
                CanonicalScope {
                    outputs,
                    step_outputs,
                    slot_types,
                },
                builder,
            )?;
            if let Some(slot) = output_slot {
                step_outputs.insert(step.id.to_string(), slot);
                // Do action outputs are dynamic; no static type inference
            }
            Ok(())
        }
        crate::StepPrimitive::Finish { result } => {
            lower_canonical_finish(index, last, id, result, outputs, step_outputs, builder)
        }
        crate::StepPrimitive::ForEach {
            input,
            variable,
            at_once,
            body,
        } => lower_bounded_for_each(
            index,
            id,
            ForEachLowering {
                input,
                variable,
                at_once: *at_once,
                body,
                next,
                scope: CanonicalScope {
                    outputs,
                    step_outputs,
                    slot_types,
                },
                inputs: for_each_inputs,
            },
            builder,
        ),
        crate::StepPrimitive::Together { branches } => {
            lower_canonical_parallel(index, id, branches, next, builder)
        }
        crate::StepPrimitive::Collect {
            source,
            variable,
            pages,
            items,
            body,
            ..
        } => {
            let slot = lower_bounded_collect(
                index,
                id,
                CollectLowering {
                    source,
                    variable,
                    pages: *pages,
                    items: *items,
                    body,
                    next,
                    scope: CanonicalScope {
                        outputs,
                        step_outputs,
                        slot_types,
                    },
                },
                builder,
            )?;
            step_outputs.insert(step.id.to_string(), slot);
            slot_types.insert(slot, ExprType::List);
            Ok(())
        }
        crate::StepPrimitive::Aggregate {
            input,
            initial,
            body,
            ..
        } => lower_canonical_aggregate(index, id, input, initial, body, next, builder),
        crate::StepPrimitive::Repeat { max_attempts, body } => lower_canonical_repeat(
            index,
            id,
            *max_attempts,
            body,
            next,
            CanonicalScope {
                outputs,
                step_outputs,
                slot_types,
            },
            builder,
        ),
        crate::StepPrimitive::Wait { event, timeout } => lower_canonical_wait(
            index,
            id,
            event.as_deref(),
            timeout.as_deref(),
            next,
            CanonicalScope {
                outputs,
                step_outputs,
                slot_types,
            },
            builder,
        ),
        crate::StepPrimitive::Ask { prompt, timeout } => lower_canonical_ask(
            index,
            id,
            prompt,
            timeout.as_deref(),
            next,
            CanonicalScope {
                outputs,
                step_outputs,
                slot_types,
            },
            builder,
        ),
        crate::StepPrimitive::Choose {
            branches,
            otherwise,
        } => super::part_14::lower_canonical_choose_in_scope(
            id,
            branches,
            otherwise.as_deref(),
            next,
            super::part_14::ChooseScope {
                index,
                names: step_names.as_ref(),
                bindings: CanonicalScope {
                    outputs,
                    step_outputs,
                    slot_types,
                },
            },
            builder,
        ),
        other => Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: canonical_primitive_name(other),
            },
        ])),
    };

    result?;
    extend_step_names_for_generated(step_names, step.id.as_str(), builder.nodes.len());
    Ok(())
}

pub(super) fn extend_step_names_for_generated(
    names: &mut Vec<Box<str>>,
    step_id: &str,
    node_count: usize,
) {
    while names.len() < node_count {
        names.push(Box::from(step_id));
    }
}

pub(super) fn lower_canonical_finish(
    index: usize,
    last: usize,
    id: StepIdx,
    result: &crate::ScalarValue,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if index != last {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        }]));
    }
    let slot = super::canonical_finish_slot_with_step_outputs(result, outputs, step_outputs)?;
    let node = lower_finish(id, slot, builder);
    builder.push_node(node);
    Ok(())
}
