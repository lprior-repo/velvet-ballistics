#![forbid(unsafe_code)]
//! Adversarial IR branch evidence checks; not source-language acceptance.
mod fixture;
use super::common::mkr;
use crate::engine::types::{EvidenceCollector, RuntimeSignal};
use crate::primitives::collect::CollectStates;
use fixture::{
    assert_reduce_state, build_reduce_workflow, count_step_started, drive_phase, get_slot_writes,
};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;

fn check_next(productive: bool) -> Result<(), String> {
    let wf = build_reduce_workflow()?;
    let snapshot = wf.to_parts();

    let mut run = mkr(4, 3)?;
    let mut store = ValueStore::new();

    let source = if productive {
        store.insert_list(vec![SlotValue::I64(7), SlotValue::I64(9)].into_boxed_slice())
    } else {
        store.insert_list(vec![SlotValue::I64(7)].into_boxed_slice())
    }
    .map_err(|e| format!("insert_list: {}", e))?;

    run.write_slot(SlotIdx::new(0), SlotValue::List(source))
        .map_err(|e| format!("write_slot: {}", e))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();

    let sig1 = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 2)
        .map_err(|e| format!("phase1 drive: {}", e))?;

    if !matches!(sig1, RuntimeSignal::StepBudgetExhausted) {
        return Err(format!(
            "phase1 expected StepBudgetExhausted, got {:?}",
            sig1
        ));
    }

    if run.pc() != StepIdx::new(2) {
        return Err(format!("phase1 expected pc=2, got {:?}", run.pc()));
    }

    let iter_before = assert_reduce_state(&run, &store, source, 7, 1)
        .map_err(|e| format!("assert_reduce_state phase1: {}", e))?;

    let phase1_events = evidence.drain();
    let writes1 = get_slot_writes(&phase1_events);

    let expected1 = vec![
        (SlotIdx::new(1), SlotValue::I64(42), Taint::Clean),
        (SlotIdx::new(0), SlotValue::I64(7), Taint::Clean),
        (SlotIdx::new(2), SlotValue::List(iter_before), Taint::Clean),
    ];

    if writes1 != expected1 {
        return Err(format!(
            "phase1 writes mismatch: got {:?}, expected {:?}",
            writes1, expected1
        ));
    }

    let sig2 = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 16)
        .map_err(|e| format!("phase2 drive: {}", e))?;

    if !matches!(sig2, RuntimeSignal::Finished(SlotValue::I64(42))) {
        return Err(format!("phase2 expected Finished(42), got {:?}", sig2));
    }

    let last = if productive { 9 } else { 7 };
    let cursor = if productive { 2 } else { 1 };

    let iter_after = assert_reduce_state(&run, &store, source, last, cursor)
        .map_err(|e| format!("assert_reduce_state phase2: {}", e))?;

    let phase2_events = evidence.drain();
    let writes2 = get_slot_writes(&phase2_events);

    let expected2 = if productive {
        vec![
            (SlotIdx::new(0), SlotValue::I64(9), Taint::Clean),
            (SlotIdx::new(2), SlotValue::List(iter_after), Taint::Clean),
        ]
    } else {
        Vec::new()
    };

    if writes2 != expected2 {
        return Err(format!(
            "phase2 writes mismatch: got {:?}, expected {:?}",
            writes2, expected2
        ));
    }

    let step2_count = count_step_started(&phase2_events, StepIdx::new(2));

    let expected_count = if productive { 2 } else { 1 };
    if step2_count != expected_count {
        return Err(format!(
            "step2 count: got {}, expected {}",
            step2_count, expected_count
        ));
    }

    if !productive && iter_before != iter_after {
        return Err("exhausted: iterBefore != iterAfter".to_string());
    }

    if wf.to_parts() != snapshot {
        return Err("IR changed".to_string());
    }

    Ok(())
}

#[test]
fn productive_reduce_next_exact_evidence() -> Result<(), String> {
    check_next(true)
}

#[test]
fn exhausted_reduce_next_no_write_evidence() -> Result<(), String> {
    check_next(false)
}
