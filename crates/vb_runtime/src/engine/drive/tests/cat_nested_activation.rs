#![forbid(unsafe_code)]

mod fixture;
use super::common::mkr;
use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeSignal};
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

fn setup_run_and_store() -> Result<(RunFrame, ValueStore), String> {
    let run = checked(mkr(8, 6))?;
    let store = ValueStore::new();
    Ok((run, store))
}

fn write_source_list(
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> Result<vb_core::ids::ListId, String> {
    let list_id = checked(store.insert_list(Box::from([SlotValue::I64(7), SlotValue::I64(9)])))?;
    checked(run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list_id), Taint::Secret))?;
    Ok(list_id)
}

#[test]
fn test_skipped_pending_nested_region_allows_outer_rearm() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let source_before = wf.to_parts();
    let (mut run, mut store) = setup_run_and_store()?;
    let source_list_id = write_source_list(&mut run, &mut store)?;
    checked(run.write_slot_with_taint(SlotIdx::new(5), SlotValue::Bool(true), Taint::Clean))?;

    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let signal = checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        3,
    ))?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(1));

    assert_eq!(
        checked(run.step_state(StepIdx::new(0)))?,
        StepState::Succeeded
    );
    for id in [1u16, 2, 3, 4, 5, 6] {
        assert_eq!(
            checked(run.step_state(StepIdx::new(id)))?,
            StepState::Pending
        );
    }

    let item = checked(run.read_slot(SlotIdx::new(1)))?;
    assert_eq!(*item, SlotValue::I64(9));
    assert_eq!(checked(run.read_taint(SlotIdx::new(1)))?, Taint::Secret);

    let iter = checked(run.read_slot(SlotIdx::new(2)))?;
    let iter_id = match *iter {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {:?}", other)),
    };
    assert_eq!(checked(run.read_taint(SlotIdx::new(2)))?, Taint::Secret);

    let iter_items = checked(store.list(iter_id))?;
    assert_eq!(
        iter_items,
        &[
            SlotValue::I64(i64::from(source_list_id.get())),
            SlotValue::I64(2),
        ]
    );

    let source = checked(run.read_slot(SlotIdx::new(0)))?;
    let source_id = match *source {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {:?}", other)),
    };
    assert_eq!(source_id, source_list_id);
    assert_eq!(checked(run.read_taint(SlotIdx::new(0)))?, Taint::Secret);

    let source_items = checked(store.list(source_list_id))?;
    assert_eq!(source_items, &[SlotValue::I64(7), SlotValue::I64(9)]);
    assert_eq!(wf.to_parts(), source_before);

    Ok(())
}

#[test]
fn test_entered_nested_start_rejects_before_effects() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let source_before = wf.to_parts();
    let (mut run, mut store) = setup_run_and_store()?;
    write_source_list(&mut run, &mut store)?;
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

    let before = run.clone();
    assert!(!evidence.drain().is_empty());

    let result = drive_phase(&wf, &mut run, &mut store, &mut evidence, &mut collect, 1);
    expect_core_reason(result, "nested loop activation unsupported")?;

    assert_eq!(run, before);
    assert_eq!(wf.to_parts(), source_before);
    assert!(evidence.drain().is_empty());

    Ok(())
}

#[test]
fn test_restored_cached_inner_body_and_control_reject_before_effects() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let source_before = wf.to_parts();
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
        2,
    ))?;
    assert_eq!(run.pc(), StepIdx::new(2));

    assert!(!evidence.drain().is_empty());
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
        expect_core_reason(result, "nested loop activation unsupported")?;
        assert_eq!(test_run, before);
        assert_eq!(wf.to_parts(), source_before);
        assert!(evidence.drain().is_empty());
    }

    Ok(())
}

#[test]
fn test_zero_budget_preserves_precedence_at_nested_start() -> Result<(), String> {
    let wf = checked(build_nested_loop_workflow())?;
    let source_before = wf.to_parts();
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
        2,
    ))?;
    assert_eq!(run.pc(), StepIdx::new(2));

    assert!(!evidence.drain().is_empty());
    let before = run.clone();
    let signal = checked(drive_phase(
        &wf,
        &mut run,
        &mut store,
        &mut evidence,
        &mut collect,
        0,
    ))?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(run, before);
    assert_eq!(wf.to_parts(), source_before);
    assert!(evidence.drain().is_empty());

    Ok(())
}
