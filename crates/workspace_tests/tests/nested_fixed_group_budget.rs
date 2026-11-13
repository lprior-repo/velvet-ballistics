#![forbid(unsafe_code)]

use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind as Kind, CompiledWorkflow, ResourceContract, WorkflowParts, node};
use vb_runtime::engine::{
    EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::engine::types::EvidenceEvent;
use vb_runtime::primitives::collect::CollectStates;

fn nested_fixed_group_plan() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let step = |id: u16, kind: Kind, output: Option<u16>, next: Option<u16>| {
        node(
            StepIdx::new(id),
            kind,
            output.map(vb_core::ids::SlotIdx::new),
            next.map(StepIdx::new),
        )
    };
    let nodes: Box<[CompiledNode]> = Box::new([
        step(
            0,
            Kind::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(8),
            },
            None,
            None,
        ),
        step(
            1,
            Kind::FirstSuccessStart {
                branches: Box::new([StepIdx::new(2), StepIdx::new(4)]),
                join: StepIdx::new(6),
            },
            None,
            None,
        ),
        step(2, Kind::SetConst { value: vb_core::ids::ConstIdx::new(0) }, Some(0), Some(3)),
        step(
            3,
            Kind::ConcurrentLaneEnd {
                owner: StepIdx::new(1),
                result: Some(vb_core::ids::SlotIdx::new(0)),
            },
            None,
            Some(6),
        ),
        step(4, Kind::SetConst { value: vb_core::ids::ConstIdx::new(1) }, Some(0), Some(5)),
        step(
            5,
            Kind::ConcurrentLaneEnd {
                owner: StepIdx::new(1),
                result: Some(vb_core::ids::SlotIdx::new(0)),
            },
            None,
            Some(6),
        ),
        step(
            6,
            Kind::ConcurrentGroupJoin { owner: StepIdx::new(1) },
            Some(1),
            Some(7),
        ),
        step(
            7,
            Kind::ConcurrentLaneEnd {
                owner: StepIdx::new(0),
                result: Some(vb_core::ids::SlotIdx::new(1)),
            },
            None,
            Some(8),
        ),
        step(
            8,
            Kind::ConcurrentGroupJoin { owner: StepIdx::new(0) },
            Some(2),
            Some(9),
        ),
        step(
            9,
            Kind::Finish {
                result: vb_core::ids::SlotIdx::new(2),
            },
            None,
            None,
        ),
    ]);
    let mut parts = WorkflowParts {
        name: "nested_fixed_groups".into(),
        digest: vb_core::ids::WorkflowDigest::from_bytes([3; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([vb_core::value::ConstValue::I64(10), vb_core::value::ConstValue::I64(20)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    };
    let encoded = postcard::to_allocvec(&parts)?;
    parts.digest = vb_core::ids::WorkflowDigest::from_bytes(blake3::hash(&encoded).into());
    Ok(CompiledWorkflow::try_from_concurrent_parts(parts)?)
}

#[test]
fn checked_nested_fixed_group_preserves_budget_exhaustion_signal() -> Result<(), Box<dyn std::error::Error>> {
    for budget_size in [1_u64, 2, 4] {
        assert_nested_budget(budget_size)?;
    }
    Ok(())
}

fn assert_nested_budget(budget_size: u64) -> Result<(), Box<dyn std::error::Error>> {
    let plan = nested_fixed_group_plan()?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, plan.node_count(), plan.slot_count())?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    let mut finished = None;
    for _ in 0..40 {
        let mut budget = StepBudget::new(budget_size);
        let signal = drive_deterministic_full(
            &plan,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &granted,
            1,
        )?;
        if let RuntimeSignal::Finished(SlotValue::List(list)) = signal {
            finished = Some(list);
            break;
        }
        assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    }
    let result = finished.ok_or("nested fixed group did not finish")?;
    assert_eq!(store.list(result)?, &[SlotValue::I64(10)]);
    assert_eq!(run.read_taint(vb_core::ids::SlotIdx::new(2))?, vb_core::value::Taint::Clean);
    assert_eq!(run.step_state(StepIdx::new(1))?, vb_core::StepState::Succeeded);

    let events = evidence.drain();
    let started_steps: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            EvidenceEvent::StepStarted { step } => Some(*step),
            _ => None,
        })
        .collect();
    assert_eq!(
        started_steps,
        vec![
            StepIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            StepIdx::new(3),
            StepIdx::new(6),
            StepIdx::new(7),
            StepIdx::new(0),
            StepIdx::new(8),
            StepIdx::new(9),
        ]
    );
    assert_eq!(count_started(&events, StepIdx::new(1)), 1);
    assert_eq!(count_succeeded(&events, StepIdx::new(1)), 1);
    assert_eq!(count_started(&events, StepIdx::new(2)), 1);
    assert_eq!(count_succeeded(&events, StepIdx::new(2)), 1);
    assert_eq!(count_started(&events, StepIdx::new(4)), 0);
    assert_eq!(count_written(&events, vb_core::ids::SlotIdx::new(0)), 1);
    assert!(events.iter().any(|event| matches!(
        event,
        EvidenceEvent::SlotWritten {
            slot,
            value: SlotValue::I64(10),
            taint: vb_core::value::Taint::Clean,
            ..
        } if *slot == vb_core::ids::SlotIdx::new(0)
    )));
    Ok(())
}

fn count_started(events: &[EvidenceEvent], step: StepIdx) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::StepStarted { step: found } if *found == step))
        .count()
}

fn count_succeeded(events: &[EvidenceEvent], step: StepIdx) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::StepSucceeded { step: found, .. } if *found == step))
        .count()
}

fn count_written(events: &[EvidenceEvent], slot: vb_core::ids::SlotIdx) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, EvidenceEvent::SlotWritten { slot: found, .. } if *found == slot))
        .count()
}

