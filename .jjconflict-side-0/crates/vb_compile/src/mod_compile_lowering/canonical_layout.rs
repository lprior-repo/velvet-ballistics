//! Checked source-to-IR layout, widths, and diagnostic names.
use super::*;

#[derive(Clone, Copy)]
pub(in crate::mod_compile_lowering) struct CanonicalStepLayout {
    pub(in crate::mod_compile_lowering) start: StepIdx,
    pub(in crate::mod_compile_lowering) width: usize,
}

pub(in crate::mod_compile_lowering) fn canonical_layout(
    steps: &[crate::StepAst],
    prefix: usize,
) -> Result<Vec<CanonicalStepLayout>, CompileError> {
    let mut layout = Vec::with_capacity(steps.len());
    let mut cursor = prefix;
    for step in steps {
        let width = canonical_step_width_with_condition(step)?;
        layout.push(CanonicalStepLayout {
            start: step_idx(cursor)?,
            width,
        });
        cursor = cursor
            .checked_add(width)
            .ok_or(CompileError::StepIndexOutOfRange { value: cursor })?;
    }
    Ok(layout)
}

pub(in crate::mod_compile_lowering) fn canonical_step_width(
    primitive: &crate::StepPrimitive,
) -> Result<usize, CompileError> {
    match primitive {
        crate::StepPrimitive::Set { .. }
        | crate::StepPrimitive::Finish { .. }
        | crate::StepPrimitive::Wait { .. } => Ok(1),
        crate::StepPrimitive::Ask { .. } => Ok(2),
        crate::StepPrimitive::ForEach { input, body, .. } => {
            super::part_14::iteration_body_width(body)?
                .checked_add(
                    6usize
                        .checked_add(super::list_input::source_width(input))
                        .ok_or(CompileError::StepIndexOutOfRange { value: 6 })?,
                )
                .ok_or(CompileError::StepIndexOutOfRange { value: 6 })
        }
        crate::StepPrimitive::Collect { body, .. } => body_width(body, 3),
        crate::StepPrimitive::Repeat { body, .. } => body_width(body, 4),
        crate::StepPrimitive::Aggregate { body, .. } => {
            if !reduce_direct_body(body) {
                body_width(body, 5)
            } else {
                body_width(body, 4)
            }
        }
        crate::StepPrimitive::Together { branches } => together_width(branches),
        crate::StepPrimitive::Choose { branches, .. } => choose_width(branches),
        _ => Ok(1),
    }
}

pub(in crate::mod_compile_lowering) fn reduce_direct_body(body: &[crate::StepAst]) -> bool {
    matches!(body, [step] if matches!(step.primitive, crate::StepPrimitive::Set { .. })
        && step.condition.is_none() && step.then.is_none() && step.on_error.is_none()
        && step.retry.is_none() && step.with.is_none())
}

pub(in crate::mod_compile_lowering) fn canonical_step_width_with_condition(
    step: &crate::StepAst,
) -> Result<usize, CompileError> {
    let mut width = canonical_step_width(&step.primitive)?;
    if let Some(value) = &step.with {
        width = width
            .checked_add(super::canonical_values::width(value)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    if step.condition.is_some() {
        return width
            .checked_add(1)
            .ok_or(CompileError::StepIndexOutOfRange { value: width });
    }
    Ok(width)
}

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

pub(in crate::mod_compile_lowering) fn canonical_step_names(
    steps: &[crate::StepAst],
    layout: &[CanonicalStepLayout],
) -> Result<Vec<Box<str>>, CompileError> {
    let total = layout
        .last()
        .map(|entry| {
            entry.start.as_usize().checked_add(entry.width).ok_or(
                CompileError::StepIndexOutOfRange {
                    value: entry.start.as_usize(),
                },
            )
        })
        .transpose()?
        .unwrap_or(0);
    let mut names = Vec::with_capacity(total);
    for (index, step) in steps.iter().enumerate() {
        let width = layout_width(layout, index)?;
        for _ in 0..width {
            names.push(Box::from(step.id.as_str()));
        }
    }
    Ok(names)
}

pub(in crate::mod_compile_lowering) fn layout_start(
    layout: &[CanonicalStepLayout],
    index: usize,
) -> Result<StepIdx, CompileError> {
    layout
        .get(index)
        .map(|entry| entry.start)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })
}

pub(in crate::mod_compile_lowering) fn layout_width(
    layout: &[CanonicalStepLayout],
    index: usize,
) -> Result<usize, CompileError> {
    layout
        .get(index)
        .map(|entry| entry.width)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })
}

/// Name steps in backward-edge errors by walking the source layout; no
/// compiled parts are cloned to build the diagnostic.
pub(in crate::mod_compile_lowering) fn named_workflow_error(
    error: CompileError,
    steps: &[crate::StepAst],
    layout: &[CanonicalStepLayout],
) -> CompileError {
    let CompileError::Workflow(vb_core::WorkflowError::BackwardEdge { from, to }) = &error else {
        return error;
    };
    CompileError::BackwardEdgeNamed {
        from: from.as_usize(),
        to: to.as_usize(),
        from_name: Box::from(step_id_at(steps, layout, from.as_usize())),
        to_name: Box::from(step_id_at(steps, layout, to.as_usize())),
    }
}

fn step_id_at<'a>(
    steps: &'a [crate::StepAst],
    layout: &[CanonicalStepLayout],
    index: usize,
) -> &'a str {
    let mut cursor = 0_usize;
    for (position, step) in steps.iter().enumerate() {
        let width = layout.get(position).map_or(1, |entry| entry.width);
        let end = cursor.saturating_add(width);
        if index >= cursor && index < end {
            return step.id.as_str();
        }
        cursor = end;
    }
    "?"
}

pub(in crate::mod_compile_lowering) fn next_layout_start(
    layout: &[CanonicalStepLayout],
    index: usize,
) -> Result<Option<StepIdx>, CompileError> {
    let next = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    Ok(layout.get(next).map(|entry| entry.start))
}
