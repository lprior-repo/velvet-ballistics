use super::part_05_digest_primitive::{
    DigestOp, HeaderNested, MAX_BODY_NESTING_DEPTH, body_nesting_error, child_depth,
    digest_step_post, digest_step_pre, emit_primitive_header,
};
use crate::mod_compile_errors::{CompileError, CompileErrors};

enum Frame<'a> {
    Step {
        step: &'a crate::StepAst,
        depth: u16,
        stage: u8,
    },
    Body {
        steps: &'a [crate::StepAst],
        index: usize,
        depth: u16,
    },
    Choose {
        branches: &'a [crate::ChooseBranch],
        index: usize,
        depth: u16,
        otherwise: Option<&'a str>,
    },
    Together {
        branches: &'a [crate::TogetherBranch],
        index: usize,
        depth: u16,
    },
}

fn run_digest_stack(
    hasher: &mut blake3::Hasher,
    mut stack: Vec<Frame<'_>>,
) -> Result<(), CompileErrors> {
    type Op<'a> = DigestOp<Frame<'a>>;
    loop {
        let op = match stack.last_mut() {
            None => return Ok(()),
            Some(Frame::Step { step, depth, stage }) => {
                if *stage == 0 {
                    let step_ref = *step;
                    let depth_val = *depth;
                    if depth_val > MAX_BODY_NESTING_DEPTH {
                        let name = super::canonical_primitive_name(&step_ref.primitive)
                            .map_err(|e| CompileErrors(vec![e]))?;
                        return Err(body_nesting_error(name, depth_val));
                    }
                    digest_step_pre(hasher, step_ref)?;
                    let name = super::canonical_primitive_name(&step_ref.primitive)
                        .map_err(|e| CompileErrors(vec![e]))?;
                    let nested = emit_primitive_header(hasher, &step_ref.primitive)?;
                    *stage = 1;
                    match nested {
                        HeaderNested::None => Op::Nothing,
                        HeaderNested::Body(body) => {
                            super::digest_len(hasher, body.len())?;
                            let child = child_depth(depth_val, name)?;
                            Op::Push(Frame::Body {
                                steps: body,
                                index: 0,
                                depth: child,
                            })
                        }
                        HeaderNested::Choose(branches, otherwise) => {
                            let child = child_depth(depth_val, name)?;
                            Op::Push(Frame::Choose {
                                branches,
                                index: 0,
                                depth: child,
                                otherwise,
                            })
                        }
                        HeaderNested::Together(branches) => {
                            let child = child_depth(depth_val, name)?;
                            Op::Push(Frame::Together {
                                branches,
                                index: 0,
                                depth: child,
                            })
                        }
                    }
                } else {
                    let step_ref = *step;
                    digest_step_post(hasher, step_ref)?;
                    Op::Pop
                }
            }
            Some(Frame::Body {
                steps,
                index,
                depth,
            }) => match steps.get(*index) {
                None => {
                    hasher.update(&[super::TAG_END]);
                    Op::Pop
                }
                Some(step) => {
                    let step_ref: &crate::StepAst = step;
                    let depth_val = *depth;
                    *index = index.checked_add(1).ok_or_else(|| {
                        CompileErrors(vec![CompileError::StepIndexOutOfRange { value: *index }])
                    })?;
                    Op::Push(Frame::Step {
                        step: step_ref,
                        depth: depth_val,
                        stage: 0,
                    })
                }
            },
            Some(Frame::Choose {
                branches,
                index,
                depth,
                otherwise,
            }) => match branches.get(*index) {
                None => {
                    hasher.update(&[super::TAG_END]);
                    let otherwise_opt = *otherwise;
                    super::digest_optional_string(hasher, b"otherwise:", otherwise_opt)?;
                    Op::Pop
                }
                Some(branch) => {
                    let when = branch.when.clone();
                    let steps_ref: &[crate::StepAst] = branch.steps.as_slice();
                    let depth_val = *depth;
                    *index = index.checked_add(1).ok_or_else(|| {
                        CompileErrors(vec![CompileError::StepIndexOutOfRange { value: *index }])
                    })?;
                    super::digest_string(hasher, &when)?;
                    super::digest_len(hasher, steps_ref.len())?;
                    Op::Push(Frame::Body {
                        steps: steps_ref,
                        index: 0,
                        depth: depth_val,
                    })
                }
            },
            Some(Frame::Together {
                branches,
                index,
                depth,
            }) => match branches.get(*index) {
                None => {
                    hasher.update(&[super::TAG_END]);
                    Op::Pop
                }
                Some(branch) => {
                    let label = branch.label.clone();
                    let steps_ref: &[crate::StepAst] = branch.steps.as_slice();
                    let depth_val = *depth;
                    *index = index.checked_add(1).ok_or_else(|| {
                        CompileErrors(vec![CompileError::StepIndexOutOfRange { value: *index }])
                    })?;
                    super::digest_string(hasher, &label)?;
                    super::digest_len(hasher, steps_ref.len())?;
                    Op::Push(Frame::Body {
                        steps: steps_ref,
                        index: 0,
                        depth: depth_val,
                    })
                }
            },
        };
        match op {
            Op::Push(frame) => stack.push(frame),
            Op::Pop => {
                stack.pop();
            }
            Op::Nothing => {}
        }
    }
}

pub(super) fn digest_step_at(
    hasher: &mut blake3::Hasher,
    step: &crate::StepAst,
    depth: u16,
) -> Result<(), CompileErrors> {
    run_digest_stack(
        hasher,
        vec![Frame::Step {
            step,
            depth,
            stage: 0,
        }],
    )
}

pub(super) fn digest_primitive_entry(
    hasher: &mut blake3::Hasher,
    primitive: &crate::StepPrimitive,
    depth: u16,
) -> Result<(), CompileErrors> {
    if depth > MAX_BODY_NESTING_DEPTH {
        let name =
            super::canonical_primitive_name(primitive).map_err(|e| CompileErrors(vec![e]))?;
        return Err(body_nesting_error(name, depth));
    }
    let name = super::canonical_primitive_name(primitive).map_err(|e| CompileErrors(vec![e]))?;
    let nested = emit_primitive_header(hasher, primitive)?;
    match nested {
        HeaderNested::None => Ok(()),
        HeaderNested::Body(body) => {
            super::digest_len(hasher, body.len())?;
            let child = child_depth(depth, name)?;
            run_digest_stack(
                hasher,
                vec![Frame::Body {
                    steps: body,
                    index: 0,
                    depth: child,
                }],
            )
        }
        HeaderNested::Choose(branches, otherwise) => {
            let child = child_depth(depth, name)?;
            run_digest_stack(
                hasher,
                vec![Frame::Choose {
                    branches,
                    index: 0,
                    depth: child,
                    otherwise,
                }],
            )
        }
        HeaderNested::Together(branches) => {
            let child = child_depth(depth, name)?;
            run_digest_stack(
                hasher,
                vec![Frame::Together {
                    branches,
                    index: 0,
                    depth: child,
                }],
            )
        }
    }
}
