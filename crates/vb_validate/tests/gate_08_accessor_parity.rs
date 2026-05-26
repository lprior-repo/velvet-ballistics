#![forbid(unsafe_code)]

use vb_core::ids::{SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::span::Span;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, PathSegment,
    ResourceContract, WorkflowError, WorkflowParts,
};
use vb_validate::ValidationError;
use vb_validate::gates;

fn finish_node() -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }
}

fn accessor_allocating_boxed_path(root: u16, path: Box<[PathSegment]>) -> AccessorProgram {
    AccessorProgram {
        root: SlotIdx::new(root),
        path,
    }
}

fn workflow_parts_with_accessors(
    slot_count: u16,
    symbols_count: u32,
    accessors: Box<[AccessorProgram]>,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("gate_08_accessor_parity"),
        digest: WorkflowDigest::from_bytes([8u8; 32]),
        nodes: Box::new([finish_node()]),
        expressions: Box::new([]),
        accessors,
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn workflow_parts_with_single_segment(symbols_count: u32, segment: PathSegment) -> WorkflowParts {
    workflow_parts_with_accessors(
        1,
        symbols_count,
        Box::new([accessor_allocating_boxed_path(0, Box::new([segment]))]),
    )
}

#[test]
fn aggregate_gate_08_accepts_empty_accessor_paths_when_root_is_valid() {
    let parts = workflow_parts_with_accessors(
        1,
        0,
        Box::new([accessor_allocating_boxed_path(0, Box::new([]))]),
    );

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Ok(())
    );
}

#[test]
fn aggregate_gate_08_accepts_empty_accessor_collection() {
    let parts = workflow_parts_with_accessors(1, 0, Box::new([]));

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Ok(())
    );
}

#[test]
fn aggregate_gate_08_rejects_accessor_root_greater_than_slot_count() {
    let parts = workflow_parts_with_accessors(
        1,
        1,
        Box::new([accessor_allocating_boxed_path(5, Box::new([]))]),
    );

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 1,
            span: Span::ZERO
        })
    );
}

#[test]
fn aggregate_gate_08_reports_invalid_field_segment_coordinates() {
    let parts = workflow_parts_with_accessors(
        2,
        2,
        Box::new([
            accessor_allocating_boxed_path(0, Box::new([PathSegment::Index(0)])),
            accessor_allocating_boxed_path(
                1,
                Box::new([PathSegment::Index(0), PathSegment::Field(SymbolId::new(2))]),
            ),
        ]),
    );

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 1,
            segment_index: 1,
            symbol: 2,
            symbols_count: 2,
            span: Span::ZERO
        })
    );
}

#[test]
fn aggregate_gate_08_checks_root_before_path_segments() {
    let parts = workflow_parts_with_accessors(
        1,
        1,
        Box::new([accessor_allocating_boxed_path(
            5,
            Box::new([PathSegment::Field(SymbolId::new(1))]),
        )]),
    );

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 1,
            span: Span::ZERO
        })
    );
}

#[test]
fn aggregate_gate_08_accepts_field_zero_when_symbols_count_is_one() {
    let parts = workflow_parts_with_single_segment(1, PathSegment::Field(SymbolId::new(0)));

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Ok(())
    );
}

#[test]
fn aggregate_gate_08_accepts_field_at_symbols_count_minus_one() {
    let parts = workflow_parts_with_single_segment(4, PathSegment::Field(SymbolId::new(3)));

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Ok(())
    );
}

#[test]
fn aggregate_gate_08_rejects_field_equal_to_symbols_count() {
    let parts = workflow_parts_with_single_segment(4, PathSegment::Field(SymbolId::new(4)));

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 4,
            symbols_count: 4,
            span: Span::ZERO
        })
    );
}

#[test]
fn aggregate_gate_08_rejects_field_above_symbols_count() {
    let parts = workflow_parts_with_single_segment(4, PathSegment::Field(SymbolId::new(5)));

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 5,
            symbols_count: 4,
            span: Span::ZERO
        })
    );
}

#[test]
fn validate_gate_08_matches_core_workflow_for_valid_field_boundaries() {
    let parts = workflow_parts_with_single_segment(1, PathSegment::Field(SymbolId::new(0)));
    let core_parts = parts.clone();

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Ok(())
    );
    assert_eq!(
        CompiledWorkflow::try_from_parts(core_parts).map(|_| ()),
        Ok(())
    );
}

#[test]
fn validate_gate_08_matches_core_workflow_for_invalid_field_boundaries() {
    let parts = workflow_parts_with_single_segment(1, PathSegment::Field(SymbolId::new(1)));
    let core_parts = parts.clone();

    assert_eq!(
        gates::validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 1,
            symbols_count: 1,
            span: Span::ZERO
        })
    );
    assert_eq!(
        CompiledWorkflow::try_from_parts(core_parts).map(|_| ()),
        Err(WorkflowError::SymbolOutOfBounds {
            symbol: SymbolId::new(1),
        })
    );
}
