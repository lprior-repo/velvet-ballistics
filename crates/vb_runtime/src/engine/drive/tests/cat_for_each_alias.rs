#![forbid(unsafe_code)]

use super::common::{cn, fin, mkr, mkwfc};
use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal};
use crate::primitives::collect::CollectStates;
use vb_core::capability::CapabilitySet;
use vb_core::engine::StepBudget;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, WorkflowError};

fn checked<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, String> {
    result.map_err(|error| format!("{error:?}"))
}

fn fixture_parts() -> Result<CompiledWorkflow, String> {
    let nodes = vec![
        cn(
            0,
            Some(2),
            None,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        ),
        cn(
            1,
            Some(1),
            None,
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(2),
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        ),
        fin(2, 1),
        fin(3, 1),
    ];
    mkwfc(nodes, 3, vec![])
}

#[test]
fn forward_distinct_for_each_next_emits_actual_writes() -> Result<(), String> {
    let workflow = fixture_parts()?;
    let original = workflow.to_parts();

    let mut run = mkr(4, 3)?;
    let mut store = ValueStore::new();
    let payload = vec![SlotValue::I64(7), SlotValue::I64(9)];
    let source = checked(store.insert_list(payload.clone().into_boxed_slice()))?;
    checked(run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(source), Taint::Secret))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();

    let r1 = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut StepBudget::new(1),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect,
        &CapabilitySet::empty(),
        1,
    );
    assert_eq!(r1, Ok(RuntimeSignal::StepBudgetExhausted));
    assert_eq!(run.pc(), StepIdx::new(1));
    let setup = evidence.drain();
    assert!(!setup.is_empty());

    let r2 = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut StepBudget::new(1),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect,
        &CapabilitySet::empty(),
        1,
    );
    assert_eq!(r2, Ok(RuntimeSignal::StepBudgetExhausted));
    assert_eq!(run.pc(), StepIdx::new(2));

    assert_eq!(checked(run.read_slot(SlotIdx::new(1)))?, &SlotValue::I64(9));
    assert_eq!(checked(run.read_taint(SlotIdx::new(1)))?, Taint::Secret);
    let SlotValue::List(iter_id) = *checked(run.read_slot(SlotIdx::new(2)))? else {
        return Err("iterator slot is not List".into());
    };
    assert_eq!(checked(run.read_taint(SlotIdx::new(2)))?, Taint::Secret);
    assert_eq!(
        checked(store.list(iter_id))?,
        &[SlotValue::I64(i64::from(source.get())), SlotValue::I64(2)]
    );
    assert_eq!(checked(store.list(source))?, payload.as_slice());
    let events = evidence.drain();
    assert_eq!(
        events,
        vec![
            EvidenceEvent::StepStarted {
                step: StepIdx::new(1)
            },
            EvidenceEvent::SlotWritten {
                slot: SlotIdx::new(1),
                value: SlotValue::I64(9),
                taint: Taint::Secret,
                extra: None
            },
            EvidenceEvent::SlotWritten {
                slot: SlotIdx::new(2),
                value: SlotValue::List(iter_id),
                taint: Taint::Secret,
                extra: None
            },
            EvidenceEvent::StepSucceeded {
                step: StepIdx::new(1),
                output: Some(SlotIdx::new(1))
            },
        ]
    );
    assert_eq!(workflow.to_parts(), original);
    Ok(())
}

#[test]
fn for_each_next_body_done_alias_rejected_at_construction() -> Result<(), String> {
    let mut parts = fixture_parts()?.to_parts();
    let node = parts
        .nodes
        .iter_mut()
        .find(|n| n.id == StepIdx::new(1))
        .ok_or("node 1 not found")?;
    if let CompiledNodeKind::ForEachNext { body, done, .. } = &mut node.kind {
        *body = StepIdx::new(2);
        *done = StepIdx::new(2);
    } else {
        return Err("node 1 is not ForEachNext".into());
    }
    let result = CompiledWorkflow::try_from_parts(parts);
    match result {
        Err(WorkflowError::ForEachBranchTargetAlias { step, target }) => {
            assert_eq!(step, StepIdx::new(1));
            assert_eq!(target, StepIdx::new(2));
        }
        other => return Err(format!("expected ForEachBranchTargetAlias, got {other:?}")),
    }
    Ok(())
}

#[test]
fn for_each_next_out_of_bounds_precedes_alias_check() -> Result<(), String> {
    let mut parts = fixture_parts()?.to_parts();
    let node = parts
        .nodes
        .iter_mut()
        .find(|n| n.id == StepIdx::new(1))
        .ok_or("node 1 not found")?;
    if let CompiledNodeKind::ForEachNext { body, done, .. } = &mut node.kind {
        *body = StepIdx::new(9);
        *done = StepIdx::new(9);
    } else {
        return Err("node 1 is not ForEachNext".into());
    }
    let result = CompiledWorkflow::try_from_parts(parts);
    match result {
        Err(WorkflowError::StepOutOfBounds { step }) => {
            assert_eq!(step, StepIdx::new(9));
        }
        other => return Err(format!("expected StepOutOfBounds, got {other:?}")),
    }
    Ok(())
}
