#![forbid(unsafe_code)]

use crate::{CompileError, CompileErrors, StepAst, StepPrimitive};

pub(in crate::mod_compile_lowering) fn validate_for_each_capture(
    variable: &str,
    body: &[StepAst],
) -> Result<(), CompileErrors> {
    validate_capture(variable, body)
}

pub(in crate::mod_compile_lowering) fn validate_reduce_capture(
    variable: &str,
    body: &[StepAst],
) -> Result<(), CompileErrors> {
    validate_capture(variable, body)
}

pub(in crate::mod_compile_lowering) fn validate_capture(
    variable: &str,
    body: &[StepAst],
) -> Result<(), CompileErrors> {
    // Borrowed lexical scope: single-element stack slice referencing the
    // caller variable, zero heap allocation for the root.
    let enclosing: &[&str] = &[variable];
    walk_body(body, enclosing)
}

fn duplicate(name: &str) -> CompileErrors {
    CompileErrors(vec![CompileError::DuplicateOutputName {
        name: name.into(),
    }])
}

/// Iterative capture validation on an explicit heap stack, replacing the
/// recursive `walk_body` -> `check_step` -> `descend` -> `walk_body` cycle
/// (Rule 1: no recursion). Semantics are unchanged: every step id and bound
/// variable is checked against its enclosing variables in depth-first
/// pre-order, and the first duplicate encountered is reported.
///
/// Termination: every iteration advances one frame's cursor over a distinct
/// step or branch (never revisited) or pops a finished frame, so iterations
/// are bounded by the number of AST nodes; stack depth never exceeds AST
/// nesting depth and lives on the heap. `enclosing` is a borrowed `&str`
/// chain cloned per descended frame (pointer copies only, no `String`
/// allocations), matching the old `descend` traversal shape while keeping
/// the traversal off the call stack. This preserves the heap-stack
/// termination proof while eliminating the per-level `String` clone cost.
fn walk_body<'a>(body: &'a [StepAst], enclosing: &[&'a str]) -> Result<(), CompileErrors> {
    enum Frame<'a> {
        Body {
            steps: &'a [StepAst],
            index: usize,
            enclosing: Vec<&'a str>,
        },
        Choose {
            branches: &'a [crate::ChooseBranch],
            index: usize,
            enclosing: Vec<&'a str>,
        },
        Together {
            branches: &'a [crate::TogetherBranch],
            index: usize,
            enclosing: Vec<&'a str>,
        },
    }
    enum Op<'a> {
        Pop,
        Push(Frame<'a>),
        LeafCheck { name: &'a str },
        Nothing,
    }
    let mut stack = vec![Frame::Body {
        steps: body,
        index: 0,
        enclosing: enclosing.to_vec(),
    }];
    loop {
        // Decide the next action inside a scoped borrow, then act on the
        // stack after the borrow ends.
        let op = match stack.last_mut() {
            None => return Ok(()),
            Some(Frame::Body {
                steps,
                index,
                enclosing,
            }) => match steps.get(*index) {
                None => Op::Pop,
                Some(step) => {
                    *index = index.checked_add(1).ok_or_else(|| {
                        CompileErrors(vec![CompileError::StepIndexOutOfRange { value: *index }])
                    })?;
                    // Borrowed slice contains: no String allocation, pointer compare only.
                    if enclosing.contains(&step.id.as_str()) {
                        return Err(duplicate(&step.id));
                    }
                    match &step.primitive {
                        StepPrimitive::Set { output, .. } => Op::LeafCheck {
                            name: output.as_str(),
                        },
                        StepPrimitive::ForEach { variable, body, .. }
                        | StepPrimitive::Collect { variable, body, .. }
                        | StepPrimitive::Aggregate { variable, body, .. } => {
                            check_binding(variable, enclosing)?;
                            let mut nested = enclosing.clone();
                            nested.push(variable.as_str());
                            Op::Push(Frame::Body {
                                steps: body,
                                index: 0,
                                enclosing: nested,
                            })
                        }
                        StepPrimitive::Repeat { body, .. } => Op::Push(Frame::Body {
                            steps: body,
                            index: 0,
                            enclosing: enclosing.clone(),
                        }),
                        StepPrimitive::Choose { branches, .. } => Op::Push(Frame::Choose {
                            branches,
                            index: 0,
                            enclosing: enclosing.clone(),
                        }),
                        StepPrimitive::Together { branches } => Op::Push(Frame::Together {
                            branches,
                            index: 0,
                            enclosing: enclosing.clone(),
                        }),
                        _ => Op::Nothing,
                    }
                }
            },
            Some(Frame::Choose {
                branches,
                index,
                enclosing,
            }) => match branches.get(*index) {
                None => Op::Pop,
                Some(branch) => {
                    *index = index.checked_add(1).ok_or_else(|| {
                        CompileErrors(vec![CompileError::StepIndexOutOfRange { value: *index }])
                    })?;
                    Op::Push(Frame::Body {
                        steps: &branch.steps,
                        index: 0,
                        enclosing: enclosing.clone(),
                    })
                }
            },
            Some(Frame::Together {
                branches,
                index,
                enclosing,
            }) => match branches.get(*index) {
                None => Op::Pop,
                Some(branch) => {
                    *index = index.checked_add(1).ok_or_else(|| {
                        CompileErrors(vec![CompileError::StepIndexOutOfRange { value: *index }])
                    })?;
                    Op::Push(Frame::Body {
                        steps: &branch.steps,
                        index: 0,
                        enclosing: enclosing.clone(),
                    })
                }
            },
        };
        match op {
            Op::Pop => {
                stack.pop();
            }
            Op::Push(frame) => stack.push(frame),
            Op::LeafCheck { name } => {
                let enclosing: &[&str] = match stack.last() {
                    Some(Frame::Body { enclosing, .. })
                    | Some(Frame::Choose { enclosing, .. })
                    | Some(Frame::Together { enclosing, .. }) => enclosing,
                    None => &[],
                };
                check_binding(name, enclosing)?;
            }
            Op::Nothing => {}
        }
    }
}

fn check_binding(name: &str, enclosing: &[&str]) -> Result<(), CompileErrors> {
    if enclosing.contains(&name) {
        Err(duplicate(name))
    } else {
        Ok(())
    }
}
