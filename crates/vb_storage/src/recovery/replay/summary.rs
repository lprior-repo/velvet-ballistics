//! Summary and frame seed building for journal recovery.
//!
//! Provides:
//! - Runtime summary construction from events
//! - Frame seed building for live-frame reconstruction

use std::collections::{HashMap, HashSet};

use crate::JournalEvent;
use crate::recovery::types::{
    RecoveredPendingAction, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState,
    RecoveryError, RecoveryFrameSeed, RecoveryHydration, RecoveryResult, RecoveryRuntimeSummary,
    RunSnapshot, UnsupportedRecoveryState,
};
use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx, Taint};

#[derive(Debug)]
struct SeedRecoveryState {
    summary: RecoveryRuntimeSummary,
    step_states: HashMap<StepIdx, RecoveredStepState>,
    slot_values: HashMap<SlotIdx, SlotValue>,
    slot_taint: HashMap<SlotIdx, Taint>,
    pending_actions: HashSet<(ActionId, StepIdx)>,
    max_step_idx: Option<StepIdx>,
    min_step_idx: StepIdx,
    max_slot_idx: Option<SlotIdx>,
    missing_slot_values: bool,
    event_slot_taint_unsupported: bool,
    pc: StepIdx,
}

/// Applies an event's effects to a runtime summary.
pub fn apply_summary_event(summary: &mut RecoveryRuntimeSummary, event: &JournalEvent) {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            summary.workflow = Some(*workflow);
        }
        JournalEvent::StepStarted { .. } => {
            summary.steps_started = summary.steps_started.saturating_add(1);
        }
        JournalEvent::StepSucceeded { .. } => {
            summary.steps_succeeded = summary.steps_succeeded.saturating_add(1);
        }
        JournalEvent::ActionScheduled { .. } => {
            summary.actions_scheduled = summary.actions_scheduled.saturating_add(1);
        }
        JournalEvent::ActionCompletedEvent { .. } | JournalEvent::ActionFailedEvent { .. } => {
            summary.actions_resolved = summary.actions_resolved.saturating_add(1);
        }
        JournalEvent::SlotWrittenEvent { .. } => {
            summary.slots_written = summary.slots_written.saturating_add(1);
        }
        JournalEvent::WaitScheduledEvent { .. }
        | JournalEvent::AskScheduledEvent { .. }
        | JournalEvent::RetryScheduledEvent { .. } => {
            summary.suspensions = summary.suspensions.saturating_add(1);
        }
        JournalEvent::AskAnsweredEvent { .. } => {}
        JournalEvent::RunCancelled { .. } => {
            summary.terminal = Some(crate::recovery::types::RecoveryTerminalState::Cancelled);
        }
        JournalEvent::RunFinished { result, .. } => {
            summary.terminal =
                Some(crate::recovery::types::RecoveryTerminalState::Finished { result: *result });
        }
        JournalEvent::RunFailedEvent { .. } => {
            summary.terminal = Some(crate::recovery::types::RecoveryTerminalState::Failed);
        }
    }
}
/// Builds a summary-only recovery product from already ordered journal events.
pub fn summarize_recovery_events(events: &[JournalEvent]) -> RecoveryResult<RecoveryHydration> {
    let Some(first) = events.first() else {
        return Err(RecoveryError::NoRecoveryData { run: RunId::new(0) });
    };
    let run = first.run_id();
    let mut summary = RecoveryRuntimeSummary {
        run,
        first_seq: first.seq(),
        last_seq: first.seq(),
        workflow: None,
        steps_started: 0,
        steps_succeeded: 0,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };

    for event in events {
        if event.run_id() != run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "recovery summary received events for multiple runs".to_owned(),
            });
        }
        summary.last_seq = event.seq();
        apply_summary_event(&mut summary, event);
    }

    Ok(RecoveryHydration::Summary(summary))
}

/// Builder that constructs a [`RecoveryFrameSeed`] from journal events.
pub struct RecoveryFrameSeedBuilder;

impl RecoveryFrameSeedBuilder {
    /// Build a frame seed from a pre-collected event slice.
    pub fn build(events: &[JournalEvent]) -> RecoveryResult<RecoveryFrameSeed> {
        recover_runtime_frame_seed_from_events(events)
    }
}

/// Recovers a [`RecoveryFrameSeed`] from ordered journal events.
///
/// Reconstructs step states, dimensions, and program counter from the
/// durable event sequence.
pub fn recover_runtime_frame_seed_from_events(
    events: &[JournalEvent],
) -> RecoveryResult<RecoveryFrameSeed> {
    let Some(first) = events.first() else {
        return Err(RecoveryError::NoRecoveryData { run: RunId::new(0) });
    };
    let run = first.run_id();
    let state = events
        .iter()
        .try_fold(initial_seed_state(first), |state, event| {
            apply_seed_event(state, run, event)
        })?;

    seed_from_state(run, state)
}

fn initial_seed_state(first: &JournalEvent) -> SeedRecoveryState {
    SeedRecoveryState {
        summary: RecoveryRuntimeSummary {
            run: first.run_id(),
            first_seq: first.seq(),
            last_seq: first.seq(),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        },
        step_states: HashMap::new(),
        slot_values: HashMap::new(),
        slot_taint: HashMap::new(),
        pending_actions: HashSet::new(),
        max_step_idx: None,
        min_step_idx: StepIdx::MAX,
        max_slot_idx: None,
        missing_slot_values: false,
        event_slot_taint_unsupported: false,
        pc: StepIdx::ZERO,
    }
}

fn apply_seed_event(
    mut state: SeedRecoveryState,
    run: RunId,
    event: &JournalEvent,
) -> RecoveryResult<SeedRecoveryState> {
    if event.run_id() != run {
        return Err(RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: "frame seed recovery received events for multiple runs".to_owned(),
        });
    }
    state.summary.last_seq = event.seq();
    apply_summary_event(&mut state.summary, event);
    apply_seed_event_body(state, event)
}

fn apply_seed_event_body(
    state: SeedRecoveryState,
    event: &JournalEvent,
) -> RecoveryResult<SeedRecoveryState> {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => Ok(record_workflow(state, *workflow)),
        JournalEvent::StepStarted { step, .. } => {
            Ok(record_step(state, *step, RecoveredStepState::Running, true))
        }
        JournalEvent::StepSucceeded { step, output, .. } => {
            Ok(record_output_step(state, *step, *output))
        }
        JournalEvent::ActionScheduled { action, step, .. } => {
            Ok(record_action_scheduled(state, *action, *step))
        }
        JournalEvent::ActionCompletedEvent { action, step, .. }
        | JournalEvent::ActionFailedEvent { action, step, .. } => {
            Ok(record_action_resolved(state, *action, *step))
        }
        JournalEvent::WaitScheduledEvent { step, .. } => Ok(record_step(
            state,
            *step,
            RecoveredStepState::Waiting,
            false,
        )),
        JournalEvent::AskScheduledEvent { step, .. } => {
            Ok(record_step(state, *step, RecoveredStepState::Asking, false))
        }
        JournalEvent::SlotWrittenEvent { slot, value, .. } => {
            record_slot_write(state, *slot, value)
        }
        JournalEvent::RunFinished { result, .. } => Ok(record_max_slot(state, *result)),
        _ => Ok(state),
    }
}

fn record_workflow(
    mut state: SeedRecoveryState,
    workflow: vb_core::WorkflowDigest,
) -> SeedRecoveryState {
    state.summary.workflow = Some(workflow);
    state
}

fn record_step(
    mut state: SeedRecoveryState,
    step: StepIdx,
    recovered: RecoveredStepState,
    update_min: bool,
) -> SeedRecoveryState {
    state.max_step_idx = max_step(state.max_step_idx, step);
    state.min_step_idx = if update_min && step < state.min_step_idx {
        step
    } else {
        state.min_step_idx
    };
    state.pc = max_pc(state.pc, step);
    state.step_states.insert(step, recovered);
    state
}

fn record_output_step(
    state: SeedRecoveryState,
    step: StepIdx,
    output: SlotIdx,
) -> SeedRecoveryState {
    record_max_slot(
        record_step(state, step, RecoveredStepState::Succeeded, false),
        output,
    )
}

fn record_action_scheduled(
    mut state: SeedRecoveryState,
    action: ActionId,
    step: StepIdx,
) -> SeedRecoveryState {
    state.pending_actions.insert((action, step));
    state
}

fn record_action_resolved(
    mut state: SeedRecoveryState,
    action: ActionId,
    step: StepIdx,
) -> SeedRecoveryState {
    state.pending_actions.remove(&(action, step));
    state
}

fn record_slot_write(
    mut state: SeedRecoveryState,
    slot: SlotIdx,
    value: &Option<Vec<u8>>,
) -> RecoveryResult<SeedRecoveryState> {
    state.max_slot_idx = max_slot(state.max_slot_idx, slot);
    match value {
        Some(bytes) => match postcard::from_bytes::<SlotValue>(bytes) {
            Ok(slot_value) => {
                state.slot_values.insert(slot, slot_value);
                state.slot_taint.remove(&slot);
                state.event_slot_taint_unsupported = true;
                Ok(state)
            }
            Err(_) => Ok(mark_missing_slot_value(state)),
        },
        None => Ok(mark_missing_slot_value(state)),
    }
}

fn record_max_slot(mut state: SeedRecoveryState, slot: SlotIdx) -> SeedRecoveryState {
    state.max_slot_idx = max_slot(state.max_slot_idx, slot);
    state
}

fn mark_missing_slot_value(mut state: SeedRecoveryState) -> SeedRecoveryState {
    state.missing_slot_values = true;
    state
}

fn max_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    Some(current.map_or(candidate, |known| known.max(candidate)))
}

fn max_slot(current: Option<SlotIdx>, candidate: SlotIdx) -> Option<SlotIdx> {
    Some(current.map_or(candidate, |known| known.max(candidate)))
}

fn max_pc(current: StepIdx, candidate: StepIdx) -> StepIdx {
    current.max(candidate)
}

fn seed_unsupported_state(
    missing_slot_values: bool,
    event_slot_taint_unsupported: bool,
    pending_actions_empty: bool,
) -> UnsupportedRecoveryState {
    [
        if missing_slot_values {
            UnsupportedRecoveryState::slot_values_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        },
        if event_slot_taint_unsupported {
            UnsupportedRecoveryState::event_slot_taint_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        },
        if pending_actions_empty {
            UnsupportedRecoveryState::SUPPORTED
        } else {
            UnsupportedRecoveryState::pending_actions_unsupported()
        },
    ]
    .into_iter()
    .fold(
        UnsupportedRecoveryState::SUPPORTED,
        UnsupportedRecoveryState::union,
    )
}

/// Recovers a [`RecoveryFrameSeed`] from a snapshot and ordered tail events.
pub fn recover_runtime_frame_seed_from_snapshot_and_tail(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
) -> RecoveryResult<RecoveryFrameSeed> {
    let accepted = JournalEvent::RunAccepted {
        run: snapshot.run,
        seq: snapshot.seq,
        workflow: snapshot.workflow,
    };
    let state = apply_seed_event(initial_seed_state(&accepted), snapshot.run, &accepted)?;
    let state = apply_snapshot_slots_to_state(state, snapshot)?;
    let state = tail_events.iter().try_fold(state, |state, event| {
        apply_seed_event(state, snapshot.run, event)
    })?;
    seed_from_state(snapshot.run, state)
}

fn seed_from_state(run: RunId, state: SeedRecoveryState) -> RecoveryResult<RecoveryFrameSeed> {
    let step_count = state
        .max_step_idx
        .map(|m| {
            m.get()
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .transpose()?
        .map_or(0, |count| count);
    let slot_count = state
        .max_slot_idx
        .map(|m| {
            m.get()
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .transpose()?
        .map_or(0, |count| count);
    let first_step = if state.min_step_idx == StepIdx::MAX {
        StepIdx::ZERO
    } else {
        state.min_step_idx
    };

    let steps: Vec<RecoveredStepEntry> = state
        .step_states
        .into_iter()
        .map(|(step, state)| RecoveredStepEntry { step, state })
        .collect();

    let slots: Vec<RecoveredSlotEntry> = state
        .slot_values
        .into_iter()
        .map(|(slot, value)| RecoveredSlotEntry {
            slot,
            value,
            taint: match state.slot_taint.get(&slot).copied() {
                Some(taint) => taint,
                None => Taint::Clean,
            },
        })
        .collect();

    let pending_actions: Vec<RecoveredPendingAction> = state
        .pending_actions
        .into_iter()
        .map(|(action, step)| RecoveredPendingAction { step, action })
        .collect();
    let unsupported = seed_unsupported_state(
        state.missing_slot_values,
        state.event_slot_taint_unsupported,
        pending_actions.is_empty(),
    );

    Ok(RecoveryFrameSeed {
        summary: state.summary,
        first_step,
        step_count,
        slot_count,
        pc: state.pc,
        steps,
        slots,
        pending_actions,
        unsupported,
    })
}

fn apply_snapshot_slots_to_state(
    state: SeedRecoveryState,
    snapshot: &RunSnapshot,
) -> RecoveryResult<SeedRecoveryState> {
    let snapshot_values = decode_snapshot_values(snapshot)?;
    let snapshot_taint = decode_snapshot_taint(snapshot)?;
    snapshot_values
        .iter()
        .enumerate()
        .try_fold(state, |state, (index, maybe_value)| {
            record_snapshot_slot(state, index, maybe_value, &snapshot_taint, snapshot)
        })
}

fn record_snapshot_slot(
    mut state: SeedRecoveryState,
    index: usize,
    maybe_value: &Option<SlotValue>,
    snapshot_taint: &[Taint],
    snapshot: &RunSnapshot,
) -> RecoveryResult<SeedRecoveryState> {
    let Some(value) = maybe_value else {
        return Ok(state);
    };
    let slot = slot_idx_from_usize(index, snapshot)?;
    state.max_slot_idx = max_slot(state.max_slot_idx, slot);
    state.slot_values.insert(slot, *value);
    match snapshot_taint.get(index).copied() {
        Some(taint) => {
            state.slot_taint.insert(slot, taint);
        }
        None => {
            state.event_slot_taint_unsupported = true;
        }
    }
    Ok(state)
}

fn decode_snapshot_values(snapshot: &RunSnapshot) -> RecoveryResult<Vec<Option<SlotValue>>> {
    if snapshot.slots.is_empty() {
        return Ok(Vec::new());
    }
    postcard::from_bytes(&snapshot.slots).map_err(|_| RecoveryError::CorruptSnapshot {
        run: snapshot.run,
        seq: snapshot.seq,
    })
}

fn decode_snapshot_taint(snapshot: &RunSnapshot) -> RecoveryResult<Vec<Taint>> {
    if snapshot.taint.is_empty() {
        return Ok(Vec::new());
    }
    postcard::from_bytes(&snapshot.taint).map_err(|_| RecoveryError::CorruptSnapshot {
        run: snapshot.run,
        seq: snapshot.seq,
    })
}

fn slot_idx_from_usize(index: usize, snapshot: &RunSnapshot) -> RecoveryResult<SlotIdx> {
    u16::try_from(index)
        .map(SlotIdx::new)
        .map_err(|_| RecoveryError::CorruptSnapshot {
            run: snapshot.run,
            seq: snapshot.seq,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventSeq;
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx};

    fn fresh_summary() -> RecoveryRuntimeSummary {
        RecoveryRuntimeSummary {
            run: RunId::new(1),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        }
    }

    fn assert_counters(
        summary: &RecoveryRuntimeSummary,
        steps_started: u64,
        steps_succeeded: u64,
        actions_scheduled: u64,
        actions_resolved: u64,
        suspensions: u64,
        slots_written: u64,
    ) {
        assert_eq!(summary.steps_started, steps_started, "steps_started");
        assert_eq!(summary.steps_succeeded, steps_succeeded, "steps_succeeded");
        assert_eq!(
            summary.actions_scheduled, actions_scheduled,
            "actions_scheduled"
        );
        assert_eq!(
            summary.actions_resolved, actions_resolved,
            "actions_resolved"
        );
        assert_eq!(summary.suspensions, suspensions, "suspensions");
        assert_eq!(summary.slots_written, slots_written, "slots_written");
    }

    #[test]
    fn ask_answered_event_is_no_op() {
        let mut summary = fresh_summary();
        let event = JournalEvent::AskAnsweredEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 0, 0);
    }

    #[test]
    fn action_failed_event_increments_actions_resolved_only() {
        let mut summary = fresh_summary();
        let event = JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(0),
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 1, 0, 0);
    }

    #[test]
    fn slot_written_event_increments_slots_written_only() {
        let mut summary = fresh_summary();
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 0, 1);
    }

    #[test]
    fn wait_scheduled_event_increments_suspensions() {
        let mut summary = fresh_summary();
        let event = JournalEvent::WaitScheduledEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 1, 0);
    }
}
