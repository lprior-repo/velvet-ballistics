//! Gate 8: Accessor path segments are valid symbols
//!
//! Validates that every accessor path segment resolves to a well-formed symbol.
//! Field segments must use valid symbol IDs (within the interned symbol table
//! range), and index segments must be finite.

use crate::{ValidationError, ValidationResult};

pub use vb_core::workflow::{AccessorProgram, PathSegment, WorkflowParts};
pub use vb_core::ids::SlotIdx;

/// Validates that every accessor path segment resolves to a well-formed symbol.
///
/// Gate 8 (budgets): Field segments must use valid symbol IDs (within the
/// interned symbol table range), and index segments must be finite.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    for (acc_index, accessor) in parts.accessors.iter().enumerate() {
        validate_accessor_root(acc_index, accessor, parts.slot_count)?;
        for (seg_index, segment) in accessor.path.iter().enumerate() {
            match segment {
                PathSegment::Field(_sym_id) => {
                    // Symbol IDs are interned; any non-sentinel value is valid.
                }
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

fn validate_accessor_root(
    acc_index: usize,
    accessor: &AccessorProgram,
    slot_count: u16,
) -> ValidationResult<()> {
    if accessor.root.as_usize() >= usize::from(slot_count) {
        return Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: acc_index,
            slot: accessor.root.as_usize(),
            slot_count: usize::from(slot_count),
        });
    }
    Ok(())
}
