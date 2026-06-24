#![forbid(unsafe_code)]
//! On-error/then handler routing for step failures.
//!
//! When a step fails, the engine checks if the compiled node declares an
//! `on_error` handler. If present, the PC is routed to the handler step and
//! the failed step index is written to the designated error slot as an `I64`.
//! The `ErrorSlotData` struct captures full diagnostic details (code, message,
//! step) for logging and auditing, but only the step index is written to the
//! slot because the `SlotValue` type system does not support arbitrary strings.
//! If no handler exists, the run fails.

use crate::CoreError;
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::workflow::CompiledWorkflow;

#[cfg(test)]
use crate::workflow::CompiledNode;

/// Outcome of an error handler routing decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorHandlerOutcome {
    /// The error was routed to a handler step. PC has been updated.
    Routed,
    /// No handler was configured; the run must fail.
    NoHandler,
}

/// Diagnostic error information captured when routing to an error handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSlotData {
    pub code: Box<str>,
    pub message: Box<str>,
    pub failed_step: StepIdx,
}

impl ErrorSlotData {
    #[must_use]
    pub fn from_engine_error(error: &EngineError, failed_step: StepIdx) -> Self {
        let code = error_code_string(error);
        let message = format!("{error}");
        Self {
            code,
            message: message.into_boxed_str(),
            failed_step,
        }
    }
}

/// Returns the static string code for engine errors that don't have a runtime code.
fn engine_error_static_code(error: &EngineError) -> &'static str {
    use CoreError::*;
    match error {
        InvalidProgramCounter { .. } => "INVALID_PROGRAM_COUNTER",
        MissingNextStep { .. } => "MISSING_NEXT_STEP",
        MissingOutputSlot { .. } => "MISSING_OUTPUT_SLOT",
        InvalidCompiledWorkflow { .. } => "INVALID_COMPILED_WORKFLOW",
        SlotOutOfBounds { .. } => "SLOT_OUT_OF_BOUNDS",
        ExprOutOfBounds { .. } => "EXPR_OUT_OF_BOUNDS",
        ConstOutOfBounds { .. } => "CONST_OUT_OF_BOUNDS",
        StepStateOutOfBounds { .. } => "STEP_STATE_OUT_OF_BOUNDS",
        ListIndexOutOfBounds { .. } => "LIST_INDEX_OUT_OF_BOUNDS",
        SymbolOutOfBounds { .. } => "SYMBOL_OUT_OF_BOUNDS",
        ListOutOfBounds { .. } => "LIST_OUT_OF_BOUNDS",
        ObjectOutOfBounds { .. } => "OBJECT_OUT_OF_BOUNDS",
        BlobOutOfBounds { .. } => "BLOB_OUT_OF_BOUNDS",
        TypeMismatch { .. } => "TYPE_MISMATCH",
        NonBoolCondition { .. } => "NON_BOOL_CONDITION",
        DivisionByZero => "DIVISION_BY_ZERO",
        NonFiniteNumber => "NON_FINITE_NUMBER",
        StepBudgetExhausted => "STEP_BUDGET_EXHAUSTED",
        StepCounterOverflow => "STEP_COUNTER_OVERFLOW",
        QueueFull => "QUEUE_FULL",
        AllocationFailed => "ALLOCATION_FAILED",
        ExpressionStackOverflow { .. } => "EXPRESSION_STACK_OVERFLOW",
        ExpressionStackUnderflow => "EXPRESSION_STACK_UNDERFLOW",
        UnsupportedPrimitive { .. } => "UNSUPPORTED_PRIMITIVE",
        UnsupportedAccessorTraversal { .. } => "UNSUPPORTED_ACCESSOR_TRAVERSAL",
        ObjectFieldNotFound { .. } => "OBJECT_FIELD_NOT_FOUND",
        InternalInvariantViolation { .. } => "INTERNAL_INVARIANT_VIOLATION",
        IterationLimitExceeded { .. } => "ITERATION_LIMIT_EXCEEDED",
        RepeatExhausted { .. } => "REPEAT_EXHAUSTED",
        InvalidRepeatState { .. } => "INVALID_REPEAT_STATE",
        CollectPageLimitExceeded => "COLLECT_PAGE_LIMIT_EXCEEDED",
        CollectItemLimitExceeded => "COLLECT_ITEM_LIMIT_EXCEEDED",
        CollectTimeLimitExceeded => "COLLECT_TIME_LIMIT_EXCEEDED",
        TogetherBranchLimitExceeded { .. } => "TOGETHER_BRANCH_LIMIT_EXCEEDED",
        ParallelLimitExceeded { .. } => "PARALLEL_LIMIT_EXCEEDED",
        BudgetExceeded { .. } => "BUDGET_EXCEEDED",
        BudgetParse { .. } => "BUDGET_PARSE",
        ResourceLimitExceeded { .. } => "RESOURCE_LIMIT_EXCEEDED",
        &CoreError::SlotUninitialized { .. } => "SLOT_UNINITIALIZED",
        CapabilityDenied { .. } => "CAPABILITY_DENIED",
        CollectPageOrderViolation { .. } => "COLLECT_PAGE_ORDER_VIOLATION",
        CollectExtraHydrationFailed { .. } => "COLLECT_EXTRA_HYDRATION_FAILED",
        CollectEvidenceCapacityExceeded { .. } => "COLLECT_EVIDENCE_CAPACITY_EXCEEDED",
        EvidenceCapacityExceeded { .. } => "EVIDENCE_CAPACITY_EXCEEDED",
        LifecycleStorageUnavailable { .. } => "LIFECYCLE_STORAGE_UNAVAILABLE",
        LifecycleDuplicateRequest { .. } => "LIFECYCLE_DUPLICATE_REQUEST",
        LifecycleStaleRequest { .. } => "LIFECYCLE_STALE_REQUEST",
        LifecycleInvalidTransition { .. } => "LIFECYCLE_INVALID_TRANSITION",
        JournalWriteFailure { .. } => "JOURNAL_WRITE_FAILURE",
        ReplayCorruption { .. } => "REPLAY_CORRUPTION",
        InvalidSpan { .. } => "INVALID_SPAN",
    }
}

fn error_code_string(error: &EngineError) -> Box<str> {
    error
        .runtime_code()
        .map(|code| code.into())
        .unwrap_or_else(|| engine_error_static_code(error).into())
}

pub fn route_error_handler(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    failed_step: StepIdx,
    error: &EngineError,
) -> Result<ErrorHandlerOutcome, EngineError> {
    let node = plan
        .node(failed_step)
        .ok_or(EngineError::InvalidProgramCounter { step: failed_step })?;
    let Some(handler_step) = node.on_error else {
        return Ok(ErrorHandlerOutcome::NoHandler);
    };

    let error_slot = plan
        .error_handler_for_body(failed_step)
        .and_then(|eh| eh.error_slot)
        .or(node.error_slot);

    if let Some(error_slot) = error_slot {
        write_error_slot(run, error_slot, error, failed_step)?;
    }

    advance_to_handler(run, handler_step)?;
    Ok(ErrorHandlerOutcome::Routed)
}

#[inline]
fn advance_to_handler(run: &mut RunFrame, handler_step: StepIdx) -> Result<(), EngineError> {
    run.set_pc(handler_step)?;
    run.increment_executed()?;
    Ok(())
}

fn write_error_slot(
    run: &mut RunFrame,
    error_slot: SlotIdx,
    _error: &EngineError,
    failed_step: StepIdx,
) -> Result<(), EngineError> {
    run.write_slot(error_slot, SlotValue::I64(i64::from(failed_step.get())))?;
    Ok(())
}

#[cfg(test)]
#[must_use]
fn has_error_handler(node: &CompiledNode) -> bool {
    node.on_error.is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        ErrorHandlerOutcome, ErrorSlotData, error_code_string, has_error_handler,
        route_error_handler,
    };
    use crate::errors::EngineError;
    use crate::frame::RunFrame;
    use crate::ids::{ConstIdx, WorkflowDigest};
    use crate::ids::{RunId, SlotIdx, StepIdx};
    use crate::value::SlotValue;
    use crate::workflow::ResourceContract;
    use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

    fn test_parts_with_error_handler() -> WorkflowParts {
        WorkflowParts {
            name: "error_handler_test".into(),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: Some(StepIdx::new(2)),
                    error_slot: Some(SlotIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![crate::value::ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    fn test_parts_without_error_handler() -> WorkflowParts {
        WorkflowParts {
            name: "no_handler_test".into(),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![crate::value::ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    #[test]
    fn route_error_handler_routes_to_handler_when_configured() {
        let parts = test_parts_with_error_handler();
        let plan = CompiledWorkflow::try_from_parts(parts)
            .ok()
            .expect("valid workflow");
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4)
            .ok()
            .expect("valid frame");
        let error = EngineError::DivisionByZero;

        let outcome = route_error_handler(&plan, &mut run, StepIdx::new(0), &error)
            .ok()
            .expect("routing should succeed");

        assert_eq!(outcome, ErrorHandlerOutcome::Routed);
        assert_eq!(run.pc(), StepIdx::new(2));
        let error_value = run
            .read_slot(SlotIdx::new(1))
            .ok()
            .expect("slot should be written");
        assert_eq!(*error_value, SlotValue::I64(0));
    }

    #[test]
    fn route_error_handler_returns_no_handler_when_not_configured() {
        let parts = test_parts_without_error_handler();
        let plan = CompiledWorkflow::try_from_parts(parts)
            .ok()
            .expect("valid workflow");
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2)
            .ok()
            .expect("valid frame");
        let error = EngineError::DivisionByZero;

        let outcome = route_error_handler(&plan, &mut run, StepIdx::new(0), &error)
            .ok()
            .expect("routing should succeed");

        assert_eq!(outcome, ErrorHandlerOutcome::NoHandler);
        assert_eq!(run.pc(), StepIdx::new(0));
    }

    #[test]
    fn route_error_handler_writes_error_slot_with_failed_step_index() {
        let parts = test_parts_with_error_handler();
        let plan = CompiledWorkflow::try_from_parts(parts)
            .ok()
            .expect("valid workflow");
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4)
            .ok()
            .expect("valid frame");
        let error = EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(99),
        };

        let _ = route_error_handler(&plan, &mut run, StepIdx::new(0), &error)
            .ok()
            .expect("routing should succeed");

        let error_value = run
            .read_slot(SlotIdx::new(1))
            .ok()
            .expect("error slot should be written");
        assert_eq!(*error_value, SlotValue::I64(0));
    }

    #[test]
    fn route_error_handler_no_error_slot_still_routes() {
        let parts = WorkflowParts {
            name: "no_error_slot_test".into(),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: Some(StepIdx::new(1)),
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![crate::value::ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let plan = CompiledWorkflow::try_from_parts(parts)
            .ok()
            .expect("valid workflow");
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2)
            .ok()
            .expect("valid frame");
        let error = EngineError::DivisionByZero;

        let outcome = route_error_handler(&plan, &mut run, StepIdx::new(0), &error)
            .ok()
            .expect("routing should succeed");

        assert_eq!(outcome, ErrorHandlerOutcome::Routed);
        assert_eq!(run.pc(), StepIdx::new(1));
    }

    #[test]
    fn error_slot_data_from_division_by_zero() {
        let error = EngineError::DivisionByZero;
        let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(5));

        assert_eq!(&*data.code, "DIVISION_BY_ZERO");
        assert_eq!(data.failed_step, StepIdx::new(5));
        assert!(!data.message.is_empty());
    }

    #[test]
    fn error_slot_data_from_type_mismatch() {
        let error = EngineError::TypeMismatch {
            expected: "i64",
            found: "bool",
        };
        let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(3));

        assert_eq!(&*data.code, "INPUT_TYPE_MISMATCH");
        assert_eq!(data.failed_step, StepIdx::new(3));
        assert!(data.message.contains("i64"));
        assert!(data.message.contains("bool"));
    }

    #[test]
    fn error_code_string_covers_all_variants() {
        let cases: Vec<(EngineError, &str)> = vec![
            (
                EngineError::InvalidProgramCounter {
                    step: StepIdx::new(0),
                },
                "INVALID_PROGRAM_COUNTER",
            ),
            (
                EngineError::MissingNextStep {
                    step: StepIdx::new(0),
                },
                "MISSING_NEXT_STEP",
            ),
            (
                EngineError::SlotOutOfBounds {
                    slot: SlotIdx::new(0),
                },
                "SLOT_OUT_OF_BOUNDS",
            ),
            (EngineError::DivisionByZero, "DIVISION_BY_ZERO"),
            (EngineError::NonFiniteNumber, "NON_FINITE_NUMBER"),
            (EngineError::StepBudgetExhausted, "STEP_BUDGET_EXHAUSTED"),
            (EngineError::QueueFull, "QUEUE_FULL"),
            (EngineError::AllocationFailed, "ALLOCATION_FAILED"),
            (
                EngineError::ExpressionStackUnderflow,
                "EXPRESSION_STACK_UNDERFLOW",
            ),
            (
                EngineError::ResourceLimitExceeded { resource: "test" },
                "RESOURCE_LIMIT_EXCEEDED",
            ),
        ];

        for (error, expected_code) in cases {
            let code = error_code_string(&error);
            assert_eq!(&*code, expected_code, "error code mismatch for {error:?}");
        }
    }

    #[test]
    fn has_error_handler_returns_true_when_configured() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: Some(StepIdx::new(1)),
            error_slot: Some(SlotIdx::new(0)),
            kind: CompiledNodeKind::Nop,
        };
        assert!(has_error_handler(&node));
    }

    #[test]
    fn has_error_handler_returns_false_when_not_configured() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        assert!(!has_error_handler(&node));
    }
}
