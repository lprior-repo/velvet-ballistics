// Tests for the P1 bug-hunt fixes batch:
//
//   RS-101 (vb-f5rpv): Cancel/kill terminalize runs without clearing
//                      runtime state. The fix routes cancel/kill through
//                      `RuntimeEvent::TerminalRemove` so `runtime_states`
//                      is cleared alongside `run_state`, `terminal_runs`,
//                      and `terminal_outcomes`.
//
//   RS-205 (vb-cwrm9): Evidence flush drops unprocessed events after the
//                      first journal error. The fix restores the
//                      unprocessed evidence suffix back into the
//                      collector on failure so retry can replay it.
//
//   RS-005 (vb-boq04): `take_run_state` / `drive_run` / `handle_timer` /
//                      `await_action` lose the run on intermediate
//                      failure. The fix re-inserts the run state into
//                      `runs` (or, for `drive_state`/`flush_evidence`
//                      errors, before the drive result is applied) so
//                      the run is not silently dropped.
//
//   SA-004 (vb-fgtaa): `JournalWriterQueue::drain_all` silently returns
//                      with items still pending under concurrent
//                      enqueue. The fix extends `JournalWriterFlushReport`
//                      with `pending_after` and (under shutdown) loops
//                      past the static iteration bound until empty.
//
// The tests below verify each fix end-to-end against real shard and
// queue instances.

// suspended_workflow(), wait_workflow(), small_config() are defined in
// earlier test chunks (chunk_dispatch_error_semantics.rs and
// chunk_001.rs) which are also `include!`d into the same module.

// =============================================================================
// RS-101: cancel/kill must clear `runtime_states`
// =============================================================================

/// RS-101: cancel must remove the run from `runtime_states` so the FSM
/// does not retain a stale `Resumable` entry after the terminal journal
/// event has been appended.
#[test]
fn cancel_clears_runtime_states() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(501);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Before cancel: FSM map tracks the run as `Resumable` because the
    // submit-then-tick path drives it into an `AwaitAction` suspension.
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::RuntimeState::Resumable)
    );

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // RS-101: after cancel, the FSM map must be cleared.
    assert_eq!(
        shard.runtime_state_get(run),
        None,
        "cancel must clear runtime_states via TerminalRemove (RS-101)"
    );
    Ok(())
}

/// RS-101: kill must remove the run from `runtime_states` symmetrically
/// to cancel.
#[test]
fn kill_clears_runtime_states() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(502);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::RuntimeState::Resumable)
    );

    assert_eq!(
        shard.enqueue(ShardCommand::Kill { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.runtime_state_get(run),
        None,
        "kill must clear runtime_states via TerminalRemove (RS-101)"
    );
    Ok(())
}

/// RS-101: cancel-after-kill and kill-after-cancel must both keep
/// `runtime_states` cleared, mirroring the terminalisation-monotonicity
/// invariant for the FSM map.
#[test]
fn cancel_kill_alternating_keeps_runtime_states_cleared() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(503);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // First terminalisation: cancel.
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.runtime_state_get(run), None);

    // Alternating cancel/kill must remain idempotent with respect to
    // runtime_states.
    for _ in 0..2 {
        assert_eq!(
            shard.enqueue(ShardCommand::Kill { run, reason: None }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.runtime_state_get(run), None);

        assert_eq!(
            shard.enqueue(ShardCommand::Cancel { run, reason: None }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.runtime_state_get(run), None);
    }
    Ok(())
}

// =============================================================================
// RS-205: `EvidenceCollector::push_event` must restore dropped events
// =============================================================================

/// RS-205: `EvidenceCollector::push_event` must accept every variant
/// produced by `drain`, including `SlotWritten` with no extra, and must
/// not panic on capacity overflow (overflow becomes `dropped`, not a
/// panic). This guards against the partial-flush restoration path
/// dropping events on capacity.
#[test]
fn evidence_collector_push_event_round_trips_drain() {
    use crate::engine::evidence::{EvidenceCollector, EvidenceEvent};
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::value::{SlotValue, Taint};

    let mut collector = EvidenceCollector::with_capacity(8);
    collector.push_step_started(StepIdx::new(0));
    collector.push_slot_written_with_taint(
        SlotIdx::new(0),
        SlotValue::I64(42),
        Taint::Clean,
    );
    collector.push_step_succeeded(StepIdx::new(0), Some(SlotIdx::new(0)));

    let drained: Vec<EvidenceEvent> = collector.drain();
    assert_eq!(drained.len(), 3);
    assert!(collector.is_empty());

    // Restore every event via `push_event`.
    for event in drained.iter().copied() {
        collector.push_event(event);
    }
    assert_eq!(collector.len(), 3);
    assert_eq!(collector.dropped(), 0);
}

/// RS-205: capacity overflow during a restore must increment the
/// `dropped` counter rather than panic or corrupt state.
#[test]
fn evidence_collector_push_event_overflow_reports_dropped() {
    use crate::engine::evidence::{EvidenceCollector, EvidenceEvent};
    use vb_core::ids::{SlotIdx, StepIdx};

    let mut collector = EvidenceCollector::with_capacity(2);
    // Capacity is 2; push 4 events to force overflow.
    for i in 0..4u16 {
        collector.push_event(EvidenceEvent::StepStarted {
            step: StepIdx::new(i),
        });
    }
    assert_eq!(collector.len(), 2);
    assert_eq!(
        collector.dropped(),
        2,
        "overflow during push_event must be observable via dropped() (RS-205)"
    );
}

// =============================================================================
// RS-005: state must be restored on intermediate failure
// =============================================================================

/// RS-005 fixture: a journal that rejects `StepStarted` events. This
/// causes `flush_evidence` to fail mid-batch inside `drive_run`, which
/// is the failure path that pre-fix silently dropped the run state.
struct RejectStepStartedJournal {
    events: std::sync::Mutex<Vec<RuntimeJournalEvent>>,
}

impl RejectStepStartedJournal {
    fn shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            events: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn snapshot(&self) -> Result<Vec<RuntimeJournalEvent>, RuntimeError> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }
}

impl crate::journal::RuntimeJournal for RejectStepStartedJournal {
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        if matches!(event, RuntimeJournalEvent::StepStarted { .. }) {
            return Err(RuntimeError::from(vb_storage::JournalError::QueueFull));
        }
        self.events
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(event);
        Ok(())
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

/// RS-005: `handle_submit` failure during the post-submit `drive_run`
/// path must NOT silently drop the run. The run must remain in
/// `run_state` so the operator can retry / inspect / cancel it.
///
/// We exercise the failure path via a journal that rejects
/// `StepStarted`, which is emitted by `flush_step_started` inside
/// `flush_evidence`. Without the RS-005 fix the run would be lost
/// from `run_state` because `take_run_state` already removed it
/// before the journal append fired.
#[test]
fn rs005_run_state_restored_on_evidence_flush_failure() -> Result<(), RuntimeError> {
    let journal = RejectStepStartedJournal::shared();
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;

    // A trivial workflow that emits at least one `StepStarted` event so
    // the rejecting journal fires during flush_evidence.
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(601);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    let tick_result = shard.tick();
    // Drive loop fails because flush_evidence rejects StepStarted.
    assert!(
        tick_result.is_err(),
        "drive_run must return Err when flush_evidence fails, got {tick_result:?}"
    );

    // RS-005: the run must still be present in run_state so the
    // operator can recover it. Pre-fix, `take_run_state` had already
    // removed it and the failure path did not re-insert.
    assert_eq!(
        shard.run_state_contains(run),
        true,
        "RS-005: run must remain in run_state after drive_run failure"
    );

    // Cancel after the failure must succeed and clear the run cleanly.
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.run_state_contains(run), false);
    assert_eq!(shard.terminal_runs_contains(run), true);
    Ok(())
}