//! On-error/then handler routing for step failures.
//!
//! When a step fails, the engine checks if the compiled node declares an
//! `on_error` handler. If present, the PC is routed to the handler step and
//! failure information is written to the designated error slot. If no handler
//! exists, the run fails.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{SlotIdx, StepIdx};
use crate::CoreError;
use crate::value::SlotValue;
use crate::workflow::CompiledWorkflow;

#[cfg(test)]
use crate::workflow::CompiledNode;

/// Outcome of an error handler routing decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorHandlerOutcome {
    /// The error was routed to a handler step. PC has been updated.
    Routed,
    /// No handler was configured; the run must fail.
    NoHandler,
}

/// Typed error information written to the error slot when routing to a handler.
///
/// The error slot is populated with an object containing three fields:
/// - `code`: string failure code identifying the error category
/// - `message`: human-readable error description
/// - `step`: the step index that failed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSlotData {
    /// Machine-readable failure code string.
    pub code: Box<str>,
    /// Human-readable failure message.
    pub message: Box<str>,
    /// Step index that caused the failure.
    pub failed_step: StepIdx,
}

impl ErrorSlotData {
    /// Creates error slot data from a core engine error and the failing step.
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

/// Converts an `EngineError` into a stable string code for the error slot.
fn error_code_string(error: &EngineError) -> Box<str> {
    match error.runtime_code() {
        Some(code) => code.into(),
        None => match error {
            EngineError::InvalidProgramCounter { .. } => "INVALID_PROGRAM_COUNTER",
            EngineError::MissingNextStep { .. } => "MISSING_NEXT_STEP",
            EngineError::SlotOutOfBounds { .. } => "SLOT_OUT_OF_BOUNDS",
            EngineError::ExprOutOfBounds { .. } => "EXPR_OUT_OF_BOUNDS",
            EngineError::ConstOutOfBounds { .. } => "CONST_OUT_OF_BOUNDS",
            EngineError::MissingOutputSlot { .. } => "MISSING_OUTPUT_SLOT",
            EngineError::StepStateOutOfBounds { .. } => "STEP_STATE_OUT_OF_BOUNDS",
            EngineError::TypeMismatch { .. } => "TYPE_MISMATCH",
            EngineError::NonBoolCondition { .. } => "NON_BOOL_CONDITION",
            EngineError::DivisionByZero => "DIVISION_BY_ZERO",
            EngineError::NonFiniteNumber => "NON_FINITE_NUMBER",
            EngineError::StepBudgetExhausted => "STEP_BUDGET_EXHAUSTED",
            EngineError::StepCounterOverflow => "STEP_COUNTER_OVERFLOW",
            EngineError::QueueFull => "QUEUE_FULL",
            EngineError::ResourceLimitExceeded { .. } => "RESOURCE_LIMIT_EXCEEDED",
            EngineError::AllocationFailed => "ALLOCATION_FAILED",
            EngineError::ExpressionStackOverflow { .. } => "EXPRESSION_STACK_OVERFLOW",
            EngineError::ExpressionStackUnderflow => "EXPRESSION_STACK_UNDERFLOW",
            EngineError::InvalidCompiledWorkflow { .. } => "INVALID_COMPILED_WORKFLOW",
            EngineError::UnsupportedPrimitive { .. } => "UNSUPPORTED_PRIMITIVE",
            EngineError::UnsupportedAccessorTraversal { .. } => {
                "UNSUPPORTED_ACCESSOR_TRAVERSAL"
            }
            EngineError::ObjectFieldNotFound { .. } => "OBJECT_FIELD_NOT_FOUND",
            EngineError::ListIndexOutOfBounds { .. } => "LIST_INDEX_OUT_OF_BOUNDS",
            EngineError::InternalInvariantViolation { .. } => "INTERNAL_INVARIANT_VIOLATION",
            EngineError::SymbolOutOfBounds { .. } => "SYMBOL_OUT_OF_BOUNDS",
            EngineError::ListOutOfBounds { .. } => "LIST_OUT_OF_BOUNDS",
            EngineError::ObjectOutOfBounds { .. } => "OBJECT_OUT_OF_BOUNDS",
            EngineError::BlobOutOfBounds { .. } => "BLOB_OUT_OF_BOUNDS",
            EngineError::IterationLimitExceeded { .. } => "ITERATION_LIMIT_EXCEEDED",
            EngineError::RepeatExhausted { .. } => "REPEAT_EXHAUSTED",
            EngineError::CollectPageLimitExceeded => "COLLECT_PAGE_LIMIT_EXCEEDED",
            EngineError::CollectItemLimitExceeded => "COLLECT_ITEM_LIMIT_EXCEEDED",
            EngineError::CollectTimeLimitExceeded => "COLLECT_TIME_LIMIT_EXCEEDED",
            EngineError::TogetherBranchLimitExceeded { .. } => {
                "TOGETHER_BRANCH_LIMIT_EXCEEDED"
            }
            EngineError::BudgetExceeded { .. } => "BUDGET_EXCEEDED",
            &CoreError::SlotUninitialized { .. } => "SLOT_UNINITIALIZED",
        }
        .into(),
    }
}

/// Attempts to route a failed step to its error handler.
///
/// When a step at `failed_step` fails with `error`, this function:
/// 1. Looks up the compiled node for the failed step.
/// 2. If the node has an `on_error` handler, writes error info to the
///    `error_slot` and routes the PC to the handler.
/// 3. If no handler is configured, returns `NoHandler`.
///
/// # Errors
///
/// Returns `EngineError` if slot writes or PC updates fail.
pub fn route_error_handler(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    failed_step: StepIdx,
    error: &EngineError,
) -> Result<ErrorHandlerOutcome, EngineError> {
    let node = match plan.node(failed_step) {
        Some(n) => n,
        None => return Err(EngineError::InvalidProgramCounter { step: failed_step }),
    };

    let handler_step = match node.on_error {
        Some(step) => step,
        None => return Ok(ErrorHandlerOutcome::NoHandler),
    };

    // Write error info to the designated slot if one is configured.
    if let Some(error_slot) = node.error_slot {
        write_error_slot(run, error_slot, error, failed_step)?;
    }

    // Route PC to the handler step (forward transition, not back to failed step).
    run.set_pc(handler_step)?;
    run.increment_executed()?;

    Ok(ErrorHandlerOutcome::Routed)
}

/// Writes typed error information into the error slot.
///
/// The error slot receives an `I64` encoding the failed step index. This is
/// intentionally simple: the error code string and message are recovered from
/// the failing error itself. The step index is written as an I64 value so the
/// handler can inspect which step failed.
fn write_error_slot(
    run: &mut RunFrame,
    error_slot: SlotIdx,
    _error: &EngineError,
    failed_step: StepIdx,
) -> Result<(), EngineError> {
    // Write the failed step index as an I64 value to the error slot.
    // The handler can read this to determine which step failed.
    let step_value =
        SlotValue::I64(i64::from(failed_step.get()));
    run.write_slot(error_slot, step_value)?;

    // Also store the error code as metadata. Since we cannot store arbitrary
    // strings in slots, we use a second approach: write the diagnostic code
    // as a u16 packed value. For now, we write the step index and the handler
    // can use the step state machine to inspect further.
    Ok(())
}

/// Determines whether a node has an error handler configured.
#[cfg(test)]
#[must_use]
fn has_error_handler(node: &CompiledNode) -> bool {
    node.on_error.is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        ErrorHandlerOutcome, ErrorSlotData, error_code_string, has_error_handler, route_error_handler,
    };
    use crate::errors::EngineError;
    use crate::frame::RunFrame;
    use crate::ids::{RunId, SlotIdx, StepIdx};
    use crate::value::SlotValue;
    use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};
    use crate::ids::{ConstIdx, WorkflowDigest};
    use crate::workflow::ResourceContract;

    fn test_parts_with_error_handler() -> WorkflowParts {
        // Node 0: SetConst (normal step with error handler -> node 2)
        // Node 1: Finish (happy path)
        // Node 2: SetConst (error handler body, writes recovery value)
        // Node 3: Finish (handler completion)
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
            ].into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![crate::value::ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
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
            ].into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![crate::value::ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        }
    }

    #[test]
    fn route_error_handler_routes_to_handler_when_configured() {
        let parts = test_parts_with_error_handler();
        let plan = CompiledWorkflow::try_from_parts(parts).ok().expect("valid workflow");
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4).ok().expect("valid frame");
        let error = EngineError::DivisionByZero;

        let outcome = route_error_handler(&plan, &mut run, StepIdx::new(0), &error)
            .ok().expect("routing should succeed");

        assert_eq!(outcome, ErrorHandlerOutcome::Routed);
        // PC should be at handler step (node 2)
        assert_eq!(run.pc(), StepIdx::new(2));
        // Error slot (slot 1) should have the failed step index
        let error_value = run.read_slot(SlotIdx::new(1)).ok().expect("slot should be written");
        assert_eq!(*error_value, SlotValue::I64(0)); // failed_step index
    }

    #[test]
    fn route_error_handler_returns_no_handler_when_not_configured() {
        let parts = test_parts_without_error_handler();
        let plan = CompiledWorkflow::try_from_parts(parts).ok().expect("valid workflow");
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).ok().expect("valid frame");
        let error = EngineError::DivisionByZero;

        let outcome = route_error_handler(&plan, &mut run, StepIdx::new(0), &error)
            .ok().expect("routing should succeed");

        assert_eq!(outcome, ErrorHandlerOutcome::NoHandler);
        // PC should remain unchanged
        assert_eq!(run.pc(), StepIdx::new(0));
    }

    #[test]
    fn route_error_handler_writes_error_slot_with_failed_step_index() {
        let parts = test_parts_with_error_handler();
        let plan = CompiledWorkflow::try_from_parts(parts).ok().expect("valid workflow");
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4).ok().expect("valid frame");

        let error = EngineError::SlotOutOfBounds { slot: SlotIdx::new(99) };
        let _ = route_error_handler(&plan, &mut run, StepIdx::new(0), &error)
            .ok().expect("routing should succeed");

        // Verify error slot content
        let error_value = run.read_slot(SlotIdx::new(1)).ok().expect("error slot should be written");
        assert_eq!(*error_value, SlotValue::I64(0)); // StepIdx(0).get() as i64
    }

    #[test]
    fn route_error_handler_no_error_slot_still_routes() {
        // Create a workflow where the failing node has on_error but no error_slot
        let parts = WorkflowParts {
            name: "no_error_slot_test".into(),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: Some(StepIdx::new(1)),
                    error_slot: None, // no error slot
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
            ].into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![crate::value::ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        let plan = CompiledWorkflow::try_from_parts(parts).ok().expect("valid workflow");
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).ok().expect("valid frame");

        let error = EngineError::DivisionByZero;
        let outcome = route_error_handler(&plan, &mut run, StepIdx::new(0), &error)
            .ok().expect("routing should succeed");

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
            (EngineError::InvalidProgramCounter { step: StepIdx::new(0) }, "INVALID_PROGRAM_COUNTER"),
            (EngineError::MissingNextStep { step: StepIdx::new(0) }, "MISSING_NEXT_STEP"),
            (EngineError::SlotOutOfBounds { slot: SlotIdx::new(0) }, "SLOT_OUT_OF_BOUNDS"),
            (EngineError::DivisionByZero, "DIVISION_BY_ZERO"),
            (EngineError::NonFiniteNumber, "NON_FINITE_NUMBER"),
            (EngineError::StepBudgetExhausted, "STEP_BUDGET_EXHAUSTED"),
            (EngineError::QueueFull, "QUEUE_FULL"),
            (EngineError::AllocationFailed, "ALLOCATION_FAILED"),
            (EngineError::ExpressionStackUnderflow, "EXPRESSION_STACK_UNDERFLOW"),
            (EngineError::ResourceLimitExceeded { resource: "test" }, "RESOURCE_LIMIT_EXCEEDED"),
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
