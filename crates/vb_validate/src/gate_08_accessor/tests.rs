use super::*;
use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{CompiledNode, CompiledNodeKind};

fn workflow_parts_with_accessors(
    slot_count: u16,
    symbols_count: u32,
    accessors: Box<[AccessorProgram]>,
) -> WorkflowParts {
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
        accessors,
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn accessor_allocating_boxed_path(root: u16, path: Box<[PathSegment]>) -> AccessorProgram {
    AccessorProgram {
        root: SlotIdx::new(root),
        path,
    }
}

fn one_accessor_parts_with_segment(
    slot_count: u16,
    symbols_count: u32,
    root: u16,
    segment: PathSegment,
) -> WorkflowParts {
    workflow_parts_with_accessors(
        slot_count,
        symbols_count,
        Box::new([accessor_allocating_boxed_path(root, Box::new([segment]))]),
    )
}

fn expected_single_field_result(symbols_count: u32, field_id: u32) -> ValidationResult<()> {
    if field_id < symbols_count {
        Ok(())
    } else {
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: field_id,
            symbols_count,
        })
    }
}

fn expected_single_index_result(index: u32) -> ValidationResult<()> {
    if index == u32::MAX {
        Err(ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 0,
        })
    } else {
        Ok(())
    }
}

fn expected_single_root_result(slot_count: u16, root: u16) -> ValidationResult<()> {
    if usize::from(root) < usize::from(slot_count) {
        Ok(())
    } else {
        Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: usize::from(root),
            slot_count: usize::from(slot_count),
        })
    }
}

fn checked_above_bound_field_id(symbols_count: u32) -> u32 {
    match symbols_count.checked_add(1) {
        Some(value) => value,
        None => symbols_count,
    }
}

#[test]
fn gate_08_accepts_field_symbol_zero_when_symbols_count_is_one() {
    let parts = one_accessor_parts_with_segment(1, 1, 0, PathSegment::Field(SymbolId::new(0)));

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_accepts_field_symbol_at_symbols_count_minus_one() {
    let parts = one_accessor_parts_with_segment(1, 3, 0, PathSegment::Field(SymbolId::new(2)));

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_accepts_field_symbol_at_larger_symbols_count_minus_one() {
    let parts = one_accessor_parts_with_segment(1, 8, 0, PathSegment::Field(SymbolId::new(7)));

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_rejects_field_symbol_equal_to_symbols_count() {
    let parts = one_accessor_parts_with_segment(1, 3, 0, PathSegment::Field(SymbolId::new(3)));

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 3,
            symbols_count: 3,
        })
    );
}

#[test]
fn gate_08_rejects_field_symbol_above_symbols_count() {
    let parts = one_accessor_parts_with_segment(1, 3, 0, PathSegment::Field(SymbolId::new(4)));

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 4,
            symbols_count: 3,
        })
    );
}

#[test]
fn gate_08_rejects_field_segment_when_symbols_count_is_zero() {
    let parts = one_accessor_parts_with_segment(1, 0, 0, PathSegment::Field(SymbolId::new(0)));

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 0,
            symbols_count: 0,
        })
    );
}

#[test]
fn gate_08_accepts_index_zero() {
    let parts = one_accessor_parts_with_segment(1, 0, 0, PathSegment::Index(0));

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_accepts_index_u32_max_minus_one() {
    let parts =
        one_accessor_parts_with_segment(1, 0, 0, PathSegment::Index(u32::MAX.saturating_sub(1)));

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_rejects_sentinel_index_segment() {
    let parts = one_accessor_parts_with_segment(1, 0, 0, PathSegment::Index(u32::MAX));

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 0,
        })
    );
}

#[test]
fn gate_08_accepts_empty_accessor_paths() {
    let parts = workflow_parts_with_accessors(
        1,
        0,
        Box::new([accessor_allocating_boxed_path(0, Box::new([]))]),
    );

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_accepts_empty_accessor_collection() {
    let parts = workflow_parts_with_accessors(1, 0, Box::new([]));

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_accepts_accessor_root_at_slot_count_minus_one() {
    let parts = workflow_parts_with_accessors(
        3,
        1,
        Box::new([accessor_allocating_boxed_path(
            2,
            Box::new([PathSegment::Field(SymbolId::new(0))]),
        )]),
    );

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_rejects_accessor_root_equal_to_slot_count() {
    let parts = workflow_parts_with_accessors(
        3,
        1,
        Box::new([accessor_allocating_boxed_path(3, Box::new([]))]),
    );

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 3,
            slot_count: 3,
        })
    );
}

#[test]
fn gate_08_rejects_accessor_root_greater_than_slot_count() {
    let parts = workflow_parts_with_accessors(
        1,
        1,
        Box::new([accessor_allocating_boxed_path(5, Box::new([]))]),
    );

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 1,
        })
    );
}

#[test]
fn gate_08_reports_invalid_field_segment_coordinates() {
    let parts = workflow_parts_with_accessors(
        2,
        2,
        Box::new([
            accessor_allocating_boxed_path(0, Box::new([PathSegment::Field(SymbolId::new(1))])),
            accessor_allocating_boxed_path(
                1,
                Box::new([PathSegment::Index(0), PathSegment::Field(SymbolId::new(2))]),
            ),
        ]),
    );

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 1,
            segment_index: 1,
            symbol: 2,
            symbols_count: 2,
        })
    );
}

#[test]
fn gate_08_checks_root_before_path_segments() {
    let parts = workflow_parts_with_accessors(
        1,
        1,
        Box::new([accessor_allocating_boxed_path(
            5,
            Box::new([PathSegment::Field(SymbolId::new(1))]),
        )]),
    );

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 1,
        })
    );
}

#[test]
fn focused_and_aggregate_gate_08_accept_field_zero_when_symbols_count_is_one() {
    let parts = one_accessor_parts_with_segment(1, 1, 0, PathSegment::Field(SymbolId::new(0)));

    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
    assert_eq!(
        crate::gates::validate_gate_08_accessor_path_segments(&parts),
        Ok(())
    );
}

#[test]
fn focused_and_aggregate_gate_08_reject_field_equal_to_symbols_count() {
    let parts = one_accessor_parts_with_segment(1, 1, 0, PathSegment::Field(SymbolId::new(1)));

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 1,
            symbols_count: 1,
        })
    );
    assert_eq!(
        crate::gates::validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 1,
            symbols_count: 1,
        })
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn proptest_gate_08_accepts_exactly_field_symbols_below_symbols_count(
        symbols_count in 0u32..=16,
        field_id in 0u32..=17,
    ) {
        let parts = one_accessor_parts_with_segment(
            1,
            symbols_count,
            0,
            PathSegment::Field(SymbolId::new(field_id)),
        );
        prop_assert_eq!(validate_gate_08_accessor_path_segments(&parts),
            expected_single_field_result(symbols_count, field_id));
    }

    #[test]
    fn proptest_above_bound_field_fixtures_use_checked_construction(
        symbols_count in 0u32..=16,
    ) {
        let field_id = checked_above_bound_field_id(symbols_count);
        let parts = one_accessor_parts_with_segment(
            1,
            symbols_count,
            0,
            PathSegment::Field(SymbolId::new(field_id)),
        );
        prop_assert_eq!(validate_gate_08_accessor_path_segments(&parts),
            expected_single_field_result(symbols_count, field_id));
    }

    #[test]
    fn proptest_gate_08_accepts_exactly_non_sentinel_index_values(index in any::<u32>()) {
        let parts = one_accessor_parts_with_segment(1, 0, 0, PathSegment::Index(index));
        prop_assert_eq!(validate_gate_08_accessor_path_segments(&parts),
            expected_single_index_result(index));
    }

    #[test]
    fn proptest_gate_08_reports_first_invalid_path_coordinate_deterministically(
        first_valid_field in 0u32..=1,
        invalid_field in 2u32..=8,
    ) {
        let parts = workflow_parts_with_accessors(
            1,
            2,
            Box::new([accessor_allocating_boxed_path(
                0,
                Box::new([
                    PathSegment::Field(SymbolId::new(first_valid_field)),
                    PathSegment::Field(SymbolId::new(invalid_field)),
                    PathSegment::Index(u32::MAX),
                ]),
            )]),
        );
        prop_assert_eq!(validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSymbolOutOfBounds {
                accessor_index: 0,
                segment_index: 1,
                symbol: invalid_field,
                symbols_count: 2,
            }));
    }

    #[test]
    fn proptest_gate_08_reports_first_invalid_accessor_with_root_precedence(
        slot_count in 0u16..=8,
        root in 0u16..=10,
    ) {
        let parts = workflow_parts_with_accessors(
            slot_count,
            1,
            Box::new([accessor_allocating_boxed_path(
                root,
                Box::new([PathSegment::Field(SymbolId::new(0))]),
            )]),
        );
        prop_assert_eq!(validate_gate_08_accessor_path_segments(&parts),
            expected_single_root_result(slot_count, root));
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn verify_field_symbol_bound_is_complete_for_small_values() {
        let symbols_count: u32 = kani::any();
        let field_id: u32 = kani::any();
        kani::assume(symbols_count <= 8);
        kani::assume(field_id <= 8);
        let parts = one_accessor_parts_with_segment(
            1,
            symbols_count,
            0,
            PathSegment::Field(SymbolId::new(field_id)),
        );

        assert_eq!(
            validate_gate_08_accessor_path_segments(&parts),
            expected_single_field_result(symbols_count, field_id)
        );
    }

    #[kani::proof]
    fn verify_root_bound_is_complete_for_small_values() {
        let slot_count: u16 = kani::any();
        let root: u16 = kani::any();
        kani::assume(slot_count <= 8);
        kani::assume(root <= 10);
        let parts = workflow_parts_with_accessors(
            slot_count,
            1,
            Box::new([accessor_allocating_boxed_path(root, Box::new([]))]),
        );

        assert_eq!(
            validate_gate_08_accessor_path_segments(&parts),
            expected_single_root_result(slot_count, root)
        );
    }

    #[kani::proof]
    fn verify_index_sentinel_classification_is_complete() {
        let index: u32 = kani::any();
        let parts = one_accessor_parts_with_segment(1, 0, 0, PathSegment::Index(index));

        assert_eq!(
            validate_gate_08_accessor_path_segments(&parts),
            expected_single_index_result(index)
        );
    }

    #[kani::proof]
    fn verify_root_before_path_error_precedence() {
        let slot_count: u16 = kani::any();
        let root: u16 = kani::any();
        kani::assume(slot_count <= 4);
        kani::assume(root > slot_count);
        kani::assume(root <= 8);
        let parts = workflow_parts_with_accessors(
            slot_count,
            1,
            Box::new([accessor_allocating_boxed_path(
                root,
                Box::new([PathSegment::Index(u32::MAX)]),
            )]),
        );

        assert_eq!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorSlotOutOfRange {
                accessor_index: 0,
                slot: usize::from(root),
                slot_count: usize::from(slot_count),
            })
        );
    }
}
