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
    let enclosing = vec![variable.to_owned()];
    walk_body(body, &enclosing)
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
/// nesting depth and lives on the heap. `enclosing` is cloned per descended
/// frame, matching the old `descend` allocation shape while keeping the
/// traversal off the call stack.
fn walk_body(body: &[StepAst], enclosing: &[String]) -> Result<(), CompileErrors> {
    enum Frame<'a> {
        Body {
            steps: &'a [StepAst],
            index: usize,
            enclosing: Vec<String>,
        },
        Choose {
            branches: &'a [crate::ChooseBranch],
            index: usize,
            enclosing: Vec<String>,
        },
        Together {
            branches: &'a [crate::TogetherBranch],
            index: usize,
            enclosing: Vec<String>,
        },
    }
    enum Op<'a> {
        Pop,
        Push(Frame<'a>),
        LeafCheck { name: String },
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
                    if enclosing.iter().any(|item| item == &step.id) {
                        return Err(duplicate(&step.id));
                    }
                    match &step.primitive {
                        StepPrimitive::Set { output, .. } => Op::LeafCheck {
                            name: output.clone(),
                        },
                        StepPrimitive::ForEach { variable, body, .. }
                        | StepPrimitive::Collect { variable, body, .. }
                        | StepPrimitive::Aggregate { variable, body, .. } => {
                            check_binding(variable, enclosing)?;
                            let mut nested = enclosing.clone();
                            nested.push(variable.to_owned());
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
                let enclosing: &[String] = match stack.last() {
                    Some(Frame::Body { enclosing, .. })
                    | Some(Frame::Choose { enclosing, .. })
                    | Some(Frame::Together { enclosing, .. }) => enclosing,
                    None => &[],
                };
                check_binding(&name, enclosing)?;
            }
            Op::Nothing => {}
        }
    }
}

fn check_binding(name: &str, enclosing: &[String]) -> Result<(), CompileErrors> {
    if enclosing.iter().any(|item| item == name) {
        Err(duplicate(name))
    } else {
        Ok(())
    }
}
