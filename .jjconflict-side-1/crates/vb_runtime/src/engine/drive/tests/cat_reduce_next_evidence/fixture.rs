#![forbid(unsafe_code)]

use super::super::common::{cn, fin, mkwfc, nop};
use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal};
use crate::primitives::collect::CollectStates;
use vb_core::capability::CapabilitySet;
use vb_core::engine::StepBudget;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledNodeKind;

pub(super) fn build_reduce_workflow() -> Result<vb_core::workflow::CompiledWorkflow, String> {
    let reduce = cn(
        0,
        Some(2),
        None,
        CompiledNodeKind::ReduceStart {
            input: SlotIdx::new(0),
            accumulator: SlotIdx::new(1),
            initial: ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
    );
    let body = nop(1, 2);
    let next = cn(
        2,
        Some(0),
        None,
        CompiledNodeKind::ReduceNext {
            iterator_slot: SlotIdx::new(2),
            accumulator: SlotIdx::new(1),
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
    );
    let finish = fin(3, 1);
    mkwfc(
        vec![reduce, body, next, finish],
        3,
        vec![ConstValue::I64(42)],
    )
}

pub(super) fn drive_phase(
    wf: &vb_core::workflow::CompiledWorkflow,
    run: &mut vb_core::frame::RunFrame,
    store: &mut ValueStore,
    evidence: &mut EvidenceCollector,
    collect: &mut CollectStates,
    steps: u64,
) -> Result<RuntimeSignal, String> {
    let mut budget = StepBudget::new(steps);
    drive_deterministic_full(
        wf,
        run,
        &mut budget,
        store,
        &[],
        RetryPolicy::NEVER,
        evidence,
        collect,
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("{e}"))
}

pub(super) fn assert_reduce_state(
    run: &vb_core::frame::RunFrame,
    store: &ValueStore,
    source: vb_core::ids::ListId,
    last: i64,
    cursor: i64,
) -> Result<vb_core::ids::ListId, String> {
    let s0 = run
        .read_slot(SlotIdx::new(0))
        .map_err(|e| format!("slot0: {}", e))?;
    if *s0 != SlotValue::I64(last) {
        return Err(format!("slot0 expected I64({}), got {:?}", last, s0));
    }
    let s1 = run
        .read_slot(SlotIdx::new(1))
        .map_err(|e| format!("slot1: {}", e))?;
    if *s1 != SlotValue::I64(42) {
        return Err(format!("slot1 expected I64(42), got {:?}", s1));
    }
    for idx in [SlotIdx::new(0), SlotIdx::new(1), SlotIdx::new(2)] {
        let t = run.read_taint(idx).map_err(|e| format!("taint: {}", e))?;
        if t != Taint::Clean {
            return Err("taint not clean".to_string());
        }
    }
    let s2 = run
        .read_slot(SlotIdx::new(2))
        .map_err(|e| format!("slot2: {}", e))?;
    let iter_id = match s2 {
        SlotValue::List(id) => *id,
        other => return Err(format!("slot2 expected List, got {:?}", other)),
    };
    let items = store.list(iter_id).map_err(|e| format!("list: {}", e))?;
    let mut iter = items.iter();
    let first = iter.next().ok_or("list empty")?;
    if *first != SlotValue::I64(i64::from(source.get())) {
        return Err(format!(
            "list[0] expected I64({}), got {:?}",
            source.get(),
            first
        ));
    }
    let second = iter.next().ok_or("list short")?;
    if *second != SlotValue::I64(cursor) {
        return Err(format!(
            "list[1] expected I64({}), got {:?}",
            cursor, second
        ));
    }
    if iter.next().is_some() {
        return Err("list too long".to_string());
    }
    Ok(iter_id)
}

pub(super) fn get_slot_writes(events: &[EvidenceEvent]) -> Vec<(SlotIdx, SlotValue, Taint)> {
    events
        .iter()
        .filter_map(|e| match e {
            EvidenceEvent::SlotWritten {
                slot, value, taint, ..
            } => Some((*slot, *value, *taint)),
            _ => None,
        })
        .collect()
}

pub(super) fn count_step_started(events: &[EvidenceEvent], step: StepIdx) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepStarted { step: s } if *s == step))
        .count()
}
