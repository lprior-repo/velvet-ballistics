use super::part_05_digest_primitive::MAX_BODY_NESTING_DEPTH;
use crate::mod_compile_errors::{CompileError, CompileErrors};
fn author_nesting_error(depth: u16) -> CompileErrors {
    body_nesting_error("author_value", depth)
}
fn body_nesting_error(primitive: &'static str, depth: u16) -> CompileErrors {
    let err = CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field: "body nesting depth",
        value: usize::from(depth),
        limit: usize::from(MAX_BODY_NESTING_DEPTH),
    };
    CompileErrors(vec![err])
}

fn child_depth(depth: u16, primitive: &'static str) -> Result<u16, CompileErrors> {
    depth
        .checked_add(1)
        .ok_or_else(|| body_nesting_error(primitive, depth))
}

/// Hash an AuthorValue iteratively (pre-order + `TAG_END`); depth budget fails
/// closed with `PrimitiveLoweringLimitExceeded` (mirrors `canonical_branch_body`).
pub(crate) fn digest_author_value(
    hasher: &mut blake3::Hasher,
    value: &crate::AuthorValue,
) -> Result<(), CompileErrors> {
    enum Frame<'a> {
        Seq {
            items: &'a [crate::AuthorValue],
            index: usize,
            depth: u16,
        },
        Map {
            entries: Vec<&'a crate::AuthorEntry<crate::AuthorValue>>,
            index: usize,
            depth: u16,
        },
    }
    enum Op<'a> {
        Push(Frame<'a>),
        Pop,
        Nothing,
    }
    let mut stack: Vec<Frame<'_>> = Vec::new();
    match value {
        crate::AuthorValue::Null => {
            hasher.update(&[0x00]);
            return Ok(());
        }
        crate::AuthorValue::Bool(b) => {
            hasher.update(&[0x01]);
            hasher.update(&[if *b { 1 } else { 0 }]);
            return Ok(());
        }
        crate::AuthorValue::I64(n) => {
            hasher.update(&[0x02]);
            super::digest_i64(hasher, *n);
            return Ok(());
        }
        crate::AuthorValue::Text(s) => {
            hasher.update(&[0x03]);
            super::digest_string(hasher, s)?;
            return Ok(());
        }
        crate::AuthorValue::Sequence(seq) => {
            hasher.update(&[0x04]);
            super::digest_len(hasher, seq.len())?;
            stack.push(Frame::Seq {
                items: seq,
                index: 0,
                depth: 0,
            });
        }
        crate::AuthorValue::Mapping(entries) => {
            hasher.update(&[0x05]);
            let mut sorted: Vec<&crate::AuthorEntry<crate::AuthorValue>> = entries.iter().collect();
            sorted.sort_by(|a, b| a.key.cmp(&b.key));
            super::digest_len(hasher, sorted.len())?;
            stack.push(Frame::Map {
                entries: sorted,
                index: 0,
                depth: 0,
            });
        }
        _ => {
            return Err(super::digest_error(
                "unknown author value variant cannot be canonicalized",
            ));
        }
    }
    loop {
        let op = match stack.last_mut() {
            None => return Ok(()),
            Some(Frame::Seq {
                items,
                index,
                depth,
            }) => match items.get(*index) {
                None => {
                    hasher.update(&[super::TAG_END]);
                    Op::Pop
                }
                Some(item) => {
                    let item_ref: &crate::AuthorValue = item;
                    let parent_depth = *depth;
                    *index = index.checked_add(1).ok_or_else(|| {
                        CompileErrors(vec![CompileError::StepIndexOutOfRange { value: *index }])
                    })?;
                    let next = child_depth(parent_depth, "author_value")?;
                    if next > MAX_BODY_NESTING_DEPTH {
                        return Err(author_nesting_error(next));
                    }
                    match item_ref {
                        crate::AuthorValue::Null => {
                            hasher.update(&[0x00]);
                            Op::Nothing
                        }
                        crate::AuthorValue::Bool(b) => {
                            hasher.update(&[0x01]);
                            hasher.update(&[if *b { 1 } else { 0 }]);
                            Op::Nothing
                        }
                        crate::AuthorValue::I64(n) => {
                            hasher.update(&[0x02]);
                            super::digest_i64(hasher, *n);
                            Op::Nothing
                        }
                        crate::AuthorValue::Text(s) => {
                            hasher.update(&[0x03]);
                            super::digest_string(hasher, s)?;
                            Op::Nothing
                        }
                        crate::AuthorValue::Sequence(seq) => {
                            hasher.update(&[0x04]);
                            super::digest_len(hasher, seq.len())?;
                            Op::Push(Frame::Seq {
                                items: seq,
                                index: 0,
                                depth: next,
                            })
                        }
                        crate::AuthorValue::Mapping(entries) => {
                            hasher.update(&[0x05]);
                            let mut sorted: Vec<&crate::AuthorEntry<crate::AuthorValue>> =
                                entries.iter().collect();
                            sorted.sort_by(|a, b| a.key.cmp(&b.key));
                            super::digest_len(hasher, sorted.len())?;
                            Op::Push(Frame::Map {
                                entries: sorted,
                                index: 0,
                                depth: next,
                            })
                        }
                        _ => {
                            return Err(super::digest_error(
                                "unknown author value variant cannot be canonicalized",
                            ));
                        }
                    }
                }
            },
            Some(Frame::Map {
                entries,
                index,
                depth,
            }) => match entries.get(*index) {
                None => {
                    hasher.update(&[super::TAG_END]);
                    Op::Pop
                }
                Some(entry) => {
                    let entry_value = &entry.value;
                    let key = entry.key.clone();
                    let parent_depth = *depth;
                    *index = index.checked_add(1).ok_or_else(|| {
                        CompileErrors(vec![CompileError::StepIndexOutOfRange { value: *index }])
                    })?;
                    super::digest_string(hasher, &key)?;
                    let next = child_depth(parent_depth, "author_value")?;
                    if next > MAX_BODY_NESTING_DEPTH {
                        return Err(author_nesting_error(next));
                    }
                    match entry_value {
                        crate::AuthorValue::Null => {
                            hasher.update(&[0x00]);
                            Op::Nothing
                        }
                        crate::AuthorValue::Bool(b) => {
                            hasher.update(&[0x01]);
                            hasher.update(&[if *b { 1 } else { 0 }]);
                            Op::Nothing
                        }
                        crate::AuthorValue::I64(n) => {
                            hasher.update(&[0x02]);
                            super::digest_i64(hasher, *n);
                            Op::Nothing
                        }
                        crate::AuthorValue::Text(s) => {
                            hasher.update(&[0x03]);
                            super::digest_string(hasher, s)?;
                            Op::Nothing
                        }
                        crate::AuthorValue::Sequence(seq) => {
                            hasher.update(&[0x04]);
                            super::digest_len(hasher, seq.len())?;
                            Op::Push(Frame::Seq {
                                items: seq,
                                index: 0,
                                depth: next,
                            })
                        }
                        crate::AuthorValue::Mapping(nested) => {
                            hasher.update(&[0x05]);
                            let mut sorted: Vec<&crate::AuthorEntry<crate::AuthorValue>> =
                                nested.iter().collect();
                            sorted.sort_by(|a, b| a.key.cmp(&b.key));
                            super::digest_len(hasher, sorted.len())?;
                            Op::Push(Frame::Map {
                                entries: sorted,
                                index: 0,
                                depth: next,
                            })
                        }
                        _ => {
                            return Err(super::digest_error(
                                "unknown author value variant cannot be canonicalized",
                            ));
                        }
                    }
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

pub(crate) fn validate_branch_counts(source: &crate::WorkflowSource) -> Result<(), CompileErrors> {
    let mut stack: Vec<(&crate::StepPrimitive, u16)> = Vec::new();
    for step in source.steps().iter().rev() {
        stack.push((&step.primitive, 0));
    }
    while let Some((primitive, depth)) = stack.pop() {
        if depth > MAX_BODY_NESTING_DEPTH {
            return Err(body_nesting_error(
                super::canonical_primitive_name(primitive),
                depth,
            ));
        }
        match primitive {
            crate::StepPrimitive::Together { branches } => {
                if branches.len() > usize::from(u16::MAX) {
                    return Err(CompileErrors(vec![
                        CompileError::PrimitiveLoweringLimitExceeded {
                            primitive: "together",
                            field: "branches",
                            value: branches.len(),
                            limit: usize::from(u16::MAX),
                        },
                    ]));
                }
                let next = child_depth(depth, "together")?;
                for branch in branches.iter().rev() {
                    for step in branch.steps.iter().rev() {
                        stack.push((&step.primitive, next));
                    }
                }
            }
            crate::StepPrimitive::ForEach { body, .. }
            | crate::StepPrimitive::Collect { body, .. }
            | crate::StepPrimitive::Aggregate { body, .. }
            | crate::StepPrimitive::Repeat { body, .. } => {
                let next = child_depth(depth, super::canonical_primitive_name(primitive))?;
                for step in body.iter().rev() {
                    stack.push((&step.primitive, next));
                }
            }
            crate::StepPrimitive::Choose { branches, .. } => {
                let next = child_depth(depth, "choose")?;
                for branch in branches.iter().rev() {
                    for step in branch.steps.iter().rev() {
                        stack.push((&step.primitive, next));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
