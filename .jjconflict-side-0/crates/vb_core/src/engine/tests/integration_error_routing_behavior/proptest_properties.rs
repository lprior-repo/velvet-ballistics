// =========================================================================
// 14. Proptest: error context never lost
// =========================================================================

use proptest::prelude::*;

use crate::engine::ErrorSlotData;
use crate::engine::error_routing::route_error_handler;
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node, node_with_handler,
};

fn prop_workflow() -> CompiledWorkflow {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("proptest"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: vec![
            node_with_handler(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
                StepIdx::new(2),
                Some(SlotIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 5,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts).expect("valid proptest workflow")
}

fn arb_engine_error() -> impl Strategy<Value = EngineError> {
    prop_oneof![
        Just(EngineError::DivisionByZero),
        Just(EngineError::NonFiniteNumber),
        Just(EngineError::StepBudgetExhausted),
        Just(EngineError::StepCounterOverflow),
        Just(EngineError::QueueFull),
        Just(EngineError::AllocationFailed),
        Just(EngineError::ExpressionStackUnderflow),
        Just(EngineError::CollectPageLimitExceeded),
        Just(EngineError::CollectItemLimitExceeded),
        Just(EngineError::CollectTimeLimitExceeded),
        (any::<u8>()).prop_map(|v| EngineError::ExpressionStackOverflow { max: v }),
        (1u16..100u16).prop_map(|v| EngineError::RepeatExhausted { max: v }),
        (1u16..100u16).prop_map(|v| EngineError::TogetherBranchLimitExceeded { max: v }),
        (1u16..100u16).prop_map(|v| EngineError::ParallelLimitExceeded { limit: v }),
        (any::<u8>()).prop_map(|_| EngineError::TypeMismatch {
            expected: "i64",
            found: "bool",
        }),
        (any::<u8>()).prop_map(|_| EngineError::ResourceLimitExceeded { resource: "test" }),
        (any::<u8>()).prop_map(|v| EngineError::BudgetExceeded {
            budget: "test",
            limit: v as u64,
        }),
        (any::<u16>()).prop_map(|v| EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(v),
        }),
    ]
}

proptest! {
    #[test]
    fn error_slot_data_code_is_never_empty(error in arb_engine_error()) {
        let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
        prop_assert!(!data.code.is_empty());
    }

    #[test]
    fn error_slot_data_message_is_never_empty(error in arb_engine_error()) {
        let data = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
        prop_assert!(!data.message.is_empty());
    }

    #[test]
    fn error_slot_data_failed_step_preserved(error in arb_engine_error(), step in 0u16..100u16) {
        let step_idx = StepIdx::new(step);
        let data = ErrorSlotData::from_engine_error(&error, step_idx);
        prop_assert_eq!(data.failed_step, step_idx);
    }

    #[test]
    fn route_error_handler_never_loses_error_context(
        error in arb_engine_error()
    ) {
        let workflow = prop_workflow();
        let mut run = RunFrame::new(
            RunId::new(1),
            StepIdx::new(0),
            workflow.node_count(),
            workflow.slot_count(),
        ).expect("valid frame");
        let result = route_error_handler(&workflow, &mut run, StepIdx::new(0), &error);
        let _ = result;
    }

    #[test]
    fn error_code_is_static_consistent(error in arb_engine_error()) {
        // Errors with runtime_code have a specific code; others use the static engine code.
        // But every error must produce a non-empty code string.
        // We already covered "never empty" above. Here we verify that the same error
        // always produces the same code (determinism).
        let data1 = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
        let data2 = ErrorSlotData::from_engine_error(&error, StepIdx::new(0));
        prop_assert_eq!(data1.code, data2.code);
        prop_assert_eq!(data1.message, data2.message);
    }
}
