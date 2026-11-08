#![forbid(unsafe_code)]
//! Step-reference lowering for expression bytecode.

use super::{ExpressionReferenceResolver, split_reference_tail};
use crate::CompileError;
use vb_core::{AccessorIdx, AccessorProgram, ExprOp, PathSegment, SlotIdx};

/// Resolver for step references ($step.<id> and $steps.<id>).
///
/// This resolver handles both bare step references like `$steps.done` (returning
/// LoadSlot for the step's output slot) and step references with field accessors
/// like `$steps.done.result` (creating an AccessorProgram).
pub(super) struct StepSlotReferenceResolver<'a> {
    pub(super) step_slots: &'a [(Box<str>, SlotIdx)],
    pub(super) accessors: &'a mut Vec<AccessorProgram>,
}

impl ExpressionReferenceResolver for StepSlotReferenceResolver<'_> {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let lowered = lower_step_reference(reference, self.step_slots, self.accessors)?;
        Ok(lowered)
    }
}

/// Parses step reference parts from a reference string.
///
/// Returns `Ok((step_id, field_option))` where:
/// - `step_id` is the step identifier (e.g., "build_result" from "$steps.build_result")
/// - `field_option` is `None` for bare references or `Some(field)` for accessors
///
/// # Errors
/// Returns `CompileError::UnknownReferenceRoot` if the reference doesn't start with
/// `$step` or `$steps`, or if it has invalid format.
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

/// Lowers a step reference to a SlotIdx or AccessorIdx.
///
/// For bare step references like `$steps.build_result`, looks up the step name
/// in the step_slots mapping and returns `LoadSlot(slot)`.
///
/// For step references with field accessors like `$steps.build.result`, creates
/// an AccessorProgram with the step's output slot as root and the field as path.
fn lower_step_reference(
    reference: &str,
    step_slots: &[(Box<str>, SlotIdx)],
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let (step_id, field) = parse_step_reference_parts(reference)?;
    let root_slot = resolve_step_slot(reference, step_id, step_slots)?;
    match field {
        Some(field_path) => {
            // Step reference with field accessor: $steps.build.result
            // Create an AccessorProgram for the field path
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
        None => {
            // Bare step reference: $steps.build_result
            // Return LoadSlot for the step's output slot
            Ok(ExprOp::LoadSlot(root_slot))
        }
    }
}

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

/// Parses a field path into PathSegment indices.
///
/// For field accessors like "result" or "data.value", creates numeric index segments.
/// Currently only supports the "result" field which maps to index 0.
/// Other field names are rejected as they require a symbol table.
fn parse_field_path_segments(
    reference: &str,
    field_path: &str,
) -> Result<Vec<PathSegment>, CompileError> {
    let mut segments = Vec::new();
    for segment in field_path.split('.') {
        // Currently only "result" field is supported, mapping to index 0
        // Other fields require a symbol table which doesn't exist yet
        if segment == "result" {
            segments.push(PathSegment::Index(0));
        } else {
            // Field accessors other than "result" require symbol table
            return Err(CompileError::UnsupportedAccessorReference {
                reference: Box::<str>::from(reference),
                root: Box::<str>::from("steps.<id>".to_string()),
                path: Box::<str>::from(field_path),
            });
        }
    }
    Ok(segments)
}
