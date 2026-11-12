#![forbid(unsafe_code)]

use crate::{ValidationError, ValidationResult};
use vb_core::ids::SymbolId;
use vb_core::workflow::{AccessorProgram, PathSegment, WorkflowParts};

// ---------------------------------------------------------------------------
// Gate 8: Accessor path segments are valid symbols
// ---------------------------------------------------------------------------

const MAX_ACCESSOR_PATH_DEPTH: usize = 16;

/// Validates that every accessor path segment resolves to a well-formed symbol.
///
/// Gate 8 (budgets): Field segments must use valid symbol IDs (within the
/// interned symbol table range), and index segments must be finite.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    validate_accessor_paths(&parts.accessors, parts.slot_count, parts.symbols_count)
}

fn validate_field_symbol(
    acc_index: usize,
    seg_index: usize,
    symbol: SymbolId,
    symbols_count: u32,
) -> ValidationResult<()> {
    if symbol.get() < symbols_count {
        Ok(())
    } else {
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: acc_index,
            segment_index: seg_index,
            symbol: symbol.get(),
            symbols_count,
        })
    }
}

fn validate_index_segment(acc_index: usize, seg_index: usize, idx: u32) -> ValidationResult<()> {
    if idx == u32::MAX {
        Err(ValidationError::AccessorPathInvalid {
            accessor_index: acc_index,
            segment_index: seg_index,
        })
    } else {
        Ok(())
    }
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

pub(super) fn validate_accessor_paths(
    accessors: &[AccessorProgram],
    slot_count: u16,
    symbols_count: u32,
) -> ValidationResult<()> {
    for (acc_index, accessor) in accessors.iter().enumerate() {
        validate_accessor_root(acc_index, accessor, slot_count)?;
        if accessor.path.len() > MAX_ACCESSOR_PATH_DEPTH {
            return Err(ValidationError::AccessorPathTooDeep {
                accessor_index: acc_index,
                depth: accessor.path.len(),
                max: MAX_ACCESSOR_PATH_DEPTH,
            });
        }
        for (seg_index, segment) in accessor.path.iter().enumerate() {
            match segment {
                PathSegment::Field(sym_id) => {
                    validate_field_symbol(acc_index, seg_index, *sym_id, symbols_count)?;
                }
                PathSegment::Index(idx) => validate_index_segment(acc_index, seg_index, *idx)?,
                // `PathSegment` is `#[non_exhaustive]`; unknown variants
                // are a structural error — fail closed rather than silently ignore.
                _ => {
                    return Err(ValidationError::AccessorPathInvalid {
                        accessor_index: acc_index,
                        segment_index: seg_index,
                    });
                }
            }
        }
    }
    Ok(())
}
