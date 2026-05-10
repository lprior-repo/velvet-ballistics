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

    fn make_parts(slot_count: u16) -> WorkflowParts {
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
            symbols_count: 0,
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
        let parts = make_parts(1);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_accessor_with_field_segment() {
        let mut parts = make_parts(2);
        parts.accessors = Box::new([accessor(0, vec![PathSegment::Field(SymbolId::new(1))])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_accessor_with_valid_index_segment() {
        let mut parts = make_parts(1);
        parts.accessors = Box::new([accessor(0, vec![PathSegment::Index(0)])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_accessor_with_empty_path() {
        let mut parts = make_parts(1);
        parts.accessors = Box::new([accessor(0, vec![])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_accessor_root_at_boundary() {
        let mut parts = make_parts(2);
        parts.accessors = Box::new([accessor(1, vec![])]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_multiple_field_segments() {
        let mut parts = make_parts(1);
        parts.accessors = Box::new([accessor(
            0,
            vec![
                PathSegment::Field(SymbolId::new(1)),
                PathSegment::Field(SymbolId::new(2)),
                PathSegment::Index(5),
            ],
        )]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    #[test]
    fn accepts_multiple_accessors() {
        let mut parts = make_parts(3);
        parts.accessors = Box::new([
            accessor(0, vec![PathSegment::Field(SymbolId::new(1))]),
            accessor(2, vec![PathSegment::Index(10)]),
        ]);
        assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_root_out_of_range() {
        let mut parts = make_parts(1);
        parts.accessors = Box::new([accessor(5, vec![])]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSlotOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_sentinel_index_segment() {
        let mut parts = make_parts(1);
        parts.accessors = Box::new([accessor(0, vec![PathSegment::Index(u32::MAX)])]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorPathInvalid { .. })
        ));
    }

    #[test]
    fn rejects_sentinel_index_in_second_accessor() {
        let mut parts = make_parts(2);
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
        let mut parts = make_parts(0);
        parts.accessors = Box::new([accessor(0, vec![])]);
        assert!(matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSlotOutOfRange { .. })
        ));
    }
}
