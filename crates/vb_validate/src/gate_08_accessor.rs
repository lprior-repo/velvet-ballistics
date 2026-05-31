#![forbid(unsafe_code)]
//! Gate 8: Accessor path segments are valid symbols.

#![allow(unreachable_pub)]

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{AccessorProgram, PathSegment, WorkflowParts};

/// Validates that every accessor path segment resolves to a well-formed symbol.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    for (acc_index, accessor) in parts.accessors.iter().enumerate() {
        validate_accessor_root(acc_index, accessor, parts.slot_count)?;
        for (seg_index, segment) in accessor.path.iter().enumerate() {
            match segment {
                PathSegment::Field(sym_id) => {
                    validate_field_symbol(acc_index, seg_index, *sym_id, parts.symbols_count)?;
                }
                PathSegment::Index(idx) => validate_index_segment(acc_index, seg_index, *idx)?,
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

fn validate_field_symbol(
    acc_index: usize,
    seg_index: usize,
    symbol: vb_core::ids::SymbolId,
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

#[cfg(test)]
#[cfg(test)]
#[path = "gate_08_accessor/tests.rs"]
mod tests;
