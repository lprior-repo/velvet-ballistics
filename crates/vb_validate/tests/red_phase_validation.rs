#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::panic
)]
// vb_validate Integration Tests — RED PHASE
// These tests prove the pipeline validate function returns correct ValidationError
// variants for symbol bounds and resource contract violations.
#![forbid(unsafe_code)]

use vb_core::ids::{SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, PathSegment, ResourceContract, WorkflowParts,
};
use vb_validate::shared::{ValidationPipeline, validate};

// ---------------------------------------------------------------------------
// Helper: minimal WorkflowParts factory
// ---------------------------------------------------------------------------

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
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

#[allow(dead_code)]
fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: if index == 0 {
            Some(StepIdx::new(1))
        } else {
            None
        },
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

#[allow(dead_code)]
fn nop_node(index: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: Some(StepIdx::new(index.saturating_add(1))),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

fn accessor(root: u16, path: Vec<PathSegment>) -> AccessorProgram {
    AccessorProgram {
        root: SlotIdx::new(root),
        path: path.into_boxed_slice(),
    }
}

// ---------------------------------------------------------------------------
// RED PHASE: These tests prove accessor symbol out-of-bounds → AccessorSymbolOutOfBounds
// ---------------------------------------------------------------------------

/// RED PHASE TEST: Pipeline must return AccessorSymbolOutOfBounds when an
/// accessor's Field segment uses a symbol ID >= symbols_count.
#[test]
fn pipeline_validate_rejects_accessor_symbol_out_of_bounds() {
    let mut parts = make_parts(1, 5);
    // SymbolId::new(7) >= symbols_count (5) → must be rejected
    parts.accessors = Box::new([accessor(0, vec![PathSegment::Field(SymbolId::new(7))])]);

    let result = validate(&parts);
    assert_eq!(
        result,
        Err(vb_validate::ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 7,
            symbols_count: 5,
        })
    );
}

/// RED PHASE TEST: Pipeline must return AccessorSymbolOutOfBounds for second accessor.
#[test]
fn pipeline_validate_rejects_second_accessor_symbol_out_of_bounds() {
    let mut parts = make_parts(2, 3);
    parts.accessors = Box::new([
        accessor(0, vec![PathSegment::Field(SymbolId::new(0))]), // valid
        accessor(1, vec![PathSegment::Field(SymbolId::new(99))]), // out of bounds
    ]);

    let result = validate(&parts);
    assert_eq!(
        result,
        Err(vb_validate::ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 1,
            segment_index: 0,
            symbol: 99,
            symbols_count: 3,
        })
    );
}

/// RED PHASE TEST: Pipeline must return AccessorSymbolOutOfBounds for deep path segment.
#[test]
fn pipeline_validate_rejects_deep_accessor_segment_symbol_out_of_bounds() {
    let mut parts = make_parts(1, 5);
    // First segment valid (symbol 2 < 5), second segment invalid (symbol 10 >= 5)
    parts.accessors = Box::new([accessor(
        0,
        vec![
            PathSegment::Field(SymbolId::new(2)),
            PathSegment::Field(SymbolId::new(10)),
        ],
    )]);

    let result = validate(&parts);
    assert_eq!(
        result,
        Err(vb_validate::ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 1,
            symbol: 10,
            symbols_count: 5,
        })
    );
}

/// RED PHASE TEST: Pipeline must return Ok when all accessor symbols are in bounds.
#[test]
fn pipeline_validate_accepts_accessor_symbols_in_bounds() {
    let mut parts = make_parts(2, 10);
    parts.accessors = Box::new([
        accessor(0, vec![PathSegment::Field(SymbolId::new(0))]),
        accessor(1, vec![PathSegment::Field(SymbolId::new(9))]),
    ]);

    assert!(
        validate(&parts).is_ok(),
        "expected Ok for valid accessor symbols"
    );
}

/// RED PHASE TEST: Pipeline must return AccessorSlotOutOfRange when accessor root >= slot_count.
#[test]
fn pipeline_validate_rejects_accessor_root_out_of_range() {
    let mut parts = make_parts(1, 0);
    parts.accessors = Box::new([accessor(5, vec![])]); // root=5, slot_count=1

    let result = validate(&parts);
    assert_eq!(
        result,
        Err(vb_validate::ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 1,
        })
    );
}

// ---------------------------------------------------------------------------
// RED PHASE: Pipeline short-circuit order tests
// ---------------------------------------------------------------------------

/// RED PHASE TEST: Pipeline must run gates in order (7, 8, 9, 10, 11, 13, 14, 15)
/// and short-circuit on first error.
#[test]
fn pipeline_validate_runs_gates_in_order_short_circuits() {
    // Create parts that would fail multiple gates:
    // - Gate 7: empty expressions, should pass
    // - Gate 8: accessor root out of range (slot 99, slot_count=1)
    // - Gate 9: slot reference out of range
    let mut parts = make_parts(1, 0);
    parts.accessors = Box::new([accessor(99, vec![])]); // slot out of range
    parts.slot_count = 1;

    let result = validate(&parts);
    // Gate 8 runs before Gate 9, so AccessorSlotOutOfRange is the first error
    assert!(
        matches!(
            &result.unwrap_err(),
            vb_validate::ValidationError::AccessorSlotOutOfRange { .. }
        ),
        "expected first error from gate 8 (accessor root), got different error"
    );
}

/// RED PHASE TEST: Pipeline with all gates disabled returns Ok for any parts.
#[test]
fn pipeline_validate_all_gates_disabled_accepts_anything() {
    let mut parts = make_parts(1, 0);
    parts.accessors = Box::new([accessor(99, vec![])]); // would fail gate 8

    let pipeline = ValidationPipeline::no_gates();
    // With all gates disabled, validate should return Ok (no gates run)
    assert!(
        pipeline.validate(&parts).is_ok(),
        "pipeline with no gates should return Ok even for malformed parts"
    );
}

// ---------------------------------------------------------------------------
// RED PHASE: Slot reference tests (Gate 9)
// ---------------------------------------------------------------------------

/// RED PHASE TEST: Pipeline must return SlotReferenceOutOfRange for node output >= slot_count.
#[test]
fn pipeline_validate_rejects_slot_reference_out_of_range() {
    let parts = WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(99)), // slot 99, slot_count=1
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1, // only slot 0 is valid
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = validate(&parts);
    assert_eq!(
        result,
        Err(vb_validate::ValidationError::SlotReferenceOutOfRange {
            slot: 99,
            slot_count: 1,
            context: "node 0".into(),
        })
    );
}

// ---------------------------------------------------------------------------
// RED PHASE: Expression stack depth tests (Gate 7)
// ---------------------------------------------------------------------------

/// RED PHASE TEST: Pipeline must return ExpressionStackExceeded when declared > limit.
#[test]
fn pipeline_validate_rejects_expression_stack_exceeds_limit() {
    let mut parts = make_parts(1, 0);
    let contract = vb_core::workflow::ResourceContract {
        max_expr_stack: 128, // exceeds MAX_EXPR_STACK_DEPTH (64)
        ..ResourceContract::DEFAULT
    };
    parts.resource_contract = contract;

    let result = validate(&parts);
    assert_eq!(
        result,
        Err(vb_validate::ValidationError::ExpressionStackExceeded {
            declared: 128,
            limit: 64,
        })
    );
}

// ---------------------------------------------------------------------------
// RED PHASE: Pipeline is deterministic
// ---------------------------------------------------------------------------

/// RED PHASE TEST: Same parts must produce same error every time.
#[test]
fn pipeline_validate_is_deterministic() {
    let mut parts = make_parts(1, 5);
    parts.accessors = Box::new([accessor(0, vec![PathSegment::Field(SymbolId::new(99))])]);

    let result1 = validate(&parts);
    let result2 = validate(&parts);
    let result3 = validate(&parts);

    assert_eq!(result1, result2, "validate must be deterministic");
    assert_eq!(result2, result3, "validate must be deterministic");
}

// ---------------------------------------------------------------------------
// RED PHASE: Resource contract — vb_validate pipeline does NOT produce WorkflowError
// ---------------------------------------------------------------------------

/// RED PHASE TEST: vb_validate pipeline must NOT produce WorkflowError variants.
/// Resource contract errors are vb_core errors; vb_validate operates at the
/// ValidationError level.
#[test]
fn pipeline_validate_does_not_produce_workflow_error() {
    let mut parts = make_parts(1, 0);
    parts.resource_contract = vb_core::workflow::ResourceContract {
        max_steps: 0, // will cause vb_core validation to fail
        ..ResourceContract::DEFAULT
    };

    // The vb_validate pipeline itself returns ValidationError, not WorkflowError.
    // Note: This test passes the parts to vb_validate pipeline only (not vb_core).
    // vb_core's own validation (validate_parts) would produce WorkflowError.
    // This test confirms the separation of error systems.
    let result = validate(&parts);
    // vb_validate accepts parts with any resource contract; resource contract
    // validation happens at vb_core level, not vb_validate level.
    // So this should pass vb_validate (only gate 8 checks accessors, which are empty).
    assert!(
        result.is_ok(),
        "vb_validate pipeline should not reject parts based on resource contract; that is vb_core's job"
    );
}
