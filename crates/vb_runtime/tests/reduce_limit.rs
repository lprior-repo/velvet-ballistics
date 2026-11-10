#![forbid(unsafe_code)]

use vb_core::workflow::node;
use vb_core::{
    CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, CoreError, EngineSignal,
    ResourceContract, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, Taint, ValueStore,
    WorkflowDigest, WorkflowParts,
};
use vb_runtime::primitives::reduce::reduce_start;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn plan() -> Result<CompiledWorkflow, vb_core::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "reduce-limit".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        nodes: Box::new([node(StepIdx::new(0), CompiledNodeKind::Nop, None, None)]),
        constants: Box::new([ConstValue::I64(42)]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
}

#[test]
fn oversized_reduce_input_rejects_before_frame_or_store_mutation() -> TestResult {
    let plan = plan()?;
    for (limit, values) in [
        (0, vec![SlotValue::I64(1)]),
        (
            2,
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
        ),
    ] {
        for accumulator in [SlotIdx::new(0), SlotIdx::new(1)] {
            let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 4)?;
            let mut store = ValueStore::new();
            let list = store.insert_list(values.clone().into_boxed_slice())?;
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Secret)?;
            run.write_slot_with_taint(
                SlotIdx::new(1),
                SlotValue::I64(99),
                Taint::DerivedFromSecret,
            )?;
            run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(17), Taint::Secret)?;
            let before_run = run.clone();
            let before_store = store.clone();
            assert_eq!(
                reduce_start(
                    &plan,
                    &mut run,
                    &mut store,
                    SlotIdx::new(0),
                    accumulator,
                    ConstIdx::new(0),
                    limit,
                    StepIdx::new(1),
                    StepIdx::new(2),
                    Some(SlotIdx::new(2))
                ),
                Err(CoreError::IterationLimitExceeded {
                    resource: "reduce_limit"
                })
            );
            assert_eq!(run, before_run);
            assert_eq!(store, before_store);
        }
    }
    Ok(())
}

#[test]
fn reduce_accepts_exact_limit_and_empty_zero_limit() -> TestResult {
    let plan = plan()?;
    for (values, limit, target) in [
        (vec![], 0, StepIdx::new(2)),
        (
            vec![SlotValue::I64(1), SlotValue::I64(2)],
            2,
            StepIdx::new(1),
        ),
    ] {
        let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 4)?;
        let mut store = ValueStore::new();
        let list = store.insert_list(values.into_boxed_slice())?;
        run.write_slot(SlotIdx::new(0), SlotValue::List(list))?;
        assert_eq!(
            reduce_start(
                &plan,
                &mut run,
                &mut store,
                SlotIdx::new(0),
                SlotIdx::new(1),
                ConstIdx::new(0),
                limit,
                StepIdx::new(1),
                StepIdx::new(2),
                Some(SlotIdx::new(2))
            ),
            Ok(EngineSignal::Continue)
        );
        assert_eq!(run.pc(), target);
        assert_eq!(*run.read_slot(SlotIdx::new(1))?, SlotValue::I64(42));
    }
    Ok(())
}
