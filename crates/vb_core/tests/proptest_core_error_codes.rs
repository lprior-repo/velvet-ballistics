//! Property test: Every CoreError::diagnostic_code() returns the correct
//! documented constant for its variant (all 40 variants).
//!
//! PO-002 / PS-002: Error code stability — diagnostic_code correct for all CoreError variants.
//!
//! Each variant's expected code is the const defined on CoreError (e.g.,
//! INVALID_PROGRAM_COUNTER_CODE = 0x1001).

use vb_core::DiagnosticCode;
use vb_core::errors::CoreError;
use vb_core::ids::{
    BlobId, ConstIdx, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId,
};

#[test]
fn invalid_program_counter_returns_correct_code() {
    let err = CoreError::InvalidProgramCounter {
        step: StepIdx::new(5),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1001),
        "INVALID_PROGRAM_COUNTER_CODE = 0x1001"
    );
}

#[test]
fn missing_next_step_returns_correct_code() {
    let err = CoreError::MissingNextStep {
        step: StepIdx::new(3),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1002),
        "MISSING_NEXT_STEP_CODE = 0x1002"
    );
}

#[test]
fn slot_out_of_bounds_returns_correct_code() {
    let err = CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(99),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1011),
        "SLOT_OUT_OF_BOUNDS_CODE = 0x1011"
    );
}

#[test]
fn slot_uninitialized_returns_correct_code() {
    let err = CoreError::SlotUninitialized {
        slot: SlotIdx::new(3),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1012),
        "SLOT_UNINITIALIZED_CODE = 0x1012"
    );
}

#[test]
fn expr_out_of_bounds_returns_correct_code() {
    let err = CoreError::ExprOutOfBounds {
        expr: ExprIdx::new(7),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1015),
        "EXPR_OUT_OF_BOUNDS_CODE = 0x1015"
    );
}

#[test]
fn const_out_of_bounds_returns_correct_code() {
    let err = CoreError::ConstOutOfBounds {
        index: ConstIdx::new(12),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1013),
        "CONST_OUT_OF_BOUNDS_CODE = 0x1013"
    );
}

#[test]
fn missing_output_slot_returns_correct_code() {
    let err = CoreError::MissingOutputSlot {
        step: StepIdx::new(4),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1305),
        "MISSING_OUTPUT_SLOT_CODE = 0x1305"
    );
}

#[test]
fn step_state_out_of_bounds_returns_correct_code() {
    let err = CoreError::StepStateOutOfBounds {
        step: StepIdx::new(5),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1306),
        "STEP_STATE_OUT_OF_BOUNDS_CODE = 0x1306"
    );
}

#[test]
fn type_mismatch_returns_correct_code() {
    let err = CoreError::TypeMismatch {
        expected: "u64",
        found: "string",
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1101),
        "TYPE_MISMATCH_CODE = 0x1101"
    );
}

#[test]
fn non_bool_condition_returns_correct_code() {
    let err = CoreError::NonBoolCondition {
        slot: SlotIdx::new(1),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1104),
        "NON_BOOL_CONDITION_CODE = 0x1104"
    );
}

#[test]
fn non_finite_number_returns_correct_code() {
    let err = CoreError::NonFiniteNumber;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1102),
        "NON_FINITE_NUMBER_CODE = 0x1102"
    );
}

#[test]
fn division_by_zero_returns_correct_code() {
    let err = CoreError::DivisionByZero;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1103),
        "DIVISION_BY_ZERO_CODE = 0x1103"
    );
}

#[test]
fn step_budget_exhausted_returns_correct_code() {
    let err = CoreError::StepBudgetExhausted;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1201),
        "STEP_BUDGET_EXHAUSTED_CODE = 0x1201"
    );
}

#[test]
fn step_counter_overflow_returns_correct_code() {
    let err = CoreError::StepCounterOverflow;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1202),
        "STEP_COUNTER_OVERFLOW_CODE = 0x1202"
    );
}

#[test]
fn queue_full_returns_correct_code() {
    let err = CoreError::QueueFull;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1301),
        "QUEUE_FULL_CODE = 0x1301"
    );
}

#[test]
fn resource_limit_exceeded_returns_correct_code() {
    let err = CoreError::ResourceLimitExceeded { resource: "cpu" };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1302),
        "RESOURCE_LIMIT_EXCEEDED_CODE = 0x1302"
    );
}

#[test]
fn allocation_failed_returns_correct_code() {
    let err = CoreError::AllocationFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1303),
        "ALLOCATION_FAILED_CODE = 0x1303"
    );
}

#[test]
fn expression_stack_overflow_returns_correct_code() {
    let err = CoreError::ExpressionStackOverflow { max: 64 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1304),
        "EXPRESSION_STACK_OVERFLOW_CODE = 0x1304"
    );
}

#[test]
fn expression_stack_underflow_returns_correct_code() {
    let err = CoreError::ExpressionStackUnderflow;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x130B),
        "EXPRESSION_STACK_UNDERFLOW_CODE = 0x130B"
    );
}

#[test]
fn invalid_compiled_workflow_returns_correct_code() {
    let err = CoreError::InvalidCompiledWorkflow { reason: "test" };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1307),
        "INVALID_COMPILED_WORKFLOW_CODE = 0x1307"
    );
}

#[test]
fn unsupported_primitive_returns_correct_code() {
    let err = CoreError::UnsupportedPrimitive { primitive: "op" };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1308),
        "UNSUPPORTED_PRIMITIVE_CODE = 0x1308"
    );
}

#[test]
fn unsupported_accessor_traversal_returns_correct_code() {
    let err = CoreError::UnsupportedAccessorTraversal {
        segment: "field",
        found: "map",
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x130A),
        "UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE = 0x130A"
    );
}

#[test]
fn object_field_not_found_returns_correct_code() {
    let err = CoreError::ObjectFieldNotFound {
        field: SymbolId::new(0),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x130C),
        "OBJECT_FIELD_NOT_FOUND_CODE = 0x130C"
    );
}

#[test]
fn list_index_out_of_bounds_returns_correct_code() {
    let err = CoreError::ListIndexOutOfBounds { index: 999 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x130D),
        "LIST_INDEX_OUT_OF_BOUNDS_CODE = 0x130D"
    );
}

#[test]
fn internal_invariant_violation_returns_correct_code() {
    let err = CoreError::InternalInvariantViolation { reason: "test" };
    // CV-105: relocated from 0x1309 to 0x1601 (Internal owns 0x16xx).
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1601),
        "INTERNAL_INVARIANT_CODE = 0x1601"
    );
}

#[test]
fn symbol_out_of_bounds_returns_correct_code() {
    let err = CoreError::SymbolOutOfBounds {
        symbol: SymbolId::new(0),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1311),
        "SYMBOL_OUT_OF_BOUNDS_CODE = 0x1311"
    );
}

#[test]
fn list_out_of_bounds_returns_correct_code() {
    let err = CoreError::ListOutOfBounds {
        list: ListId::new(0),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1312),
        "LIST_OUT_OF_BOUNDS_CODE = 0x1312"
    );
}

#[test]
fn object_out_of_bounds_returns_correct_code() {
    let err = CoreError::ObjectOutOfBounds {
        object: ObjectId::new(0),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1313),
        "OBJECT_OUT_OF_BOUNDS_CODE = 0x1313"
    );
}

#[test]
fn blob_out_of_bounds_returns_correct_code() {
    let err = CoreError::BlobOutOfBounds {
        blob: BlobId::new(0),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1314),
        "BLOB_OUT_OF_BOUNDS_CODE = 0x1314"
    );
}

#[test]
fn iteration_limit_exceeded_returns_correct_code() {
    let err = CoreError::IterationLimitExceeded { resource: "cpu" };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1401),
        "ITERATION_LIMIT_EXCEEDED_CODE = 0x1401"
    );
}

#[test]
fn repeat_exhausted_returns_correct_code() {
    let err = CoreError::RepeatExhausted { max: 3 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1402),
        "REPEAT_EXHAUSTED_CODE = 0x1402"
    );
}

#[test]
fn collect_page_limit_exceeded_returns_correct_code() {
    let err = CoreError::CollectPageLimitExceeded;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1403),
        "COLLECT_PAGE_LIMIT_CODE = 0x1403"
    );
}

#[test]
fn collect_item_limit_exceeded_returns_correct_code() {
    let err = CoreError::CollectItemLimitExceeded;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1404),
        "COLLECT_ITEM_LIMIT_CODE = 0x1404"
    );
}

#[test]
fn collect_time_limit_exceeded_returns_correct_code() {
    let err = CoreError::CollectTimeLimitExceeded;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1407),
        "COLLECT_TIME_LIMIT_CODE = 0x1407"
    );
}

#[test]
fn together_branch_limit_exceeded_returns_correct_code() {
    let err = CoreError::TogetherBranchLimitExceeded { max: 1 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1405),
        "TOGETHER_BRANCH_LIMIT_CODE = 0x1405"
    );
}

#[test]
fn parallel_limit_exceeded_returns_correct_code() {
    let err = CoreError::ParallelLimitExceeded { limit: 1 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1408),
        "PARALLEL_LIMIT_EXCEEDED_CODE = 0x1408"
    );
}

#[test]
fn capability_denied_returns_correct_code() {
    let err = CoreError::CapabilityDenied {
        action: vb_core::ids::ActionId::new(1),
        required: vb_core::capability::Capability::new(
            Box::from("required"),
            vb_core::ids::ActionId::new(2),
        ),
        granted: vb_core::capability::CapabilitySet::empty(),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1409),
        "CAPABILITY_DENIED_CODE = 0x1409"
    );
}

#[test]
fn budget_exceeded_returns_correct_code() {
    let err = CoreError::BudgetExceeded {
        budget: "cpu",
        limit: 100,
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1406),
        "BUDGET_EXCEEDED_CODE = 0x1406"
    );
}

#[test]
fn budget_parse_returns_correct_code() {
    let err = CoreError::BudgetParse { reason: "bad" };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x140A),
        "BUDGET_PARSE_CODE = 0x140A"
    );
}

#[test]
fn collect_page_order_violation_returns_correct_code() {
    let err = CoreError::CollectPageOrderViolation {
        kind: vb_core::errors::CollectPageOrderViolationKind::OutOfOrder,
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(3),
        expected_page: ListId::new(2),
        observed_page: ListId::new(3),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x140B),
        "COLLECT_PAGE_ORDER_VIOLATION_CODE = 0x140B"
    );
}

#[test]
fn collect_extra_hydration_failed_returns_correct_code() {
    let err = CoreError::CollectExtraHydrationFailed {
        kind: vb_core::errors::CollectExtraHydrationFailureKind::EmptyExtra,
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(3),
        event_seq: Some(vb_core::ids::EventSeq::new(1)),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x140C),
        "COLLECT_EXTRA_HYDRATION_FAILED_CODE = 0x140C"
    );
}

#[test]
fn collect_evidence_capacity_exceeded_returns_correct_code() {
    let err = CoreError::CollectEvidenceCapacityExceeded {
        run_id: RunId::new(1),
        slot: SlotIdx::new(3),
        capacity: 10,
        len: 11,
        required: "extra slots",
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x140D),
        "COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE = 0x140D"
    );
}

#[test]
fn lifecycle_storage_unavailable_returns_correct_code() {
    let err = CoreError::LifecycleStorageUnavailable {
        code: DiagnosticCode::new(0x1501),
        context: "test".into(),
        timestamp: chrono::Utc::now(),
        bead_id: Some(RunId::new(1)),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1501),
        "LIFECYCLE_STORAGE_UNAVAILABLE_CODE = 0x1501"
    );
}

#[test]
fn lifecycle_duplicate_request_returns_correct_code() {
    let err = CoreError::LifecycleDuplicateRequest {
        code: DiagnosticCode::new(0x1502),
        context: "test".into(),
        timestamp: chrono::Utc::now(),
        bead_id: Some(RunId::new(1)),
        command: Some("run"),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1502),
        "LIFECYCLE_DUPLICATE_REQUEST_CODE = 0x1502"
    );
}

#[test]
fn lifecycle_stale_request_returns_correct_code() {
    let err = CoreError::LifecycleStaleRequest {
        code: DiagnosticCode::new(0x1503),
        context: "test".into(),
        timestamp: chrono::Utc::now(),
        bead_id: Some(RunId::new(1)),
        command: Some("cancel"),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1503),
        "LIFECYCLE_STALE_REQUEST_CODE = 0x1503"
    );
}

#[test]
fn lifecycle_invalid_transition_returns_correct_code() {
    let err = CoreError::LifecycleInvalidTransition {
        code: DiagnosticCode::new(0x1504),
        context: "test".into(),
        timestamp: chrono::Utc::now(),
        bead_id: Some(RunId::new(1)),
        command: Some("run"),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1504),
        "LIFECYCLE_INVALID_TRANSITION_CODE = 0x1504"
    );
}

#[test]
fn journal_write_failure_returns_correct_code() {
    let err = CoreError::JournalWriteFailure {
        code: DiagnosticCode::new(0x1505),
        context: "test".into(),
        timestamp: chrono::Utc::now(),
        bead_id: Some(RunId::new(1)),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1505),
        "JOURNAL_WRITE_FAILURE_CODE = 0x1505"
    );
}

#[test]
fn replay_corruption_returns_correct_code() {
    let err = CoreError::ReplayCorruption {
        code: DiagnosticCode::new(0x1506),
        context: "test".into(),
        timestamp: chrono::Utc::now(),
        bead_id: Some(RunId::new(1)),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x1506),
        "REPLAY_CORRUPTION_CODE = 0x1506"
    );
}
