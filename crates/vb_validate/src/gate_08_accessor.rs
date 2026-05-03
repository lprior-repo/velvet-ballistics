//! Gate 8: Accessor path segments are valid symbols.

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{AccessorProgram, PathSegment, WorkflowParts};

/// Validates that every accessor path segment resolves to a well-formed symbol.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    for (acc_index, accessor) in parts.accessors.iter().enumerate() {
        validate_accessor_root(acc_index, accessor, parts.slot_count)?;
        for (seg_index, segment) in accessor.path.iter().enumerate() {
            match segment {
                PathSegment::Field(_sym_id) => {}
                PathSegment::Index(idx) => {
                    if *idx == u32::MAX {
                        return Err(ValidationError::AccessorPathInvalid {
                            accessor_index: acc_index,
                            segment_index: seg_index,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_accessor_root(acc_index: usize, accessor: &AccessorProgram, slot_count: u16) -> ValidationResult<()> {
    if accessor.root.as_usize() >= usize::from(slot_count) {
        return Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: acc_index,
            slot: accessor.root.as_usize(),
            slot_count: usize::from(slot_count),
        });
    }
    Ok(())
}
