use super::*;

pub(in crate::mod_compile_lowering) fn body_width(
    body: &[crate::StepAst],
    overhead: usize,
) -> Result<usize, CompileError> {
    let mut width = overhead;
    let mut pending = vec![(body, false)];
    while let Some((steps, track)) = pending.pop() {
        for step in steps {
            let extra = usize::from(step.condition.is_some())
                .checked_add(
                    if track && super::part_14::iteration_body::produces_value(step) {
                        1
                    } else {
                        0
                    },
                )
                .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
            width = width
                .checked_add(extra)
                .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
            if let Some(value) = &step.with {
                width = width
                    .checked_add(super::canonical_values::width(value)?)
                    .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
            }
            let fixed = match &step.primitive {
                crate::StepPrimitive::ForEach { input, body, .. } => {
                    let tracked = super::part_14::iteration_body::needs_tracking(body);
                    pending.push((body, tracked));
                    6usize
                        .checked_add(super::list_input::source_width(input))
                        .and_then(|n| n.checked_add(usize::from(tracked)))
                        .ok_or(CompileError::StepIndexOutOfRange { value: width })?
                }
                crate::StepPrimitive::Collect { body, .. } => {
                    pending.push((body, false));
                    3
                }
                crate::StepPrimitive::Repeat { body, .. } => {
                    pending.push((body, false));
                    4
                }
                crate::StepPrimitive::Aggregate { body, .. } => {
                    pending.push((body, false));
                    if reduce_direct_body(body) { 4 } else { 5 }
                }
                crate::StepPrimitive::Choose { branches, .. } => {
                    for branch in branches {
                        pending.push((&branch.steps, false));
                    }
                    1
                }
                crate::StepPrimitive::Together { branches } => {
                    for branch in branches {
                        pending.push((&branch.steps, false));
                    }
                    branches
                        .len()
                        .checked_mul(2)
                        .and_then(|n| n.checked_add(2))
                        .ok_or(CompileError::StepIndexOutOfRange { value: width })?
                }
                crate::StepPrimitive::Ask { .. } => 2,
                _ => 1,
            };
            width = width
                .checked_add(fixed)
                .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
        }
    }
    Ok(width)
}

pub(in crate::mod_compile_lowering) fn choose_width(
    branches: &[crate::ChooseBranch],
) -> Result<usize, CompileError> {
    let mut width = 1usize;
    for branch in branches {
        width = width
            .checked_add(body_width(&branch.steps, 0)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(in crate::mod_compile_lowering) fn together_width(
    branches: &[crate::TogetherBranch],
) -> Result<usize, CompileError> {
    let mut width = 2usize;
    for branch in branches {
        width = width
            .checked_add(body_width(&branch.steps, 2)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}
