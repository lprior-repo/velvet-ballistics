use super::support::{Case, TestResult};
use vb_core::{SlotValue, StepIdx, Taint};
use vb_runtime::{RuntimeError, recovery::RuntimeRecoveryProduct, runtime::Runtime};
use vb_storage::JournalEvent;

#[test]
fn fresh_and_real_cold_has_preserve_canonical_owned_arenas_and_exact_results() -> TestResult {
    for present in [true, false] {
        for cold in [false, true] {
            run(present, cold)?;
        }
    }
    Ok(())
}
fn run(present: bool, cold: bool) -> TestResult {
    let case = Case::new(present)?;
    let journal = case.open()?;
    case.assert_prefix(&journal)?;
    let retained = case.retained(&journal)?;
    let prefix = journal.events_for_run_full(case.run)?;
    let mut runtime = case.runtime(journal.clone())?;
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    assert_eq!(journal.events_for_run_full(case.run)?, prefix);
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_cut(&case, &runtime, 1, 1, None)?;
    assert_eq!(case.retained(&journal)?, retained);
    let (mut runtime, journal) = if cold {
        drop(runtime);
        drop(journal);
        let reopened = case.open()?;
        let before = reopened.events_for_run_full(case.run)?;
        let runtime = case.runtime(reopened.clone())?;
        assert!(matches!(
            runtime.recover_product(case.run)?,
            RuntimeRecoveryProduct::Resumable(_)
        ));
        assert_eq!(reopened.events_for_run_full(case.run)?, before);
        runtime.recover_and_resume(case.run)?;
        assert_eq!(reopened.events_for_run_full(case.run)?, before);
        assert_eq!(case.retained(&reopened)?, retained);
        (runtime, reopened)
    } else {
        (runtime, journal)
    };
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_cut(&case, &runtime, 2, 2, Some(present))?;
    let state = runtime.inspect_run_state(case.run)?;
    assert_eq!(
        state.frame.read_taint(case.output(1)?)?,
        Taint::DerivedFromSecret
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(
        runtime.inspect_run_state(case.run).map(|_| ()),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(case.retained(&journal)?, retained);
    let events = journal.events_for_run_full(case.run)?;
    assert_eq!(events.get(..prefix.len()), Some(prefix.as_slice()));
    let results: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::RunFinished {
                run,
                result,
                attempt,
                ..
            } => Some((*run, *result, *attempt)),
            _ => None,
        })
        .collect();
    assert_eq!(results, [(case.run, case.output(1)?, 1)]);
    let writes: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::SlotWrittenEvent { slot, attempt, .. }
                if *slot == case.output(1).ok()? =>
            {
                Some((*slot, *attempt, event.slot_value().ok()??))
            }
            _ => None,
        })
        .collect();
    assert_eq!(writes, [(case.output(1)?, 1, SlotValue::Bool(present))]);
    let before = events;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(journal.events_for_run_full(case.run)?, before);
    Ok(())
}
fn assert_cut(
    case: &Case,
    runtime: &Runtime,
    steps: u16,
    executed: u64,
    result: Option<bool>,
) -> TestResult {
    let state = runtime.inspect_run_state(case.run)?;
    let mut expected = vb_core::new_run_frame(case.run, &case.workflow)?;
    expected.set_max_parallel_in_flight(0);
    for (slot, value, taint) in case.bundle.roots() {
        expected.write_slot_with_taint(*slot, *value, *taint)?;
    }
    expected.write_slot_with_taint(case.output(0)?, case.token, Taint::Secret)?;
    if let Some(result) = result {
        expected.write_slot_with_taint(
            case.output(1)?,
            SlotValue::Bool(result),
            Taint::DerivedFromSecret,
        )?;
    }
    for step in 0..steps {
        expected.mark_succeeded(StepIdx::new(step))?;
    }
    expected.set_pc(StepIdx::new(steps))?;
    for _ in 0..executed {
        expected.increment_executed()?;
    }
    assert_eq!(state.frame.checkpoint_v1(), expected.checkpoint_v1());
    assert_eq!(state.store.checkpoint_v1(), *case.bundle.checkpoint());
    assert_eq!(state.workflow.to_parts(), case.workflow.to_parts());
    assert_eq!(state.run_started_millis, Some(case.started_millis));
    Ok(())
}
