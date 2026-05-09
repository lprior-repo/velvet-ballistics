#![forbid(unsafe_code)]
//! Gate 8: Accessor path segments are valid symbols.

#![allow(unreachable_pub)]

use crate::{ValidationError, ValidationResult};
use vb_core::limits::MAX_PATH_DEPTH;
use vb_core::workflow::{AccessorProgram, PathSegment, WorkflowParts};

/// Validates that every accessor path segment resolves to a well-formed symbol.
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()> {
    for (acc_index, accessor) in parts.accessors.iter().enumerate() {
        validate_accessor_root(acc_index, accessor, parts.slot_count)?;
        let path_len = accessor.path.len();
        if path_len > MAX_PATH_DEPTH {
            return Err(ValidationError::AccessorPathTooDeep {
                accessor_index: acc_index,
                depth: path_len,
                max: MAX_PATH_DEPTH,
            });
        }
        for (seg_index, segment) in accessor.path.iter().enumerate() {
            match segment {
                PathSegment::Field(sym_id) => {
                    if sym_id.get() >= parts.symbols_count {
                        return Err(ValidationError::AccessorSymbolOutOfBounds {
                            accessor_index: acc_index,
                            segment_index: seg_index,
                            symbol: sym_id.get(),
                            symbols_count: parts.symbols_count,
                        });
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{SlotIdx, StepIdx, SymbolId};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind};

    fn make_parts(slot_count: u16, symbols_count: u32) -> WorkflowParts {
        WorkflowParts {
            name: Box::from("test"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
            nodes: Box::new([CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count,
            entry: StepIdx::new(0),
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    fn accessor(root: u16, path: Vec<PathSegment>) -> AccessorProgram {
        AccessorProgram {
            root: SlotIdx::new(root),
            path: path.into_boxed_slice(),
        }
    }

    // -- Pass cases --

    #[test]
    fn accepts_empty_accessors() {
        let parts = make_parts(1, 0);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_accessor_with_field_segment() {
        let mut parts = make_parts(2, 2);
        parts.accessors = Box::new([accessor(0, vec![PathSegment::Field(SymbolId::new(1))])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_accessor_with_valid_index_segment() {
        let mut parts = make_parts(1, 0);
        parts.accessors = Box::new([accessor(0, vec![PathSegment::Index(0)])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_accessor_with_empty_path() {
        let mut parts = make_parts(1, 0);
        parts.accessors = Box::new([accessor(0, vec![])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_accessor_root_at_boundary() {
        let mut parts = make_parts(2, 0);
        parts.accessors = Box::new([accessor(1, vec![])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_multiple_field_segments() {
        let mut parts = make_parts(1, 3);
        parts.accessors = Box::new([accessor(
            0,
            vec![
                PathSegment::Field(SymbolId::new(0)),
                PathSegment::Field(SymbolId::new(1)),
                PathSegment::Index(5),
            ],
        )]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_multiple_accessors() {
        let mut parts = make_parts(3, 2);
        parts.accessors = Box::new([
            accessor(0, vec![PathSegment::Field(SymbolId::new(1))]),
            accessor(2, vec![PathSegment::Index(10)]),
        ]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_root_out_of_range() {
        let mut parts = make_parts(1, 0);
        parts.accessors = Box::new([accessor(5, vec![])]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSlotOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_sentinel_index_segment() {
        let mut parts = make_parts(1, 0);
        parts.accessors = Box::new([accessor(0, vec![PathSegment::Index(u32::MAX)])]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorPathInvalid { .. })
        ));
    }

    #[test]
    fn rejects_sentinel_index_in_second_accessor() {
        let mut parts = make_parts(2, 2);
        parts.accessors = Box::new([
            accessor(0, vec![PathSegment::Field(SymbolId::new(1))]),
            accessor(1, vec![PathSegment::Index(u32::MAX)]),
        ]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorPathInvalid { .. })
        ));
    }

    #[test]
    fn rejects_root_zero_with_zero_slot_count() {
        let mut parts = make_parts(0, 0);
        parts.accessors = Box::new([accessor(0, vec![])]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSlotOutOfRange { .. })
        ));
    }

    // -- Depth tests --

    #[test]
    fn rejects_path_exceeds_max_depth() {
        use vb_core::limits::MAX_PATH_DEPTH;
        let mut parts = make_parts(1, 0);
        let deep_path: Vec<PathSegment> = (0..=MAX_PATH_DEPTH).map(|i| PathSegment::Index(i as u32)).collect();
        parts.accessors = Box::new([accessor(0, deep_path)]);
        let result = validate_gate_08_accessor_path_segments(&parts);
        assert!(matches!(result, Err(ValidationError::AccessorPathTooDeep { .. })));
        if let Err(ValidationError::AccessorPathTooDeep { depth, max, .. }) = result {
            assert_eq!(depth, MAX_PATH_DEPTH + 1);
            assert_eq!(max, MAX_PATH_DEPTH);
        }
    }

    #[test]
    fn accepts_path_at_max_depth() {
        use vb_core::limits::MAX_PATH_DEPTH;
        let mut parts = make_parts(1, 0);
        let max_path: Vec<PathSegment> = (0..MAX_PATH_DEPTH).map(|i| PathSegment::Index(i as u32)).collect();
        parts.accessors = Box::new([accessor(0, max_path)]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    // -- Symbol bounds tests --

    #[test]
    fn rejects_field_symbol_out_of_bounds() {
        let mut parts = make_parts(1, 2);
        parts.accessors = Box::new([accessor(0, vec![PathSegment::Field(SymbolId::new(2))])]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSymbolOutOfBounds { symbol: 2, symbols_count: 2, .. })
        ));
    }

    #[test]
    fn accepts_field_symbol_at_max_valid() {
        let mut parts = make_parts(1, 3);
        parts.accessors = Box::new([accessor(0, vec![PathSegment::Field(SymbolId::new(2))])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn rejects_field_symbol_in_second_accessor() {
        let mut parts = make_parts(2, 1);
        parts.accessors = Box::new([
            accessor(0, vec![PathSegment::Field(SymbolId::new(0))]),
            accessor(1, vec![PathSegment::Field(SymbolId::new(1))]),
        ]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSymbolOutOfBounds { accessor_index: 1, segment_index: 0, .. })
        ));
    }
}
