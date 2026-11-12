use super::*;
use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{SlotIdx, SlotValue, StepIdx, StepState, Taint, action::ActionOutputReady};
use vb_runtime::{runtime::Runtime, shard::ShardConfig};
use vb_storage::FjallJournal;

fn pending_action_with_tail() -> TestResult<(tempfile::TempDir, Arc<FjallJournal>, Runtime)> {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let yaml = source(3, "0ms", false).replace(
        "  - id: done\n",
        "  - id: tail_first\n    set: {output: tail_first, value: '1'}\n  - id: tail_second\n    set: {output: tail_second, value: '2'}\n  - id: done\n",
    );
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
    assert_eq!(workflow.node_count(), 5);
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 32,
            step_budget_per_tick: 1,
            max_active_runs: 4,
            policy: RuntimePolicy::Journaled,
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    submit(&mut runtime, workflow)?;
    for _ in 0..5 {
        if runtime.lookup_pending_action_ticket(RUN, 1).is_some() {
            break;
        }
        assert!(runtime.tick_all()?);
    }
    let issued = ticket(&runtime)?;
    assert_eq!(issued.attempt, 1);
    assert_eq!(issued.capacity, 3);
    assert_eq!(runtime.counters_snapshot().runs_completed, 0);
    Ok((directory, journal, runtime))
}

fn assert_finished_once(runtime: &mut Runtime, journal: &FjallJournal) -> TestResult {
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RetryTimerScheduled { .. }))
            .count(),
        0
    );
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
    for _ in 0..5 {
        assert!(runtime.tick_all()?);
    }
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}

#[test]
fn one_tick_preserves_pending_continuation_and_idle_ticks_finish_it() -> TestResult {
    let (_directory, journal, mut runtime) = pending_action_with_tail()?;
    let value = SlotValue::I64(99);
    runtime.complete_action_with_output(
        ticket(&runtime)?,
        ActionOutputReady {
            output_slot: SlotIdx::new(1),
            value,
            encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
            taint: Taint::Clean,
        },
    )?;
    assert!(runtime.tick_all()?);
    assert_eq!(runtime.counters_snapshot().runs_completed, 0);
    let state = runtime.inspect_run_state(RUN)?;
    assert_eq!(state.frame.pc(), StepIdx::new(3));
    assert_eq!(state.frame.step_state(StepIdx::new(3))?, StepState::Pending);
    assert_eq!(state.frame.read_slot(SlotIdx::new(1))?, &SlotValue::I64(99));
    assert_eq!(state.frame.read_slot(SlotIdx::new(2))?, &SlotValue::I64(1));
    assert!(runtime.tick_all()?);
    assert_eq!(runtime.counters_snapshot().runs_completed, 0);
    assert_eq!(runtime.inspect_run_state(RUN)?.frame.pc(), StepIdx::new(4));
    assert!(runtime.tick_all()?);
    assert_finished_once(&mut runtime, &journal)
}

#[test]
fn succeed_fixture_finishes_cooperative_tail_once() -> TestResult {
    let (_directory, journal, mut runtime) = pending_action_with_tail()?;
    succeed(&mut runtime)?;
    assert_finished_once(&mut runtime, &journal)
}
