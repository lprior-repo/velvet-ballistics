#![forbid(unsafe_code)]

use super::{CompileError, canonical_primitive_name};
use crate::{ChooseBranch, StepAst, StepPrimitive};

enum Frame<'a> {
    Body {
        steps: &'a [StepAst],
        index: usize,
        width: usize,
    },
    Choose {
        branches: &'a [ChooseBranch],
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
    fn body(steps: &'a [StepAst], overhead: usize) -> Self {
        Self::Body {
            steps,
            index: 0,
            width: overhead,
        }
    }

    fn choose(branches: &'a [ChooseBranch]) -> Self {
        Self::Choose {
            branches,
            index: 0,
            width: 1,
        }
    }

    fn width(&self) -> usize {
        match self {
            Self::Body { width, .. } | Self::Choose { width, .. } => *width,
        }
    }

    fn width_slot(&mut self) -> &mut usize {
        match self {
            Self::Body { width, .. } | Self::Choose { width, .. } => width,
        }
    }

    fn advance(&mut self) -> Result<Action<'a>, CompileError> {
        match self {
            Self::Body {
                steps,
                index,
                width,
            } => body_action(steps, index, width),
            Self::Choose {
                branches, index, ..
            } => branch_action(branches, index),
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
) -> Result<Action<'a>, CompileError> {
    let Some(step) = steps.get(*index) else {
        return Ok(Action::Fold);
    };
    advance_index(index)?;
    primitive_action(&step.primitive, width)
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

fn primitive_action<'a>(
    primitive: &'a StepPrimitive,
    width: &mut usize,
) -> Result<Action<'a>, CompileError> {
    match primitive {
        StepPrimitive::Set { .. } | StepPrimitive::Do { .. } => {
            add_width(width, 1).map(|()| Action::Idle)
        }
        StepPrimitive::ForEach { body, .. } => Ok(Action::Push(Frame::body(body, 2))),
        StepPrimitive::Collect { body, .. } | StepPrimitive::Repeat { body, .. } => {
            Ok(Action::Push(Frame::body(body, 3)))
        }
        StepPrimitive::Choose { branches, .. } => Ok(Action::Push(Frame::choose(branches))),
        other => Err(CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: canonical_primitive_name(other),
        }),
    }
}

fn fold_frame(stack: &mut Vec<Frame<'_>>) -> Result<Progress, CompileError> {
    let total = stack.pop().map_or(0, |frame| frame.width());
    let Some(parent) = stack.last_mut() else {
        return Ok(Progress::Complete(total));
    };
    add_width(parent.width_slot(), total)?;
    Ok(Progress::Continue)
}

/// Iterative layout width for a nested body, replacing the recursive
/// `body_width` -> `canonical_body_step_width` -> `body_width` cycle with an
/// explicit heap stack (Rule 1: no recursion).
///
/// Each frame owns one body (or choose-branch table) plus its running
/// accumulator; container steps push a child frame instead of recursing and
/// completed frames fold their total into the parent. Error values match the
/// recursive version exactly: the accumulator that overflowed, or the
/// parent's accumulator when a child total overflows into it.
///
/// Termination: every iteration either advances one frame's cursor (one step
/// or branch consumed, never revisited) or pops a finished frame, so the
/// total number of iterations is bounded by the number of nodes in the AST
/// tree; the stack depth never exceeds the AST nesting depth and lives on
/// the heap, so deep nesting cannot overflow the call stack.
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
