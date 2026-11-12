use super::CanonicalStepLayout;
use crate::mod_compile_errors::CompileError;
use vb_core::StepIdx;

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
