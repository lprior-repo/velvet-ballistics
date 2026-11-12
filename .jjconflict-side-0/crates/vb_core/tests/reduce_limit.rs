#![forbid(unsafe_code)]

use vb_core::workflow::{CompiledInputConstraint, CompiledInputSlot, ContractResource, node};
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, ConstIdx, ConstValue, InputSlotKind, ResourceContract,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parts(limit: u32) -> WorkflowParts {
    WorkflowParts {
        name: "reduce-limit".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::List,
            constraint: CompiledInputConstraint::required(),
        }]),
        nodes: Box::new([
            node(
                StepIdx::new(0),
                K::ReduceStart {
                    limit,
                    input: SlotIdx::new(0),
                    accumulator: SlotIdx::new(1),
                    initial: ConstIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            node(StepIdx::new(1), K::Nop, None, Some(StepIdx::new(2))),
            node(
                StepIdx::new(2),
                K::ReduceNext {
                    iterator_slot: SlotIdx::new(2),
                    accumulator: SlotIdx::new(1),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(0)),
                None,
            ),
            node(
                StepIdx::new(3),
                K::ReduceFinish {
                    accumulator: SlotIdx::new(1),
                },
                Some(SlotIdx::new(3)),
                Some(StepIdx::new(4)),
            ),
            node(
                StepIdx::new(4),
                K::Finish {
                    result: SlotIdx::new(3),
                },
                None,
                None,
            ),
        ]),
        constants: Box::new([ConstValue::I64(0)]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

#[test]
fn reduce_limits_above_the_hard_list_cap_are_rejected() -> TestResult {
    let max = u32::try_from(vb_core::limits::MAX_LIST_ITEMS_PER_VALUE)?;
    for limit in [max.checked_add(1).ok_or("limit overflow")?, u32::MAX] {
        assert_eq!(
            CompiledWorkflow::try_from_parts(parts(limit)).map(|_| ()),
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::ListItems
            })
        );
    }
    Ok(())
}

#[test]
fn zero_and_hard_maximum_reduce_limits_remain_valid() -> TestResult {
    let max = u32::try_from(vb_core::limits::MAX_LIST_ITEMS_PER_VALUE)?;
    for limit in [0, 2, max] {
        CompiledWorkflow::try_from_parts(parts(limit))?;
    }
    Ok(())
}

#[test]
fn reduce_budget_uses_the_compiled_limit() -> TestResult {
    let small = CompiledWorkflow::try_from_parts(parts(2))?;
    let larger = CompiledWorkflow::try_from_parts(parts(3))?;
    let budget = |workflow: &CompiledWorkflow| {
        let parts = workflow.to_parts();
        vb_core::WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)
    };
    assert_eq!(budget(&small)?.max_total_steps, 7);
    assert_eq!(budget(&larger)?.max_total_steps, 9);
    Ok(())
}
