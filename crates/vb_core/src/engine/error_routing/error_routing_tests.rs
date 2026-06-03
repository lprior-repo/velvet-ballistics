#![forbid(unsafe_code)]
//! Tests for error routing.

use crate::workflow::CompiledNode;

use super::{ErrorHandlerOutcome, ErrorSlotData, error_code_string, route_error_handler};

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
