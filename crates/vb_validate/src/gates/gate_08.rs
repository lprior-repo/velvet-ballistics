#![forbid(unsafe_code)]
//! Gate 8: Accessor path segments are valid symbols.
//!
//! Gate 8 (budgets): Field segments must use valid symbol IDs (within the
//! interned symbol table range), and index segments must be finite.

use crate::{ValidationError, ValidationResult};
use vb_core::ids::SymbolId;
use vb_core::workflow::{AccessorProgram, PathSegment, WorkflowParts};

/// Maximum accessor path depth allowed by the v1 protocol.
const MAX_ACCESSOR_PATH_DEPTH: usize = 16;

/// Validates that every accessor path segment resolves to a well-formed symbol.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    let mut acc_index = 0usize;
    while acc_index < parts.accessors.len() {
        let accessor = match parts.accessors.get(acc_index) {
            Some(value) => value,
            None => return Err(accessor_path_invalid(acc_index, 0)),
        };

        validate_accessor_root(acc_index, accessor, parts.slot_count)?;
        validate_accessor_path(acc_index, accessor, parts.symbols_count)?;
        acc_index = next_accessor_index(acc_index)?;
    }
    Ok(())
}

fn validate_accessor_path(
    acc_index: usize,
    accessor: &AccessorProgram,
    symbols_count: u32,
) -> ValidationResult<()> {
    let path_len = accessor.path.len();
    if path_len > MAX_ACCESSOR_PATH_DEPTH {
        return Err(ValidationError::AccessorPathTooDeep {
            accessor_index: acc_index,
            depth: path_len,
            max: MAX_ACCESSOR_PATH_DEPTH,
        });
    }

    let mut seg_index = 0usize;
    while seg_index < path_len {
        let segment = match accessor.path.get(seg_index) {
            Some(value) => value,
            None => return Err(accessor_path_invalid(acc_index, seg_index)),
        };
        validate_path_segment(acc_index, seg_index, *segment, symbols_count)?;
        seg_index = next_segment_index(acc_index, seg_index)?;
    }
    Ok(())
}

fn validate_path_segment(
    acc_index: usize,
    seg_index: usize,
    segment: PathSegment,
    symbols_count: u32,
) -> ValidationResult<()> {
    match segment {
        PathSegment::Field(sym_id) => {
            validate_field_symbol(acc_index, seg_index, sym_id, symbols_count)
        }
        PathSegment::Index(idx) => validate_index_segment(acc_index, seg_index, idx),
        // `PathSegment` is `#[non_exhaustive]`; unknown variants are a
        // structural error — fail closed rather than silently ignore.
        _ => Err(accessor_path_invalid(acc_index, seg_index)),
    }
}

fn next_accessor_index(acc_index: usize) -> ValidationResult<usize> {
    match acc_index.checked_add(1) {
        Some(next) => Ok(next),
        None => Err(accessor_path_invalid(acc_index, 0)),
    }
}

fn next_segment_index(acc_index: usize, seg_index: usize) -> ValidationResult<usize> {
    match seg_index.checked_add(1) {
        Some(next) => Ok(next),
        None => Err(accessor_path_invalid(acc_index, seg_index)),
    }
}

fn accessor_path_invalid(acc_index: usize, seg_index: usize) -> ValidationError {
    ValidationError::AccessorPathInvalid {
        accessor_index: acc_index,
        segment_index: seg_index,
    }
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
