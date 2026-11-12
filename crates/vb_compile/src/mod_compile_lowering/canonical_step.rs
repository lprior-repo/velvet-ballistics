//! Canonical dispatch for one complete authored step.
use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::mod_compile_lowering) fn lower_canonical_step(
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
    parent: Option<&CanonicalScope<'_>>,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    let control = canonical_controls::prepare(
        step,
        id,
        next,
        step_names,
        CanonicalScope {
            outputs,
            step_outputs,
            slot_types,
            parent,
        },
        builder,
    )?;
    let step = &control.step;
    let id = control.body;
    let next = control.next;
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
                    parent,
                },
                builder,
            )?;
            outputs.insert(output.to_owned(), slot);
            step_outputs.insert(step.id.to_string(), slot);
            builder.record_slot(slot);
            slot_types.insert(slot, inferred);
            Ok(Some(slot))
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
                    parent,
                },
                builder,
            )?;
            if let Some(slot) = output_slot {
                step_outputs.insert(step.id.to_string(), slot);
                // Do action outputs are dynamic; no static type inference
            }
            Ok(output_slot)
        }
        crate::StepPrimitive::Finish { result } => canonical_finish::lower_canonical_finish_scoped(
            index,
            last,
            id,
            result,
            CanonicalScope {
                outputs,
                step_outputs,
                slot_types,
                parent,
            },
            builder,
        )
        .map(|()| None),
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
                    parent,
                },
                inputs: for_each_inputs,
            },
            builder,
        )
        .map(|slot| {
            step_outputs.insert(step.id.to_string(), slot);
            slot_types.insert(slot, ExprType::List);
            Some(slot)
        }),
        crate::StepPrimitive::Together { branches } => {
            lower_canonical_parallel(
                index,
                id,
                branches,
                next,
                CanonicalScope {
                    outputs,
                    step_outputs,
                    slot_types,
                    parent,
                },
                builder,
            )?;
            let output = builder.nodes.last().and_then(|node| node.output.slot());
            if let Some(slot) = output {
                step_outputs.insert(step.id.clone(), slot);
                slot_types.insert(slot, ExprType::List);
            }
            Ok(output)
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
                        parent,
                    },
                },
                builder,
            )?;
            step_outputs.insert(step.id.to_string(), slot);
            slot_types.insert(slot, ExprType::List);
            Ok(Some(slot))
        }
        crate::StepPrimitive::Aggregate {
            input,
            variable,
            initial,
            body,
        } => {
            let (slot, inferred) = lower_canonical_aggregate(
                index,
                id,
                ReduceLowering {
                    input,
                    variable,
                    initial,
                    body,
                    next,
                    scope: CanonicalScope {
                        outputs,
                        step_outputs,
                        slot_types,
                        parent,
                    },
                },
                builder,
            )?;
            step_outputs.insert(step.id.to_string(), slot);
            slot_types.insert(slot, inferred);
            Ok(Some(slot))
        }
        crate::StepPrimitive::Repeat { max_attempts, body } => {
            let (result, inferred) = lower_canonical_repeat(
                index,
                id,
                *max_attempts,
                body,
                next,
                CanonicalScope {
                    outputs,
                    step_outputs,
                    slot_types,
                    parent,
                },
                builder,
            )?;
            step_outputs.insert(step.id.clone(), result);
            slot_types.insert(result, inferred);
            Ok(Some(result))
        }
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
                parent,
            },
            builder,
        )
        .map(|()| None),
        crate::StepPrimitive::Ask { prompt, timeout } => {
            let answer = lower_canonical_ask(
                index,
                id,
                prompt,
                timeout.as_deref(),
                next,
                CanonicalScope {
                    outputs,
                    step_outputs,
                    slot_types,
                    parent,
                },
                builder,
            )?;
            step_outputs.insert(step.id.clone(), answer);
            slot_types.insert(answer, ExprType::Unknown);
            Ok(Some(answer))
        }
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
                    parent,
                },
            },
            builder,
        )
        .map(|()| None),
        other => Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: canonical_primitive_name(other),
            },
        ])),
    };

    let output = result?;
    canonical_controls::complete(&control, builder);
    extend_step_names_for_generated(step_names, step.id.as_str(), builder.nodes.len());
    Ok(output)
}
