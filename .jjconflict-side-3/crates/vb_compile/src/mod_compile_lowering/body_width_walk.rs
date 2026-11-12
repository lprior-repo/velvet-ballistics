#![forbid(unsafe_code)]

use super::{CompileError, canonical_primitive_name};
use crate::{ChooseBranch, StepAst, StepPrimitive, TogetherBranch};

enum Frame<'a> {
    Body {
        steps: &'a [StepAst],
        index: usize,
        width: usize,
        track: bool,
    },
    Choose {
        branches: &'a [ChooseBranch],
        index: usize,
        width: usize,
    },
    Together {
        branches: &'a [TogetherBranch],
        index: usize,
        width: usize,
    },
}

enum Action<'a> {
    Fold,
    Idle,
    Push(Frame<'a>),
}

enum Progress {
    Continue,
    Complete(usize),
}

impl<'a> Frame<'a> {
    fn tracked_body(steps: &'a [StepAst], overhead: usize, track: bool) -> Self {
        Self::Body {
            steps,
            index: 0,
            width: overhead,
            track,
        }
    }

    fn body(steps: &'a [StepAst], overhead: usize) -> Self {
        Self::tracked_body(steps, overhead, false)
    }

    fn choose(branches: &'a [ChooseBranch]) -> Self {
        Self::Choose {
            branches,
            index: 0,
            width: 1,
        }
    }

    fn together(branches: &'a [TogetherBranch]) -> Self {
        Self::Together {
            branches,
            index: 0,
            width: 2,
        }
    }

    fn width(&self) -> usize {
        match self {
            Self::Body { width, .. }
            | Self::Choose { width, .. }
            | Self::Together { width, .. } => *width,
        }
    }

    fn width_slot(&mut self) -> &mut usize {
        match self {
            Self::Body { width, .. }
            | Self::Choose { width, .. }
            | Self::Together { width, .. } => width,
        }
    }

    fn advance(&mut self) -> Result<Action<'a>, CompileError> {
        match self {
            Self::Body {
                steps,
                index,
                width,
                track,
            } => body_action(steps, index, width, *track),
            Self::Choose {
                branches, index, ..
            } => branch_action(branches, index),
            Self::Together {
                branches, index, ..
            } => together_branch_action(branches, index),
        }
    }
}

fn advance_index(index: &mut usize) -> Result<(), CompileError> {
    *index = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: *index })?;
    Ok(())
}

fn add_width(width: &mut usize, amount: usize) -> Result<(), CompileError> {
    *width = width
        .checked_add(amount)
        .ok_or(CompileError::StepIndexOutOfRange { value: *width })?;
    Ok(())
}

fn body_action<'a>(
    steps: &'a [StepAst],
    index: &mut usize,
    width: &mut usize,
    track: bool,
) -> Result<Action<'a>, CompileError> {
    let Some(step) = steps.get(*index) else {
        return Ok(Action::Fold);
    };
    advance_index(index)?;
    add_step_extras(step, width, track)?;
    primitive_action(&step.primitive, width)
}

fn add_step_extras(step: &StepAst, width: &mut usize, track: bool) -> Result<(), CompileError> {
    let copy = track && super::super::part_14::iteration_body::produces_value(step);
    let extra = usize::from(step.condition.is_some())
        .checked_add(usize::from(copy))
        .ok_or(CompileError::StepIndexOutOfRange { value: *width })?;
    add_width(width, extra)?;
    if let Some(value) = &step.with {
        add_width(width, super::super::canonical_values::width(value)?)?;
    }
    Ok(())
}

fn branch_action<'a>(
    branches: &'a [ChooseBranch],
    index: &mut usize,
) -> Result<Action<'a>, CompileError> {
    let Some(branch) = branches.get(*index) else {
        return Ok(Action::Fold);
    };
    advance_index(index)?;
    Ok(Action::Push(Frame::body(&branch.steps, 0)))
}

fn together_branch_action<'a>(
    branches: &'a [TogetherBranch],
    index: &mut usize,
) -> Result<Action<'a>, CompileError> {
    let Some(branch) = branches.get(*index) else {
        return Ok(Action::Fold);
    };
    advance_index(index)?;
    Ok(Action::Push(Frame::body(&branch.steps, 2)))
}

fn primitive_action<'a>(
    primitive: &'a StepPrimitive,
    width: &mut usize,
) -> Result<Action<'a>, CompileError> {
    match primitive {
        StepPrimitive::Set { .. }
        | StepPrimitive::Save { .. }
        | StepPrimitive::Do { .. }
        | StepPrimitive::Wait { .. }
        | StepPrimitive::Finish { .. } => add_width(width, 1).map(|()| Action::Idle),
        StepPrimitive::Ask { .. } => add_width(width, 2).map(|()| Action::Idle),
        _ => container_action(primitive, *width),
    }
}

fn container_action(
    primitive: &StepPrimitive,
    parent_width: usize,
) -> Result<Action<'_>, CompileError> {
    match primitive {
        StepPrimitive::ForEach { .. } => for_each_action(primitive, parent_width),
        StepPrimitive::Collect { .. }
        | StepPrimitive::Repeat { .. }
        | StepPrimitive::Aggregate { .. } => loop_action(primitive),
        StepPrimitive::Choose { branches, .. } => Ok(Action::Push(Frame::choose(branches))),
        StepPrimitive::Together { branches } => Ok(Action::Push(Frame::together(branches))),
        other => unsupported_body(other),
    }
}

fn for_each_action(
    primitive: &StepPrimitive,
    parent_width: usize,
) -> Result<Action<'_>, CompileError> {
    let StepPrimitive::ForEach { input, body, .. } = primitive else {
        return unsupported_body(primitive);
    };
    let tracked = super::super::part_14::iteration_body::needs_tracking(body);
    let overhead = 6usize
        .checked_add(super::super::list_input::source_width(input))
        .and_then(|width| width.checked_add(usize::from(tracked)))
        .ok_or(CompileError::StepIndexOutOfRange {
            value: parent_width,
        })?;
    Ok(Action::Push(Frame::tracked_body(body, overhead, tracked)))
}

fn loop_action(primitive: &StepPrimitive) -> Result<Action<'_>, CompileError> {
    let (body, overhead) = match primitive {
        StepPrimitive::Collect { body, .. } => (body, 3),
        StepPrimitive::Repeat { body, .. } => (body, 4),
        StepPrimitive::Aggregate { body, .. } => (
            body,
            if super::reduce_direct_body(body) {
                4
            } else {
                5
            },
        ),
        other => return unsupported_body(other),
    };
    Ok(Action::Push(Frame::body(body, overhead)))
}

fn unsupported_body(primitive: &StepPrimitive) -> Result<Action<'_>, CompileError> {
    Err(CompileError::UnsupportedStepPrimitive {
        step: 0,
        primitive: canonical_primitive_name(primitive)?,
    })
}

fn fold_frame(stack: &mut Vec<Frame<'_>>) -> Result<Progress, CompileError> {
    let total = stack.pop().map_or(0, |frame| frame.width());
    let Some(parent) = stack.last_mut() else {
        return Ok(Progress::Complete(total));
    };
    add_width(parent.width_slot(), total)?;
    Ok(Progress::Continue)
}

pub(super) fn body_width(body: &[StepAst], overhead: usize) -> Result<usize, CompileError> {
    let mut stack = vec![Frame::body(body, overhead)];
    loop {
        let action = match stack.last_mut() {
            None => return Ok(0),
            Some(frame) => frame.advance()?,
        };
        match action {
            Action::Fold => match fold_frame(&mut stack)? {
                Progress::Continue => {}
                Progress::Complete(width) => return Ok(width),
            },
            Action::Idle => {}
            Action::Push(frame) => stack.push(frame),
        }
    }
}

#[cfg(test)]
#[path = "body_width_walk/fixtures.rs"]
mod fixtures;

#[cfg(test)]
#[path = "body_width_walk/tests.rs"]
mod tests;
