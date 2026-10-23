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
use vb_core::workflow::CompiledNodeKind;

fn checked<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, String> {
    result.map_err(|error| format!("{error:?}"))
}

#[test]
fn forward_aliased_for_each_next_emits_actual_writes() -> Result<(), String> {
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
                done: StepIdx::new(2),
            },
        ),
        fin(2, 1),
        fin(3, 1),
    ];
    let workflow = mkwfc(nodes, 3, vec![])?;
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
        &[SlotValue::I64(i64::from(source.get())), SlotValue::I64(2),]
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
