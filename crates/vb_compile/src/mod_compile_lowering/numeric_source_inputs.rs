//! Retain explicit external numeric references in the admission input table.

use std::collections::BTreeSet;
use crate::{CompileError, ScalarValue, StepAst, StepPrimitive, WorkflowSource};
use vb_core::{CompiledInputConstraint, InputSlotKind, SlotIdx};
use super::SlotCompiler;

pub(super) fn record_numeric_inputs(
    source: &WorkflowSource,
    builder: &mut SlotCompiler,
) -> Result<(), CompileError> {
    let mut produced = BTreeSet::new();
    for (index, step) in source.steps().iter().enumerate() {
        let mut pending = vec![step];
        while let Some(step) = pending.pop() {
            visit(step, &produced, builder, &mut pending)?;
        }
        if matches!(step.primitive, StepPrimitive::Set { .. } | StepPrimitive::Do { .. }) {
            let index = u16::try_from(index)
                .map_err(|_| CompileError::StepIndexOutOfRange { value: index })?;
            produced.insert(index);
        }
    }
    Ok(())
}

fn visit<'a>(
    step: &'a StepAst,
    produced: &BTreeSet<u16>,
    builder: &mut SlotCompiler,
    pending: &mut Vec<&'a StepAst>,
) -> Result<(), CompileError> {
    match &step.primitive {
        StepPrimitive::Do { input, .. } => record(input, InputSlotKind::Any, produced, builder)?,
        StepPrimitive::ForEach { input, body, .. }
        | StepPrimitive::Aggregate { input, body, .. } => {
            record(input, InputSlotKind::List, produced, builder)?;
            pending.extend(body.iter().rev());
        }
        StepPrimitive::Collect { source, body, .. } => {
            record(source, InputSlotKind::List, produced, builder)?;
            pending.extend(body.iter().rev());
        }
        StepPrimitive::Repeat { body, .. } => pending.extend(body.iter().rev()),
        StepPrimitive::Choose { branches, .. } => for branch in branches {
            record(&branch.when, InputSlotKind::Bool, produced, builder)?;
            pending.extend(branch.steps.iter().rev());
        },
        StepPrimitive::Together { branches } => for branch in branches {
            pending.extend(branch.steps.iter().rev());
        },
        StepPrimitive::Wait { event, timeout } => {
            for text in [event, timeout].into_iter().flatten() {
                record(text, InputSlotKind::I64, produced, builder)?;
            }
        }
        StepPrimitive::Ask { prompt, timeout } => {
            record(prompt, InputSlotKind::Any, produced, builder)?;
            if let Some(timeout) = timeout { record(timeout, InputSlotKind::I64, produced, builder)?; }
        }
        StepPrimitive::Finish { result: ScalarValue::Integer(value) } => {
            record(&value.to_string(), InputSlotKind::Any, produced, builder)?;
        }
        _ => {}
    }
    Ok(())
}

fn record(
    text: &str,
    kind: InputSlotKind,
    produced: &BTreeSet<u16>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileError> {
    let Ok(raw) = text.parse::<u16>() else { return Ok(()); };
    if produced.contains(&raw) { return Ok(()); }
    let slot = SlotIdx::new(raw);
    if let Some(input) = builder.input_slots.iter_mut().find(|input| input.slot == slot) {
        if input.kind == InputSlotKind::Any { input.kind = kind; }
        if kind != InputSlotKind::Any && input.kind != kind {
            return Err(CompileError::InvalidInputSchema {
                field: "numeric source input", expected: "consistent input value kinds",
            });
        }
    } else {
        builder.record_input_slot(slot, kind, CompiledInputConstraint::required());
    }
    Ok(())
}
