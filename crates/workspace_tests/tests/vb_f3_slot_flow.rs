//! Public compiler, validation, constructor and interpreter slot-flow contracts.
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind as K,
    CompiledWorkflow, InputSlotKind, ResourceContract, WorkflowParts, node,
};
use vb_validate::shared;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "vb_f3_slot_flow/loop_cases.rs"]
mod loop_cases;

fn slot(raw: u16) -> SlotIdx {
    SlotIdx::new(raw)
}
fn step(raw: u16) -> StepIdx {
    StepIdx::new(raw)
}
fn cn(id: u16, kind: K, out: Option<u16>, next: Option<u16>) -> CompiledNode {
    node(step(id), kind, out.map(slot), next.map(step))
}

fn reduction() -> WorkflowParts {
    WorkflowParts {
        name: "slot-flow-reduce".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: vec![
            cn(
                0,
                K::ReduceStart {
                    limit: 65_535,
                    input: slot(0),
                    accumulator: slot(2),
                    initial: ConstIdx::new(0),
                    body: step(1),
                    done: step(5),
                },
                Some(1),
                None,
            ),
            cn(1, K::Copy { source: slot(2) }, Some(3), Some(2)),
            cn(2, K::Copy { source: slot(3) }, Some(4), Some(3)),
            cn(3, K::Copy { source: slot(4) }, Some(2), Some(4)),
            cn(
                4,
                K::ReduceNext {
                    iterator_slot: slot(1),
                    accumulator: slot(2),
                    body: step(1),
                    done: step(5),
                },
                Some(0),
                None,
            ),
            cn(
                5,
                K::ReduceFinish {
                    accumulator: slot(2),
                },
                Some(5),
                Some(6),
            ),
            cn(6, K::Finish { result: slot(5) }, None, None),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 6,
        symbols_count: 0,
        entry: step(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: vec![CompiledInputSlot {
            slot: slot(0),
            kind: InputSlotKind::List,
            constraint: CompiledInputConstraint::required(),
        }]
        .into_boxed_slice(),
    }
}

#[test]
fn initialized_multi_step_reduce_passes_public_gates() -> TestResult {
    let p = reduction();
    shared::validate_gate_13_no_slot_cycles(&p)?;
    shared::validate_gate_14_slot_type_consistency(&p)?;
    shared::validate(&p)?;
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn uninitialized_cyclic_dependency_remains_rejected() -> TestResult {
    let mut p = reduction();
    p.nodes = vec![
        cn(0, K::Copy { source: slot(1) }, Some(2), Some(1)),
        cn(1, K::Copy { source: slot(2) }, Some(1), Some(2)),
        cn(2, K::Finish { result: slot(1) }, None, None),
    ]
    .into_boxed_slice();
    loop_cases::assert_cycle_error(
        shared::validate_gate_13_no_slot_cycles(&p),
        2,
        "slot 2 -> slot 1",
    )?;
    let expected = vb_core::workflow::SlotFlowError {
        step: step(0),
        slot: Some(slot(1)),
        reason: vb_core::workflow::SlotFlowFailure::UninitializedRead,
    };
    loop_cases::assert_validation_flow_error(
        shared::validate_gate_14_slot_type_consistency(&p),
        &expected,
    )?;
    loop_cases::assert_constructor_flow_error(p, &expected)
}

#[test]
fn accepted_reduce_runs_multiple_iterations() -> TestResult {
    use vb_core::capability::CapabilitySet;
    use vb_core::{RunFrame, StepBudget, ValueStore};
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;
    let plan = CompiledWorkflow::try_from_parts(reduction())?;
    let mut run = RunFrame::new(
        RunId::new(1),
        plan.entry(),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![SlotValue::I64(3), SlotValue::I64(7)].into_boxed_slice())?;
    run.write_slot(slot(0), SlotValue::List(list))?;
    let result = drive_deterministic_full(
        &plan,
        &mut run,
        &mut StepBudget::new(64),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )?;
    assert_eq!(result, RuntimeSignal::Finished(SlotValue::I64(1)));
    Ok(())
}

const YAML: &str = "version: velvet-ballistics/v1\nname: slot_flow\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: save_count\n    save:\n      output: saved\n      value: '$input.count + 1'\n  - id: done\n    finish:\n      result: saved\n";

#[test]
fn yaml_declared_numeric_input_reaches_shared_analysis() -> TestResult {
    let plan = vb_compile::compile_workflow(YAML.as_bytes())?;
    shared::validate(&plan.to_parts())?;
    Ok(())
}

#[test]
fn decoded_ir_cannot_erase_yaml_input_initialization_contract() -> TestResult {
    let mut p = vb_compile::compile_workflow(YAML.as_bytes())?.to_parts();
    p.input_slots = Box::new([]);
    let expected = vb_core::workflow::SlotFlowError {
        step: step(0),
        slot: Some(slot(1)),
        reason: vb_core::workflow::SlotFlowFailure::UninitializedRead,
    };
    loop_cases::assert_validation_flow_error(shared::validate(&p), &expected)?;
    loop_cases::assert_constructor_flow_error(p, &expected)
}

#[test]
fn accepted_together_executes_all_ordered_branches() -> TestResult {
    use vb_core::capability::CapabilitySet;
    use vb_core::{RunFrame, StepBudget, ValueStore};
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;
    let mut p = reduction();
    p.slot_count = 2;
    p.input_slots = Box::new([]);
    p.nodes = vec![
        cn(
            0,
            K::TogetherStart {
                branches: vec![step(1), step(3)].into_boxed_slice(),
                join: step(5),
            },
            Some(0),
            None,
        ),
        cn(
            1,
            K::TogetherBranch {
                branch: 0,
                entry: step(2),
                join: step(5),
                accumulator: slot(0),
            },
            None,
            None,
        ),
        cn(
            2,
            K::SetConst {
                value: ConstIdx::new(0),
            },
            Some(1),
            Some(3),
        ),
        cn(
            3,
            K::TogetherBranch {
                branch: 1,
                entry: step(4),
                join: step(5),
                accumulator: slot(0),
            },
            Some(1),
            None,
        ),
        cn(4, K::Copy { source: slot(1) }, Some(1), Some(5)),
        cn(
            5,
            K::TogetherJoin {
                branch_count: 2,
                accumulator: slot(0),
            },
            Some(1),
            Some(6),
        ),
        cn(6, K::Finish { result: slot(1) }, None, None),
    ]
    .into_boxed_slice();
    shared::validate(&p)?;
    let plan = CompiledWorkflow::try_from_parts(p)?;
    let mut run = RunFrame::new(
        RunId::new(1),
        plan.entry(),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();
    let signal = drive_deterministic_full(
        &plan,
        &mut run,
        &mut StepBudget::new(64),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )?;
    let RuntimeSignal::Finished(SlotValue::List(list)) = signal else {
        return Err(format!("expected list, got {signal:?}").into());
    };
    assert_eq!(store.list(list)?, &[SlotValue::I64(1), SlotValue::I64(1)]);
    Ok(())
}
