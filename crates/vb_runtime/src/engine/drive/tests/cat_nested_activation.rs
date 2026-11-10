#![forbid(unsafe_code)]

//! Nested loop activation: inner loops execute and outer rearm resets them.
//!
//! The fixture nests a `RepeatStart` region (steps 2..=5) inside an outer
//! `ForEachStart` region (steps 1..=6). These tests prove the runtime
//! activates the nested start, runs the inner region per outer item, and
//! resets the whole inner region when the outer latch rearms.

mod fixture;
use super::common::mkr;
use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
};
use crate::primitives::collect::CollectStates;
use fixture::build_nested_loop_workflow;
use vb_core::capability::CapabilitySet;
use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

fn checked<T, E: std::fmt::Debug>(r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| format!("{e:?}"))
}

fn drive_phase(
    wf: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    evidence: &mut EvidenceCollector,
    collect: &mut CollectStates,
    steps: u64,
) -> Result<RuntimeSignal, RuntimeEngineError> {
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
}

fn setup_run_and_store() -> Result<(RunFrame, ValueStore), String> {
    let run = mkr(8, 6).map_err(|e| format!("{e:?}"))?;
    let store = ValueStore::new();
    Ok((run, store))
}

fn write_source_list(
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> Result<vb_core::ids::ListId, String> {
    let list_id = store
        .insert_list(Box::from([SlotValue::I64(7), SlotValue::I64(9)]))
        .map_err(|e| format!("{e:?}"))?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list_id), Taint::Secret)
        .map_err(|e| format!("{e:?}"))?;
    Ok(list_id)
}

fn start_count(evidence: &[EvidenceEvent], step: u16) -> usize {
    evidence
        .iter()
        .filter(|e| {
            matches!(
                e,
                EvidenceEvent::StepStarted { step: s } if s.get() == step
            )
        })
        .count()
}

fn assert_outer_entry_evidence(
    evidence: Vec<EvidenceEvent>,
    run: &RunFrame,
    store: &ValueStore,
    source: vb_core::ListId,
) -> Result<(), String> {
    assert_eq!(
        checked(run.read_slot(SlotIdx::ZERO))?,
        &SlotValue::List(source)
    );
    let cursor_value = *checked(run.read_slot(SlotIdx::new(2)))?;
    let SlotValue::List(cursor) = cursor_value else {
        return Err("outer cursor must be a list".into());
    };
    assert_eq!(
        checked(store.list(cursor))?,
        &[SlotValue::I64(i64::from(source.get())), SlotValue::I64(1)]
    );
    assert_eq!(
        evidence,
        vec![
            EvidenceEvent::StepStarted {
                step: StepIdx::ZERO
            },
            EvidenceEvent::SlotWritten {
                slot: SlotIdx::new(2),
                value: cursor_value,
                taint: Taint::Secret,
                extra: None
            },
            EvidenceEvent::StepSucceeded {
                step: StepIdx::ZERO,
                output: Some(SlotIdx::new(2))
            },
            EvidenceEvent::StepStarted {
                step: StepIdx::new(1)
            },
            EvidenceEvent::StepSucceeded {
                step: StepIdx::new(1),
                output: None
            },
        ]
    );
    Ok(())
}

fn drive_enter_inner(
    take_inner: bool,
) -> Result<
    (
        CompiledWorkflow,
        RunFrame,
        ValueStore,
        EvidenceCollector,
        CollectStates,
    ),
    String,
> {
    let wf = build_nested_loop_workflow()?;
    let source_before = wf.to_parts();
    let (mut run, mut store) = setup_run_and_store()?;
    write_source_list(&mut run, &mut store)?;
    run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(!take_inner), Taint::Clean)
        .map_err(|e| format!("{e:?}"))?;
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        2,
    ))?;
    assert_eq!(run.pc(), StepIdx::new(2));
    assert_eq!(wf.to_parts(), source_before);
    Ok((wf, run, store, evidence, collect))
}

#[test]
fn test_skipped_pending_nested_region_allows_outer_rearm() -> Result<(), String> {
    let wf = build_nested_loop_workflow()?;
    let source_before = wf.to_parts();
    let (mut run, mut store) = setup_run_and_store()?;
    let source_list_id = write_source_list(&mut run, &mut store)?;
    checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(true), Taint::Clean))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let signal = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 3)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(1));

    assert_eq!(
        run.step_state(StepIdx::new(0))
            .map_err(|e| format!("{e:?}"))?,
        StepState::Succeeded
    );
    for id in [1u16, 2, 3, 4, 5, 6] {
        assert_eq!(
            run.step_state(StepIdx::new(id))
                .map_err(|e| format!("{e:?}"))?,
            StepState::Pending
        );
    }

    let item = run
        .read_slot(SlotIdx::new(1))
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(*item, SlotValue::I64(9));
    assert_eq!(
        run.read_taint(SlotIdx::new(1))
            .map_err(|e| format!("{e:?}"))?,
        Taint::Secret
    );

    let iter = run
        .read_slot(SlotIdx::new(2))
        .map_err(|e| format!("{e:?}"))?;
    let iter_id = match *iter {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {other:?}")),
    };
    assert_eq!(
        run.read_taint(SlotIdx::new(2))
            .map_err(|e| format!("{e:?}"))?,
        Taint::Secret
    );

    let iter_items = store.list(iter_id).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        iter_items,
        &[
            SlotValue::I64(i64::from(source_list_id.get())),
            SlotValue::I64(2),
        ]
    );

    let source = run
        .read_slot(SlotIdx::new(0))
        .map_err(|e| format!("{e:?}"))?;
    let source_id = match *source {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {other:?}")),
    };
    assert_eq!(source_id, source_list_id);
    assert_eq!(
        run.read_taint(SlotIdx::new(0))
            .map_err(|e| format!("{e:?}"))?,
        Taint::Secret
    );

    let source_items = store.list(source_list_id).map_err(|e| format!("{e:?}"))?;
    assert_eq!(source_items, &[SlotValue::I64(7), SlotValue::I64(9)]);
    assert_eq!(wf.to_parts(), source_before);

    Ok(())
}

#[test]
fn test_entered_nested_start_activates_before_effects() -> Result<(), String> {
    let (wf, mut run, mut store, mut evidence, mut collect) = drive_enter_inner(true)?;
    let source_before = wf.to_parts();
    assert!(!evidence.drain().is_empty());

    let signal = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 1)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(3));
    assert_eq!(
        run.step_state(StepIdx::new(2))
            .map_err(|e| format!("{e:?}"))?,
        StepState::Succeeded
    );
    assert_eq!(wf.to_parts(), source_before);
    assert!(!evidence.drain().is_empty());

    Ok(())
}

#[test]
fn test_inner_body_and_control_activate() -> Result<(), String> {
    let (wf, mut run, mut store, mut evidence, mut collect) = drive_enter_inner(true)?;
    let source_before = wf.to_parts();
    assert!(!evidence.drain().is_empty());

    let signal = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 7)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(6));
    for id in [2u16, 3, 4, 5] {
        assert_eq!(
            run.step_state(StepIdx::new(id))
                .map_err(|e| format!("{e:?}"))?,
            StepState::Succeeded
        );
    }
    let events = evidence.drain();
    assert_eq!(start_count(&events, 2), 1);
    assert_eq!(start_count(&events, 3), 2);
    assert_eq!(start_count(&events, 5), 2);
    assert_eq!(wf.to_parts(), source_before);

    Ok(())
}

#[test]
fn test_outer_rearm_resets_inner_region_between_items() -> Result<(), String> {
    let (wf, mut run, mut store, mut evidence, mut collect) = drive_enter_inner(true)?;
    let source_before = wf.to_parts();
    assert!(!evidence.drain().is_empty());

    let signal = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 8)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(1));
    // The outer rearm interval is inclusive of its latch: the whole inner
    // region plus the outer body and latch return to Pending.
    for id in [1u16, 2, 3, 4, 5, 6] {
        assert_eq!(
            run.step_state(StepIdx::new(id))
                .map_err(|e| format!("{e:?}"))?,
            StepState::Pending
        );
    }
    let events = evidence.drain();
    assert_eq!(start_count(&events, 2), 1);
    assert_eq!(start_count(&events, 3), 2);
    assert_eq!(start_count(&events, 5), 2);
    assert_eq!(start_count(&events, 6), 1);
    assert_eq!(wf.to_parts(), source_before);

    Ok(())
}

#[test]
fn test_nested_loops_complete_once_per_outer_item() -> Result<(), String> {
    let (wf, mut run, mut store, mut evidence, mut collect) = drive_enter_inner(true)?;
    let source_before = wf.to_parts();
    assert!(!evidence.drain().is_empty());

    let signal = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 100)
        .map_err(|e| format!("{e:?}"))?;
    assert!(matches!(signal, RuntimeSignal::Finished(_)));
    let events = evidence.drain();
    assert_eq!(start_count(&events, 1), 1);
    assert_eq!(start_count(&events, 2), 2);
    assert_eq!(start_count(&events, 3), 4);
    assert_eq!(start_count(&events, 5), 4);
    assert_eq!(start_count(&events, 6), 2);
    assert_eq!(wf.to_parts(), source_before);

    Ok(())
}

#[test]
fn test_zero_budget_preserves_precedence_at_nested_start() -> Result<(), String> {
    let (wf, mut run, mut store, mut evidence, mut collect) = drive_enter_inner(true)?;
    let source_before = wf.to_parts();
    assert!(!evidence.drain().is_empty());

    let before = run.clone();
    let signal = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run, before);
    assert_eq!(wf.to_parts(), source_before);
    assert!(evidence.drain().is_empty());

    Ok(())
}

#[test]
fn test_nested_loop_outer_repeat_completes() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let (mut run, mut store) = setup_run_and_store()?;
    let source = write_source_list(&mut run, &mut store)?;
    checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(false), Taint::Clean))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let signal = checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        100,
    ))?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::List(source)));

    let started = evidence
        .drain()
        .iter()
        .filter(|e| matches!(e, crate::engine::types::EvidenceEvent::StepStarted { .. }))
        .count();
    assert_eq!(started, 20);

    Ok(())
}

#[test]
fn test_nested_loop_taint_preserved_across_iterations() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let (mut run, mut store) = setup_run_and_store()?;
    write_source_list(&mut run, &mut store)?;
    checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(false), Taint::Clean))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        100,
    ))?;

    // After execution, the iterator slot should still have the outer list's taint
    let _iter = checked(run.read_slot(SlotIdx::new(2)))?;
    assert_eq!(checked(run.read_taint(SlotIdx::new(2)))?, Taint::Secret);

    // The input list should be unchanged
    let source = checked(run.read_slot(SlotIdx::new(0)))?;
    let source_id = match *source {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {:?}", other)),
    };
    let source_items = checked(store.list(source_id))?;
    assert_eq!(source_items, &[SlotValue::I64(7), SlotValue::I64(9)]);
    assert_eq!(checked(run.read_taint(SlotIdx::new(0)))?, Taint::Secret);

    Ok(())
}

#[test]
fn test_nested_loop_exact_step_count() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let (mut run, mut store) = setup_run_and_store()?;
    write_source_list(&mut run, &mut store)?;
    checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(false), Taint::Clean))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        100,
    ))?;

    let events = evidence.drain();
    let step_started = events
        .iter()
        .filter(|e| matches!(e, crate::engine::types::EvidenceEvent::StepStarted { .. }))
        .count();
    let step_succeeded = events
        .iter()
        .filter(|e| matches!(e, crate::engine::types::EvidenceEvent::StepSucceeded { .. }))
        .count();

    assert_eq!(step_started, step_succeeded);
    assert_eq!(step_started, 20);
    Ok(())
}

#[test]
fn completed_outer_control_cannot_authorize_an_unfinished_inner_repeat() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let (mut run, mut store) = setup_run_and_store()?;
    write_source_list(&mut run, &mut store)?;
    checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(false), Taint::Clean))?;
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        3,
    ))?;
    checked(run.mark_succeeded(StepIdx::new(6)))?;
    checked(run.set_pc(StepIdx::new(1)))?;
    let before = run.clone();
    let result =
        super::super::loop_rearm::loop_body_rearm(&wf, &mut run, StepIdx::new(6), StepIdx::new(1));
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(
            EngineError::InvalidCompiledWorkflow {
                reason: "nested loop region not consistently completed"
            }
        ))
    ));
    assert_eq!(run, before);
    Ok(())
}

fn expect_core_reason(
    r: Result<RuntimeSignal, RuntimeEngineError>,
    reason: &str,
) -> Result<(), String> {
    match r {
        Err(RuntimeEngineError::Core(EngineError::InvalidCompiledWorkflow { reason: actual }))
            if actual == reason =>
        {
            Ok(())
        }
        other => Err(format!("unexpected error: {other:?}")),
    }
}

#[test]
fn test_entered_nested_start_allowed() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let source_before = wf.to_parts();
    let (mut run, mut store) = setup_run_and_store()?;
    let source = write_source_list(&mut run, &mut store)?;
    checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(false), Taint::Clean))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let signal = checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        2,
    ))?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(2));

    assert_outer_entry_evidence(evidence.drain(), &run, &store, source)?;

    // Inner loop (RepeatStart at step 2) should now be allowed to execute
    // because the outer ForEachStart (step 0) has Succeeded.
    let signal = checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        50,
    ))?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::List(source)));

    assert_eq!(wf.to_parts(), source_before);
    Ok(())
}

#[test]
fn test_restored_cached_inner_body_and_control_reject_before_effects() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let source_before = wf.to_parts();
    let (mut run, mut store) = setup_run_and_store()?;
    let source = write_source_list(&mut run, &mut store)?;
    checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(false), Taint::Clean))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        2,
    ))?;
    assert_eq!(run.pc(), StepIdx::new(2));

    assert_outer_entry_evidence(evidence.drain(), &run, &store, source)?;
    for step_id in [3u16, 4, 5] {
        let mut test_run = run.clone();
        checked(test_run.set_pc(StepIdx::new(step_id)))?;

        let before = test_run.clone();
        let result = drive_phase(
            &wf,
            &mut test_run,
            &mut store,
            &mut evidence,
            &mut collect,
            1,
        );
        expect_core_reason(result, "containing loop region start not succeeded")?;
        assert_eq!(test_run, before);
        assert_eq!(wf.to_parts(), source_before);
        assert_eq!(evidence.drain(), Vec::<EvidenceEvent>::new());
    }

    Ok(())
}
