#![forbid(unsafe_code)]
//! Slot-reference lowering for expression bytecode.

use super::{ExpressionReferenceResolver, split_reference_tail};
use crate::CompileError;
use vb_core::{AccessorIdx, AccessorProgram, ExprOp, PathSegment, SlotIdx};

#[cfg(test)]
pub(super) struct RejectingReferenceResolver;

#[cfg(test)]
impl ExpressionReferenceResolver for RejectingReferenceResolver {
    fn resolve_reference(&mut self, _reference: &str) -> Result<ExprOp, CompileError> {
        Err(CompileError::ExpressionLoweringUnsupported {
            feature: "accessor references".into(),
        })
    }
}

#[cfg(test)]
pub(super) struct SlotAccessorReferenceResolver<'a> {
    pub(super) accessors: &'a mut Vec<AccessorProgram>,
}

#[cfg(test)]
impl ExpressionReferenceResolver for SlotAccessorReferenceResolver<'_> {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let lowered = lower_slot_reference(reference, self.accessors)?;
        Ok(lowered)
    }
}

#[cfg(test)]
fn lower_slot_reference(
    reference: &str,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let (root, tail) = parse_slot_reference_parts(reference)?;
    let (slot, path) = split_reference_tail(tail);
    let root_slot = parse_slot_reference_index(reference, slot)?;
    match path {
        Some(path) => lower_accessor_reference(reference, root, slot, path, root_slot, accessors),
        None => Ok(ExprOp::LoadSlot(root_slot)),
    }
}

#[cfg(test)]
fn parse_slot_reference_parts(reference: &str) -> Result<(&str, &str), CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(reference),
        });
    };
    let Some((root, tail)) = body.split_once('.') else {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(body),
        });
    };
    if !matches!(root, "slot" | "slots") {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(root),
        });
    }
    Ok((root, tail))
}

#[cfg(test)]
fn parse_slot_reference_index(reference: &str, slot: &str) -> Result<SlotIdx, CompileError> {
    let parsed = slot
        .parse::<u16>()
        .map_err(|_| CompileError::UnknownReferenceName {
            kind: "slot",
            reference: Box::<str>::from(reference),
            name: Box::<str>::from(slot),
        })?;
    Ok(SlotIdx::new(parsed))
}

#[cfg(test)]
fn lower_accessor_reference(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
    root_slot: SlotIdx,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let path = numeric_path_segments(reference, root, slot, path)?;
    let index = u16::try_from(accessors.len()).map_err(|_| {
        CompileError::ExpressionLoweringUnsupported {
            feature: "accessor table overflow".into(),
        }
    })?;
    accessors.push(AccessorProgram {
        root: root_slot,
        path: path.into_boxed_slice(),
    });
    Ok(ExprOp::LoadAccessor(AccessorIdx::new(index)))
}

fn numeric_path_segments(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
) -> Result<Vec<PathSegment>, CompileError> {
    let mut segments = Vec::new();
    for segment in path.split('.') {
        let index = parse_list_index_segment(reference, root, slot, path, segment)?;
        segments.push(PathSegment::Index(index));
    }
    Ok(segments)
}

#[cfg(test)]
fn parse_list_index_segment(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
    segment: &str,
) -> Result<u32, CompileError> {
    segment
        .parse::<u32>()
        .map_err(|_| unsupported_accessor_reference(reference, root, slot, path))
}

#[cfg(test)]
fn unsupported_accessor_reference(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
) -> CompileError {
    CompileError::UnsupportedAccessorReference {
        reference: Box::<str>::from(reference),
        root: Box::<str>::from(format!("{root}.{slot}")),
        path: Box::<str>::from(path),
    }
}
