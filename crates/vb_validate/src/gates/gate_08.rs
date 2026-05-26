#![forbid(unsafe_code)]
//! Gate 8: Accessor path validation

use crate::{ValidationError, ValidationResult};
use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{
use vb_core::span::Span;
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};

const MAX_ACCESSOR_PATH_DEPTH: usize = 16;

pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    for (acc_index, accessor) in parts.accessors.iter().enumerate() {
        validate_accessor_root(acc_index, accessor, parts.slot_count)?;
        if accessor.path.len() > MAX_ACCESSOR_PATH_DEPTH {
            return Err(ValidationError::AccessorPathTooDeep {
                accessor_index: acc_index,
                depth: accessor.path.len(),
                max: MAX_ACCESSOR_PATH_DEPTH,
             span: Span::ZERO});
        }
        for (seg_index, segment) in accessor.path.iter().enumerate() {
            match segment {
                PathSegment::Field(sym_id) => {
                    validate_field_symbol(acc_index, seg_index, *sym_id, parts.symbols_count)?;
                }
                PathSegment::Index(idx) => validate_index_segment(acc_index, seg_index, *idx)?,
            }
        }
    }
    Ok(())
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
         span: Span::ZERO})
    }
}

fn validate_index_segment(acc_index: usize, seg_index: usize, idx: u32) -> ValidationResult<()> {
    if idx == u32::MAX {
        Err(ValidationError::AccessorPathInvalid {
            accessor_index: acc_index,
            segment_index: seg_index,
         span: Span::ZERO})
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
         span: Span::ZERO});
    }
    Ok(())
}
