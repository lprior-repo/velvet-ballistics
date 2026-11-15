#![forbid(unsafe_code)]

#[path = "runtime_strict_action_preflight/fixture.rs"]
mod fixture;

use fixture::{Case, RUN, TestResult, open, pending, rows};
use vb_core::{SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_storage::JournalEvent;

#[test]
fn wrong_encoded_length_is_refused_before_enqueue_without_cold_mutation() -> TestResult {
    denial(false)
}

#[test]
fn taint_downgrade_is_refused_before_enqueue_without_cold_mutation() -> TestResult {
    denial(true)
}

fn denial(downgrade: bool) -> TestResult {
    let case = Case::new()?;
    let issued = case.issue()?;
    let before_rows = rows(case.path())?;
    let (journal, mut runtime) = open(case.path())?;
    runtime.recover_and_resume(RUN)?;
    assert_eq!(pending(&mut runtime, case.action)?, issued);
    let state = runtime.inspect_run_state(RUN)?.clone();
    assert_eq!(state.frame.read_slot(case.input)?, &SlotValue::I64(42));
    assert_eq!(state.frame.read_taint(case.input)?, Taint::Random);
    let pending_before = runtime.pending_boundary_snapshot(8);
    let counters = runtime.counters_snapshot();
    let events = journal.events_for_run_full(RUN)?;
    let receipt = case.receipt(&journal)?;
    let mut output = case.output()?;
    let expected = if downgrade {
        output.taint = Taint::Clean;
        RuntimeError::ActionTaintDowngrade {
            required: Taint::Random,
            supplied: Taint::Clean,
        }
    } else {
        let actual = output.encoded_len;
        output.encoded_len = actual.checked_add(1).ok_or("test length overflow")?;
        RuntimeError::ActionOutputLengthMismatch {
            declared: output.encoded_len,
            actual,
        }
    };
    assert_eq!(
        runtime.complete_action_with_output(issued, output),
        Err(expected)
    );
    assert_eq!(runtime.inspect_run_state(RUN)?, &state);
    assert_eq!(
        runtime.lookup_pending_action_ticket(RUN, u64::from(case.action.get())),
        Some(issued)
    );
    assert_eq!(runtime.pending_boundary_snapshot(8), pending_before);
    assert_eq!(runtime.counters_snapshot(), counters);
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(case.receipt(&journal)?, receipt);
    drop(runtime);
    drop(journal);
    assert_eq!(rows(case.path())?, before_rows);
    Ok(())
}

#[test]
fn genuine_random_completion_is_accepted_and_persists_once() -> TestResult {
    let case = Case::new()?;
    let issued = case.issue()?;
    let (journal, mut runtime) = open(case.path())?;
    runtime.recover_and_resume(RUN)?;
    assert_eq!(pending(&mut runtime, case.action)?, issued);
    let receipt = case.receipt(&journal)?;
    let output = case.output()?;
    assert_eq!(runtime.complete_action_with_output(issued, output), Ok(()));
    for _ in 0..12 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    assert_eq!(
        runtime.lookup_pending_action_ticket(RUN, u64::from(case.action.get())),
        None
    );
    let events = journal.events_for_run_full(RUN)?;
    let completed: Vec<_> = events
        .iter()
        .filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
        .collect();
    let [
        JournalEvent::ActionCompletedEnvelope {
            ticket,
            output: slot,
            value,
            encoded_len,
            taint,
            ..
        },
    ] = completed.as_slice()
    else {
        return Err("expected exactly one actual completion envelope".into());
    };
    assert_eq!(*ticket, issued);
    assert_eq!(*slot, case.output);
    assert_eq!(*taint, Taint::Random);
    assert_eq!(*encoded_len, output.encoded_len);
    assert_eq!(
        postcard::from_bytes::<SlotValue>(value)?,
        SlotValue::I64(73)
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    assert_eq!(case.receipt(&journal)?, receipt);
    drop(runtime);
    drop(journal);
    let (journal, _runtime) = open(case.path())?;
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(case.receipt(&journal)?, receipt);
    Ok(())
}
