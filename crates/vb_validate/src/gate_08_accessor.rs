#![forbid(unsafe_code)]
//! Gate 8: Accessor path segments are valid symbols.

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{AccessorProgram, PathSegment, WorkflowParts};

/// Maximum accessor path depth allowed by the v1 protocol.
///
/// The public Gate 8 wrapper and focused checks share this production bound.
const MAX_ACCESSOR_PATH_DEPTH: usize = 16;

/// Validates that every accessor path segment resolves to a well-formed symbol.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    validate_accessor_programs(&parts.accessors, parts.slot_count, parts.symbols_count)
}

pub(crate) fn validate_accessor_programs(
    accessors: &[AccessorProgram],
    slot_count: u16,
    symbols_count: u32,
) -> ValidationResult<()> {
    accessors
        .iter()
        .enumerate()
        .try_for_each(|(acc_index, accessor)| {
            validate_accessor(acc_index, accessor, slot_count, symbols_count)
        })
}

fn validate_accessor(
    acc_index: usize,
    accessor: &AccessorProgram,
    slot_count: u16,
    symbols_count: u32,
) -> ValidationResult<()> {
    validate_accessor_root(acc_index, accessor, slot_count)?;
    if accessor.path.len() > MAX_ACCESSOR_PATH_DEPTH {
        return Err(ValidationError::AccessorPathTooDeep {
            accessor_index: acc_index,
            depth: accessor.path.len(),
            max: MAX_ACCESSOR_PATH_DEPTH,
        });
    }
    accessor
        .path
        .iter()
        .enumerate()
        .try_for_each(|(seg_index, segment)| {
            validate_segment(acc_index, seg_index, segment, symbols_count)
        })
}

fn validate_segment(
    acc_index: usize,
    seg_index: usize,
    segment: &PathSegment,
    symbols_count: u32,
) -> ValidationResult<()> {
    match segment {
        PathSegment::Field(sym_id) => {
            validate_field_symbol(acc_index, seg_index, *sym_id, symbols_count)
        }
        PathSegment::Index(idx) => validate_index_segment(acc_index, seg_index, *idx),
        _ => Err(ValidationError::AccessorPathInvalid {
            accessor_index: acc_index,
            segment_index: seg_index,
        }),
    }
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
#[path = "gate_08_accessor/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gate_08_accessor/projection_tests.rs"]
mod projection_tests;
