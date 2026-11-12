use super::super::canonical_primitive_name;
use super::body_width_walk;
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
pub(in crate::mod_compile_lowering) fn body_width(
    body: &[crate::StepAst],
    overhead: usize,
) -> Result<usize, CompileError> {
    body_width_walk::body_width(body, overhead)
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
