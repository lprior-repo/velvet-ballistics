//! Reserve source-visible slots before allocating private lowering state.

use super::SlotCompiler;
use crate::{CompileError, ScalarValue, StepAst, StepPrimitive, WorkflowSource};

/// A compiled workflow cannot contain more source steps than numeric node IDs.
const MAX_SOURCE_STEPS: usize = 65_535;

/// Preserves the existing positional source-slot contract without reserving
/// literal values, action IDs, iteration counts, or generated node indices.
///
/// The walk is iterative and bounded by the representable source-step count.
/// It computes the complete reservation before mutating `builder`, so a late
/// invalid reference cannot leave a partial reservation behind.
pub(super) fn reserve_source_slots(
    source: &WorkflowSource,
    builder: &mut SlotCompiler,
) -> Result<(), CompileError> {
    let mut count = builder.slot_count()?;
    let mut pending = Vec::new();
    push_steps(&mut pending, source.steps())?;
    for (index, step) in source.steps().iter().enumerate() {
        if matches!(
            step.primitive,
            StepPrimitive::Set { .. } | StepPrimitive::Do { .. }
        ) {
            let slot = u16::try_from(index)
                .map_err(|_| CompileError::SlotIndexOutOfRange { value: i64::MAX })?;
            reserve_index(&mut count, slot)?;
        }
    }
    for _ in 0..MAX_SOURCE_STEPS {
        let Some(step) = pending.pop() else {
            return builder.reserve_slots(count);
        };
        reserve_primitive(&step.primitive, &mut count, &mut pending)?;
    }
    if pending.is_empty() {
        builder.reserve_slots(count)
    } else {
        Err(source_step_limit())
    }
}

fn reserve_primitive<'a>(
    primitive: &'a StepPrimitive,
    count: &mut u16,
    pending: &mut Vec<&'a StepAst>,
) -> Result<(), CompileError> {
    match primitive {
        StepPrimitive::Do { input, .. } => reserve_numeric(count, input),
        StepPrimitive::ForEach { input, body, .. }
        | StepPrimitive::Aggregate { input, body, .. } => {
            reserve_numeric(count, input)?;
            push_steps(pending, body)
        }
        StepPrimitive::Collect { source, body, .. } => {
            reserve_numeric(count, source)?;
            push_steps(pending, body)
        }
        StepPrimitive::Repeat { body, .. } => push_steps(pending, body),
        StepPrimitive::Together { branches } => {
            for branch in branches {
                push_steps(pending, &branch.steps)?;
            }
            Ok(())
        }
        StepPrimitive::Choose { branches, .. } => {
            for branch in branches {
                reserve_numeric(count, &branch.when)?;
                push_steps(pending, &branch.steps)?;
            }
            Ok(())
        }
        _ => reserve_leaf(primitive, count),
    }
}

fn reserve_leaf(primitive: &StepPrimitive, count: &mut u16) -> Result<(), CompileError> {
    match primitive {
        StepPrimitive::Wait { event, timeout } => {
            reserve_optional(count, event.as_deref())?;
            reserve_optional(count, timeout.as_deref())
        }
        StepPrimitive::Ask { prompt, timeout } => {
            reserve_numeric(count, prompt)?;
            reserve_optional(count, timeout.as_deref())
        }
        StepPrimitive::Finish {
            result: ScalarValue::Integer(value),
        } => reserve_integer(count, *value),
        _ => Ok(()),
    }
}

fn reserve_optional(count: &mut u16, text: Option<&str>) -> Result<(), CompileError> {
    if let Some(text) = text {
        reserve_numeric(count, text)?;
    }
    Ok(())
}

fn reserve_numeric(count: &mut u16, text: &str) -> Result<(), CompileError> {
    // Non-numeric references retain their existing lowering/validation path.
    // In particular, a quoted finish name and a literal Set value are never
    // silently reclassified as runtime slot indices by this reservation pass.
    if let Ok(value) = text.parse::<i64>() {
        reserve_integer(count, value)?;
    }
    Ok(())
}

fn reserve_integer(count: &mut u16, value: i64) -> Result<(), CompileError> {
    let index = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    reserve_index(count, index)
}

fn reserve_index(count: &mut u16, index: u16) -> Result<(), CompileError> {
    let required = index
        .checked_add(1)
        .ok_or(CompileError::SlotIndexOutOfRange {
            value: i64::from(index),
        })?;
    *count = (*count).max(required);
    Ok(())
}

fn push_steps<'a>(
    pending: &mut Vec<&'a StepAst>,
    steps: &'a [StepAst],
) -> Result<(), CompileError> {
    let count = pending
        .len()
        .checked_add(steps.len())
        .ok_or_else(source_step_limit)?;
    if count > MAX_SOURCE_STEPS {
        return Err(source_step_limit());
    }
    pending
        .try_reserve(steps.len())
        .map_err(|_| source_step_limit())?;
    pending.extend(steps.iter().rev());
    Ok(())
}

fn source_step_limit() -> CompileError {
    CompileError::NodeLimit {
        limit: u32::from(u16::MAX),
    }
}

#[cfg(test)]
#[path = "source_slot_reservation_tests.rs"]
mod tests;
