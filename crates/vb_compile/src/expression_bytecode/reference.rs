//! Slot and step reference parsing and lowering.
//!
//! Handles `$slot.N`, `$slots.N`, `$step.N`, and `$steps.N` reference syntax,
/// parsing them into `LoadSlot` or `LoadAccessor` bytecode operations.

use crate::CompileError;
use vb_core::{AccessorIdx, AccessorProgram, ExprOp, PathSegment, SlotIdx};

/// Lowers a slot reference (`$slot.N` or `$slots.N`) to an `ExprOp`.
pub(crate) fn lower_slot_reference(
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

/// Lowers a step reference (`$step.N` or `$steps.N`) to an `ExprOp`.
///
/// For bare step references like `$steps.build_result`, looks up the step name
/// in the step_slots mapping and returns `LoadSlot(slot)`.
///
/// For step references with field accessors like `$steps.build.result`, creates
/// an AccessorProgram with the step's output slot as root and the field as path.
pub(crate) fn lower_step_reference(
    reference: &str,
    step_slots: &[(Box<str>, SlotIdx)],
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let (step_id, field) = parse_step_reference_parts(reference)?;
    let root_slot = resolve_step_slot(reference, step_id, step_slots)?;
    match field {
        Some(field_path) => {
            let path_segments = parse_field_path_segments(reference, field_path)?;
            let index = u16::try_from(accessors.len()).map_err(|_| {
                CompileError::ExpressionLoweringUnsupported {
                    feature: "accessor table overflow".into(),
                }
            })?;
            accessors.push(AccessorProgram {
                root: root_slot,
                path: path_segments.into_boxed_slice(),
            });
            Ok(ExprOp::LoadAccessor(AccessorIdx::new(index)))
        }
        None => Ok(ExprOp::LoadSlot(root_slot)),
    }
}

// ── Slot reference parsing ──────────────────────────────────────────────────

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

/// Parses step reference parts from a reference string.
///
/// Returns `Ok((step_id, field_option))` where:
/// - `step_id` is the step identifier (e.g., "build_result" from "$steps.build_result")
/// - `field_option` is `None` for bare references or `Some(field)` for accessors
fn parse_step_reference_parts(reference: &str) -> Result<(&str, Option<&str>), CompileError> {
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
    if !matches!(root, "step" | "steps") {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(root),
        });
    }
    // Step reference: $step.<id> or $step.<id>.<field>
    // The tail is <id>[.<field>]
    let (step_id, field) = split_reference_tail(tail);
    Ok((step_id, field))
}

// ── Step slot resolution ────────────────────────────────────────────────────

/// Resolves a step ID to its output SlotIdx using the step_slots mapping.
fn resolve_step_slot(
    reference: &str,
    step_id: &str,
    step_slots: &[(Box<str>, SlotIdx)],
) -> Result<SlotIdx, CompileError> {
    step_slots
        .iter()
        .find(|(name, _)| name.as_ref() == step_id)
        .map(|(_, slot)| *slot)
        .ok_or_else(|| CompileError::UnknownReferenceName {
            kind: "step",
            reference: Box::<str>::from(reference),
            name: Box::<str>::from(step_id),
        })
}

// ── Field path parsing ──────────────────────────────────────────────────────

/// Parses a field path into PathSegment indices.
///
/// For field accessors like "result" or "data.value", creates numeric index segments.
/// Currently only supports the "result" field which maps to index 0.
fn parse_field_path_segments(
    reference: &str,
    field_path: &str,
) -> Result<Vec<PathSegment>, CompileError> {
    let mut segments = Vec::new();
    for segment in field_path.split('.') {
        if segment == "result" {
            segments.push(PathSegment::Index(0));
        } else {
            return Err(CompileError::UnsupportedAccessorReference {
                reference: Box::<str>::from(reference),
                root: Box::<str>::from("steps.<id>".to_string()),
                path: Box::<str>::from(field_path),
            });
        }
    }
    Ok(segments)
}

/// Splits a reference tail into (slot_id, optional path).
///
/// For `"build.result"`, returns `("build", Some("result"))`.
/// For `"build"`, returns `("build", None)`.
pub(crate) fn split_reference_tail(tail: &str) -> (&str, Option<&str>) {
    match tail.split_once('.') {
        Some((slot, path)) => (slot, Some(path)),
        None => (tail, None),
    }
}

/// Parses a slot reference index from the slot name portion.
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

// ── Accessor reference lowering ─────────────────────────────────────────────

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
