use super::super::canonical_primitive_name;
use crate::mod_compile_errors::CompileError;

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
pub(in crate::mod_compile_lowering) fn body_width(body: &[crate::StepAst], overhead: usize) -> Result<usize, CompileError> {
    enum Frame<'a> {
        Body {
            steps: &'a [crate::StepAst],
            index: usize,
            width: usize,
        },
        Choose {
            branches: &'a [crate::ChooseBranch],
            index: usize,
            width: usize,
        },
    }
    impl<'a> Frame<'a> {
        fn width(&self) -> usize {
            match self {
                Frame::Body { width, .. } | Frame::Choose { width, .. } => *width,
            }
        }
        fn width_slot(&mut self) -> &mut usize {
            match self {
                Frame::Body { width, .. } | Frame::Choose { width, .. } => width,
            }
        }
        fn fold(width: usize, parent: &mut Frame<'a>) -> Result<(), CompileError> {
            *parent.width_slot() = parent.width_slot().checked_add(width).ok_or(
                CompileError::StepIndexOutOfRange {
                    value: *parent.width_slot(),
                },
            )?;
            Ok(())
        }
    }
    enum Step<'a> {
        Fold,
        Idle,
        Push(Frame<'a>),
    }
    let mut stack = vec![Frame::Body {
        steps: body,
        index: 0,
        width: overhead,
    }];
    loop {
        // Decide the next action inside a scoped borrow, then act on the
        // stack after the borrow ends.
        let step = match stack.last_mut() {
            None => return Ok(0),
            Some(Frame::Body {
                steps,
                index,
                width,
            }) => match steps.get(*index) {
                None => Step::Fold,
                Some(step) => {
                    *index = index
                        .checked_add(1)
                        .ok_or(CompileError::StepIndexOutOfRange { value: *index })?;
                    let _ = width;
                    match &step.primitive {
                        crate::StepPrimitive::Set { .. } | crate::StepPrimitive::Do { .. } => {
                            *width = width
                                .checked_add(1)
                                .ok_or(CompileError::StepIndexOutOfRange { value: *width })?;
                            Step::Idle
                        }
                        crate::StepPrimitive::ForEach { body, .. } => Step::Push(Frame::Body {
                            steps: body,
                            index: 0,
                            width: 2,
                        }),
                        crate::StepPrimitive::Collect { body, .. }
                        | crate::StepPrimitive::Repeat { body, .. } => Step::Push(Frame::Body {
                            steps: body,
                            index: 0,
                            width: 3,
                        }),
                        crate::StepPrimitive::Choose { branches, .. } => {
                            Step::Push(Frame::Choose {
                                branches,
                                index: 0,
                                width: 1,
                            })
                        }
                        other => {
                            return Err(CompileError::UnsupportedStepPrimitive {
                                step: 0,
                                primitive: canonical_primitive_name(other),
                            });
                        }
                    }
                }
            },
            Some(Frame::Choose {
                branches, index, ..
            }) => match branches.get(*index) {
                None => Step::Fold,
                Some(branch) => {
                    *index = index
                        .checked_add(1)
                        .ok_or(CompileError::StepIndexOutOfRange { value: *index })?;
                    Step::Push(Frame::Body {
                        steps: &branch.steps,
                        index: 0,
                        width: 0,
                    })
                }
            },
        };
        match step {
            Step::Fold => {
                let total = stack.last().map_or(0, Frame::width);
                stack.pop();
                let Some(parent) = stack.last_mut() else {
                    return Ok(total);
                };
                Frame::fold(total, parent)?;
            }
            Step::Idle => {}
            Step::Push(frame) => stack.push(frame),
        }
    }
}

pub(in crate::mod_compile_lowering) fn choose_width(branches: &[crate::ChooseBranch]) -> Result<usize, CompileError> {
    let mut width = 1usize;
    for branch in branches {
        width = width
            .checked_add(body_width(&branch.steps, 0)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(in crate::mod_compile_lowering) fn together_width(branches: &[crate::TogetherBranch]) -> Result<usize, CompileError> {
    let mut width = 2usize;
    for branch in branches {
        width = width
            .checked_add(body_width(&branch.steps, 1)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

/// Width of one step nested inside a loop body or choose branch.
///
/// Mirrors [`canonical_step_width`] for the container primitives that own
/// multi-step bodies, so layout reserves exactly the nodes recursive body
/// emission produces. Value-position containers (`aggregate`, `together`) and
/// boundary/terminal primitives stay rejected here exactly as in emission.
pub(in crate::mod_compile_lowering) fn canonical_body_step_width(
    primitive: &crate::StepPrimitive,
) -> Result<usize, CompileError> {
    match primitive {
        crate::StepPrimitive::Set { .. } | crate::StepPrimitive::Do { .. } => Ok(1),
        crate::StepPrimitive::ForEach { body, .. } => body_width(body, 2),
        crate::StepPrimitive::Collect { body, .. } | crate::StepPrimitive::Repeat { body, .. } => {
            body_width(body, 3)
        }
        crate::StepPrimitive::Choose { branches, .. } => choose_width(branches),
        other => Err(CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: canonical_primitive_name(other),
        }),
    }
}
