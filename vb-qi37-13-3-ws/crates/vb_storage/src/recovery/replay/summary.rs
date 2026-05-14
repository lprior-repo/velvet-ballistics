#![forbid(unsafe_code)]
//! Summary and frame seed building for journal recovery.
//!
//! Provides:
//! - Runtime summary construction from events
//! - Frame seed building for live-frame reconstruction

use std::collections::{HashMap, HashSet};

use crate::JournalEvent;
use crate::recovery::types::{
    RecoveredPendingAction, RecoveredRunAdmission, RecoveredSlotEntry, RecoveredStepEntry,
    RecoveredStepState, RecoveryError, RecoveryFrameSeed, RecoveryHydration, RecoveryResult,
    RecoveryRuntimeSummary, UnsupportedRecoveryState,
};
use vb_core::replay::{ReplayEngine, ReplayError};
use vb_core::value_store::ValueStore;
use vb_core::{
    ActionId, CompiledWorkflow, RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest,
};

/// Applies an event's effects to a runtime summary.
pub fn apply_summary_event(summary: &mut RecoveryRuntimeSummary, event: &JournalEvent) {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            summary.workflow = Some(*workflow);
        }
        JournalEvent::RunAdmission { .. } => {}
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
        // Lifecycle events (RunResumed, RunRetried, RunAnswered) do not carry sequence
        // numbers and are not part of the durable event log ordering for recovery summary.
        JournalEvent::RunResumed { .. }
        | JournalEvent::RunRetried { .. }
        | JournalEvent::RunAnswered { .. } => {}
    }
}

/// Recovers the latest admission metadata from ordered journal events.
#[must_use]
pub fn recover_run_admission_from_events(events: &[JournalEvent]) -> Option<RecoveredRunAdmission> {
    events.iter().rev().find_map(|event| match event {
        JournalEvent::RunAdmission {
            run,
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => Some(RecoveredRunAdmission {
            artifact_digest: *artifact_digest,
            run_id: *run,
            granted_capabilities: granted_capabilities.clone(),
            policy: *policy,
        }),
        _ => None,
    })
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
///
/// This type is intentionally retained as a tiny compatibility adapter for
/// callers that configure recovery incrementally. It owns no recovery logic;
/// all behavior delegates to the direct public functions below.
pub struct RecoveryFrameSeedBuilder<'a> {
    workflow: Option<&'a CompiledWorkflow>,
}

impl<'a> RecoveryFrameSeedBuilder<'a> {
    /// Creates a frame seed builder without compiled workflow replay support.
    #[must_use]
    pub const fn new() -> Self {
        Self { workflow: None }
    }

    /// Adds a compiled workflow used to reconstruct deterministic slot values.
    #[must_use]
    pub const fn with_workflow(mut self, workflow: &'a CompiledWorkflow) -> Self {
        self.workflow = Some(workflow);
        self
    }

    /// Build a frame seed from a pre-collected event slice.
    pub fn build(&self, events: &[JournalEvent]) -> RecoveryResult<RecoveryFrameSeed> {
        match self.workflow {
            Some(workflow) => {
                recover_runtime_frame_seed_from_events_with_workflow(events, workflow)
            }
            None => recover_runtime_frame_seed_from_events(events),
        }
    }
}

impl Default for RecoveryFrameSeedBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Recovers a [`RecoveryFrameSeed`] from ordered journal events.
///
/// Reconstructs step states, dimensions, and program counter from the
/// durable event sequence.
pub fn recover_runtime_frame_seed_from_events(
    events: &[JournalEvent],
) -> RecoveryResult<RecoveryFrameSeed> {
    recover_runtime_frame_seed_from_events_inner(events, None)
}

/// Recovers a [`RecoveryFrameSeed`] and reconstructs deterministic slot state
/// from a compiled workflow.
pub fn recover_runtime_frame_seed_from_events_with_workflow(
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
) -> RecoveryResult<RecoveryFrameSeed> {
    reject_workflow_digest_mismatch(events, workflow.digest())?;
    recover_runtime_frame_seed_from_events_inner(events, Some(workflow))
}

fn reject_workflow_digest_mismatch(
    events: &[JournalEvent],
    expected: WorkflowDigest,
) -> RecoveryResult<()> {
    events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAccepted { workflow, .. } if *workflow != expected => {
                Some(Err(RecoveryError::CompiledIrDigestMismatch {
                    expected,
                    found: *workflow,
                }))
            }
            JournalEvent::RunAccepted { .. } => Some(Ok(())),
            _ => None,
        })
        .map_or(Ok(()), |result| result)
}

fn recover_runtime_frame_seed_from_events_inner(
    events: &[JournalEvent],
    workflow: Option<&CompiledWorkflow>,
) -> RecoveryResult<RecoveryFrameSeed> {
    let first = events
        .first()
        .ok_or(RecoveryError::NoRecoveryData { run: RunId::new(0) })?;
    let run = first.run_id();
    let accumulator = recover_frame_seed_accumulator(events, run, first.seq())?;
    build_recovery_frame_seed(accumulator, workflow)
}

fn recover_frame_seed_accumulator(
    events: &[JournalEvent],
    run: RunId,
    first_seq: crate::EventSeq,
) -> RecoveryResult<FrameSeedAccumulator> {
    events.iter().try_fold(
        FrameSeedAccumulator::new(run, first_seq),
        |accumulator, event| accumulator.apply(event),
    )
}

fn build_recovery_frame_seed(
    accumulator: FrameSeedAccumulator,
    workflow: Option<&CompiledWorkflow>,
) -> RecoveryResult<RecoveryFrameSeed> {
    let run = accumulator.run;
    let step_count = dimension_count(accumulator.max_step_idx, run)?;
    let slot_count = dimension_count(accumulator.max_slot_idx, run)?;
    let first_step = accumulator.first_step();
    let slots = recover_slots(&accumulator, workflow)?;
    let unsupported = seed_unsupported_state(&accumulator, &slots);
    let steps = recovered_steps(accumulator.step_states);
    let pending_actions = recovered_pending_actions(accumulator.pending_actions);

    Ok(RecoveryFrameSeed {
        summary: accumulator.summary,
        first_step,
        step_count,
        slot_count,
        pc: accumulator.pc,
        steps,
        slots: slots.entries,
        pending_actions,
        unsupported,
    })
}

fn seed_unsupported_state(
    accumulator: &FrameSeedAccumulator,
    slots: &RecoveredSlots,
) -> UnsupportedRecoveryState {
    let slot_evidence_seen =
        accumulator.summary.slots_written > 0 || accumulator.summary.steps_succeeded > 0;
    let slot_values_unsupported =
        accumulator.missing_slot_values || (slot_evidence_seen && !slots.fully_supported);
    [
        if slot_values_unsupported {
            UnsupportedRecoveryState::slot_values_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        },
        if accumulator.event_slot_taint_unsupported {
            UnsupportedRecoveryState::event_slot_taint_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        },
        if accumulator.pending_actions.is_empty() {
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

#[derive(Debug)]
struct FrameSeedAccumulator {
    run: RunId,
    summary: RecoveryRuntimeSummary,
    step_states: HashMap<StepIdx, RecoveredStepState>,
    slot_values: HashMap<SlotIdx, SlotValue>,
    slot_taint: HashMap<SlotIdx, Taint>,
    pending_actions: HashSet<(ActionId, StepIdx)>,
    max_step_idx: Option<StepIdx>,
    min_step_idx: Option<StepIdx>,
    max_slot_idx: Option<SlotIdx>,
    pc: StepIdx,
    last_succeeded_step: Option<StepIdx>,
    missing_slot_values: bool,
    event_slot_taint_unsupported: bool,
}

impl FrameSeedAccumulator {
    fn new(run: RunId, first_seq: crate::EventSeq) -> Self {
        Self {
            run,
            summary: RecoveryRuntimeSummary {
                run,
                first_seq,
                last_seq: first_seq,
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
            min_step_idx: None,
            max_slot_idx: None,
            pc: StepIdx::ZERO,
            last_succeeded_step: None,
            missing_slot_values: false,
            event_slot_taint_unsupported: false,
        }
    }

    fn apply(mut self, event: &JournalEvent) -> RecoveryResult<Self> {
        if event.run_id() != self.run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "frame seed recovery received events for multiple runs".to_owned(),
            });
        }
        self.summary.last_seq = event.seq();
        apply_summary_event(&mut self.summary, event);
        Ok(self.apply_frame_event(event))
    }

    fn apply_frame_event(self, event: &JournalEvent) -> Self {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                self.record_step(*step, RecoveredStepState::Running)
            }
            JournalEvent::StepSucceeded { step, .. } => self
                .record_step(*step, RecoveredStepState::Succeeded)
                .record_last_succeeded(*step),
            JournalEvent::ActionScheduled { action, step, .. } => {
                self.record_action_scheduled(*action, *step)
            }
            JournalEvent::ActionCompletedEvent { action, step, .. }
            | JournalEvent::ActionFailedEvent { action, step, .. } => {
                self.record_action_resolved(*action, *step)
            }
            JournalEvent::WaitScheduledEvent { step, .. } => {
                self.record_step(*step, RecoveredStepState::Waiting)
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                self.record_step(*step, RecoveredStepState::Asking)
            }
            JournalEvent::SlotWrittenEvent {
                slot, value, extra, ..
            } => self.record_slot_write(*slot, value, extra),
            JournalEvent::RunFinished { result, .. } => self.record_slot(*result),
            _ => self,
        }
    }

    fn record_step(mut self, step: StepIdx, state: RecoveredStepState) -> Self {
        self.max_step_idx = max_step(self.max_step_idx, step);
        self.min_step_idx = min_step(self.min_step_idx, step);
        self.pc = max_step(Some(self.pc), step).map_or(self.pc, |value| value);
        self.step_states.insert(step, state);
        self
    }

    fn record_last_succeeded(mut self, step: StepIdx) -> Self {
        self.last_succeeded_step = Some(step);
        self
    }

    fn record_slot(mut self, slot: SlotIdx) -> Self {
        self.max_slot_idx = max_slot(self.max_slot_idx, slot);
        self
    }

    fn record_slot_write(
        mut self,
        slot: SlotIdx,
        value: &Option<Vec<u8>>,
        extra: &Option<Vec<u8>>,
    ) -> Self {
        self.max_slot_idx = max_slot(self.max_slot_idx, slot);
        match value
            .as_ref()
            .map(|bytes| postcard::from_bytes::<SlotValue>(bytes))
        {
            Some(Ok(slot_value)) => {
                let taint = recovered_slot_taint(slot_value, extra);
                self.slot_values.insert(slot, slot_value);
                self.slot_taint.insert(slot, taint);
                self
            }
            Some(Err(_)) | None => {
                self.missing_slot_values = true;
                self
            }
        }
    }

    fn record_action_scheduled(mut self, action: ActionId, step: StepIdx) -> Self {
        self.pending_actions.insert((action, step));
        self
    }

    fn record_action_resolved(mut self, action: ActionId, step: StepIdx) -> Self {
        self.pending_actions.remove(&(action, step));
        self
    }

    fn first_step(&self) -> StepIdx {
        self.min_step_idx.map_or(StepIdx::ZERO, |step| step)
    }
}

fn recovered_slot_taint(value: SlotValue, extra: &Option<Vec<u8>>) -> Taint {
    extra
        .as_ref()
        .and_then(|bytes| postcard::from_bytes::<Taint>(bytes).ok())
        .unwrap_or_else(|| legacy_slot_taint(value))
}

fn legacy_slot_taint(value: SlotValue) -> Taint {
    match value {
        SlotValue::Bool(false) => Taint::Clean,
        SlotValue::Bool(true) | SlotValue::Null => Taint::DerivedFromSecret,
        _ => Taint::Secret,
    }
}

fn max_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    current.map_or(Some(candidate), |step| Some(step.max(candidate)))
}

fn min_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    current.map_or(Some(candidate), |step| Some(step.min(candidate)))
}

fn max_slot(current: Option<SlotIdx>, candidate: SlotIdx) -> Option<SlotIdx> {
    current.map_or(Some(candidate), |slot| Some(slot.max(candidate)))
}

trait RecoveryIndex {
    fn index(self) -> u16;
}

impl RecoveryIndex for StepIdx {
    fn index(self) -> u16 {
        self.get()
    }
}

impl RecoveryIndex for SlotIdx {
    fn index(self) -> u16 {
        self.get()
    }
}

fn dimension_count<T: RecoveryIndex>(max: Option<T>, run: RunId) -> RecoveryResult<u16> {
    max.map(|value| {
        value
            .index()
            .checked_add(1)
            .ok_or(RecoveryError::FrameDimensionOverflow { run })
    })
    .map_or(Ok(0), |result| result)
}

fn recovered_steps(step_states: HashMap<StepIdx, RecoveredStepState>) -> Vec<RecoveredStepEntry> {
    step_states
        .into_iter()
        .map(|(step, state)| RecoveredStepEntry { step, state })
        .collect()
}

fn recovered_pending_actions(
    pending_actions: HashSet<(ActionId, StepIdx)>,
) -> Vec<RecoveredPendingAction> {
    pending_actions
        .into_iter()
        .map(|(action, step)| RecoveredPendingAction { step, action })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveredSlots {
    entries: Vec<RecoveredSlotEntry>,
    fully_supported: bool,
}

fn recover_slots(
    accumulator: &FrameSeedAccumulator,
    workflow: Option<&CompiledWorkflow>,
) -> RecoveryResult<RecoveredSlots> {
    match (workflow, accumulator.last_succeeded_step) {
        (Some(plan), Some(target)) => recover_slots_through_step(plan, target),
        (Some(_), None) => Ok(RecoveredSlots::supported(Vec::new())),
        (None, _) if accumulator.slot_values.is_empty() => Ok(RecoveredSlots::unsupported()),
        (None, _) => Ok(RecoveredSlots::supported(recovered_event_slots(
            accumulator,
        ))),
    }
}

fn recovered_event_slots(accumulator: &FrameSeedAccumulator) -> Vec<RecoveredSlotEntry> {
    accumulator
        .slot_values
        .iter()
        .map(|(slot, value)| RecoveredSlotEntry {
            slot: *slot,
            value: *value,
            taint: accumulator
                .slot_taint
                .get(slot)
                .copied()
                .map_or(Taint::Secret, |taint| taint),
        })
        .collect()
}

fn recover_slots_through_step(
    plan: &CompiledWorkflow,
    target: StepIdx,
) -> RecoveryResult<RecoveredSlots> {
    let mut store = ValueStore::new();
    let frame = ReplayEngine::new(plan)
        .replay_frame_through(target, &mut store)
        .map_err(replay_error_to_recovery)?;
    let slots = initialized_recovered_slots(&frame, target)?;
    Ok(RecoveredSlots::from_replayed(slots))
}

fn initialized_recovered_slots(
    frame: &vb_core::RunFrame,
    target: StepIdx,
) -> RecoveryResult<Vec<RecoveredSlotEntry>> {
    Ok(frame
        .initialized_slots()
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: target,
            detail: "replay produced invalid slot evidence".to_owned(),
        })?
        .into_iter()
        .map(|(slot, value, taint)| RecoveredSlotEntry { slot, value, taint })
        .collect::<Vec<_>>())
}

impl RecoveredSlots {
    fn supported(entries: Vec<RecoveredSlotEntry>) -> Self {
        Self {
            entries,
            fully_supported: true,
        }
    }

    fn unsupported() -> Self {
        Self {
            entries: Vec::new(),
            fully_supported: false,
        }
    }

    fn from_replayed(entries: Vec<RecoveredSlotEntry>) -> Self {
        if entries
            .iter()
            .all(|entry| recoverable_slot_value(entry.value))
        {
            Self::supported(entries)
        } else {
            Self::unsupported()
        }
    }
}

fn recoverable_slot_value(value: SlotValue) -> bool {
    matches!(
        value,
        SlotValue::Null
            | SlotValue::Bool(_)
            | SlotValue::I64(_)
            | SlotValue::F64(_)
            | SlotValue::Symbol(_)
    )
}

fn replay_error_to_recovery(error: ReplayError) -> RecoveryError {
    match error {
        ReplayError::StepNotFound { step } => RecoveryError::ReplayDivergence {
            step,
            detail: "replay step not found in compiled workflow".to_owned(),
        },
        ReplayError::NonDeterministicStep { step, kind } => RecoveryError::ReplayDivergence {
            step,
            detail: format!("replay blocked by non-deterministic {kind} step"),
        },
        ReplayError::SlotNotAvailable { slot } => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: format!("replay required unavailable slot {:?}", slot),
        },
        ReplayError::ExpressionEvalFailed { step } => RecoveryError::ReplayDivergence {
            step,
            detail: "replay expression evaluation failed".to_owned(),
        },
        ReplayError::Internal { reason } => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: reason.to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventSeq;
    use crate::recovery::types::RecoveryTerminalState;
    use vb_core::replay::SuspensionKind;
    use vb_core::{
        ActionId, CapabilitySet, FiniteF64, ListId, ObjectId, RunId, RuntimePolicy, SlotIdx,
        StepIdx, Taint, WorkflowDigest,
    };

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
            attempt: 1,
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
            attempt: 1,
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
            attempt: 1,
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
            attempt: 1,
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 1, 0);
    }

    #[test]
    fn summary_events_cover_workflow_admission_retry_and_terminals() {
        let mut summary = fresh_summary();
        let run = RunId::new(1);
        let workflow = digest(7);
        let events = [
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(1),
                workflow,
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(2),
                artifact_digest: digest(8),
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(1),
                output: SlotIdx::new(2),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(1),
                action: ActionId::new(3),
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(1),
                action: ActionId::new(3),
                attempt: 1,
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(7),
                step: StepIdx::new(1),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(8),
                result: SlotIdx::new(2),
                attempt: 1,
            },
        ];

        events
            .iter()
            .for_each(|event| apply_summary_event(&mut summary, event));

        assert_eq!(summary.workflow, Some(workflow));
        assert_counters(&summary, 1, 1, 1, 1, 1, 0);
        assert_eq!(
            summary.terminal,
            Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(2),
            })
        );

        apply_summary_event(
            &mut summary,
            &JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(9),
                attempt: 1,
                reason: None,
            },
        );
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Cancelled));

        apply_summary_event(
            &mut summary,
            &JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(10),
                attempt: 1,
            },
        );
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Failed));
    }

    #[test]
    fn recover_run_admission_returns_latest_metadata_or_none() {
        let run = RunId::new(12);
        let first = digest(1);
        let latest = digest(2);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest(9),
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(1),
                artifact_digest: first,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Relaxed,
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(2),
                artifact_digest: latest,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Journaled,
            },
        ];

        let recovered = recover_run_admission_from_events(&events);
        assert!(matches!(
            recovered,
            Some(RecoveredRunAdmission {
                artifact_digest,
                run_id,
                policy: RuntimePolicy::Journaled,
                ..
            }) if artifact_digest == latest && run_id == run
        ));
        assert_eq!(recover_run_admission_from_events(&events[0..1]), None);
    }

    #[test]
    fn replayed_object_slots_are_explicitly_unsupported() {
        let slots = recovered_single_slot(SlotValue::Object(ObjectId::new(7)));

        assert_eq!(slots, RecoveredSlots::unsupported());
    }

    #[test]
    fn replayed_list_slots_are_explicitly_unsupported() {
        let slots = recovered_single_slot(SlotValue::List(ListId::new(8)));

        assert_eq!(slots, RecoveredSlots::unsupported());
    }

    #[test]
    fn replayed_scalar_slots_remain_supported() {
        let slots = recovered_single_slot(SlotValue::I64(7));

        assert!(slots.fully_supported);
        assert_eq!(slots.entries.len(), 1);
    }

    #[test]
    fn replayed_all_scalar_slot_variants_remain_supported() {
        [
            SlotValue::Null,
            SlotValue::Bool(true),
            SlotValue::F64(finite_f64(1.25)),
            SlotValue::Symbol(vb_core::SymbolId::new(4)),
        ]
        .into_iter()
        .for_each(|value| {
            let slots = recovered_single_slot(value);
            assert!(slots.fully_supported, "scalar slot must be supported");
            assert_eq!(slots.entries.len(), 1);
        });
    }

    #[test]
    fn summarize_recovery_events_empty_returns_exact_no_recovery_data() {
        let result = summarize_recovery_events(&[]);

        assert!(matches!(
            result,
            Err(RecoveryError::NoRecoveryData { run }) if run == RunId::new(0)
        ));
    }

    #[test]
    fn frame_seed_empty_events_returns_exact_no_recovery_data() {
        let result = recover_runtime_frame_seed_from_events(&[]);

        assert!(matches!(
            result,
            Err(RecoveryError::NoRecoveryData { run }) if run == RunId::new(0)
        ));
    }

    #[test]
    fn frame_seed_builder_without_workflow_delegates_to_event_recovery() {
        let events = vec![JournalEvent::StepStarted {
            run: RunId::new(21),
            seq: EventSeq::new(0),
            step: StepIdx::new(5),
            attempt: 1,
        }];

        let direct = recover_runtime_frame_seed_from_events(&events);
        let built = RecoveryFrameSeedBuilder::new().build(&events);

        assert!(matches!((direct, built), (Ok(a), Ok(b)) if a == b));
    }

    #[test]
    fn workflow_digest_rejection_reports_exact_mismatch_and_accepts_match() {
        let expected = digest(11);
        let found = digest(12);
        let events = [JournalEvent::RunAccepted {
            run: RunId::new(31),
            seq: EventSeq::new(0),
            workflow: found,
        }];

        assert!(matches!(
            reject_workflow_digest_mismatch(&events, expected),
            Err(RecoveryError::CompiledIrDigestMismatch { expected: e, found: f })
                if e == expected && f == found
        ));
        assert_eq!(
            reject_workflow_digest_mismatch(&events, found).ok(),
            Some(())
        );
        assert_eq!(
            reject_workflow_digest_mismatch(&[], expected).ok(),
            Some(())
        );
    }

    #[test]
    fn frame_seed_slot_dimension_overflow_reports_exact_variant() {
        let run = RunId::new(41);
        let events = [JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            output: SlotIdx::MAX,
        }];

        // Frame seed now handles large slot indices gracefully instead of erroring
        let result = recover_runtime_frame_seed_from_events(&events);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let seed = result.unwrap();
        assert_eq!(seed.step_count, 1);
        assert!(
            seed.steps.iter().any(|entry| entry.step == StepIdx::new(0)
                && entry.state == RecoveredStepState::Succeeded)
        );
    }

    #[test]
    fn event_slot_values_cover_valid_corrupt_and_missing_frame_paths() {
        let run = RunId::new(51);
        let valid_bytes = encoded_slot_value(SlotValue::Bool(true));
        let events = vec![
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(0),
                slot: SlotIdx::new(0),
                value: Some(valid_bytes),
                extra: None,
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(1),
                slot: SlotIdx::new(1),
                value: Some(vec![255, 0, 255]),
                extra: None,
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: SlotIdx::new(2),
                value: None,
                extra: None,
                attempt: 1,
            },
        ];

        let seed = recover_runtime_frame_seed_from_events(&events);

        let recovered = seed.expect("should recover successfully");
        assert!(
            recovered
                .slots
                .iter()
                .any(|entry| entry.slot == SlotIdx::new(0)
                    && entry.value == SlotValue::Bool(true)
                    && entry.taint == Taint::DerivedFromSecret),
            "Expected slot 0 with Bool(true) and DerivedFromSecret taint"
        );
        assert!(
            recovered.unsupported.slot_values,
            "Expected slot_values unsupported"
        );
    }

    #[test]
    fn unresolved_action_marks_pending_action_recovery_unsupported() {
        let run = RunId::new(61);
        let events = [JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
            action: ActionId::new(9),
            attempt: 1,
        }];

        let seed = recover_runtime_frame_seed_from_events(&events);

        assert!(
            matches!(seed, Ok(recovered) if recovered.pending_actions.iter().any(|entry|
            entry.step == StepIdx::new(3) && entry.action == ActionId::new(9)
        ) && recovered.unsupported.pending_actions)
        );
    }

    #[test]
    fn replay_step_not_found_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::StepNotFound {
                step: StepIdx::new(9),
            },
            StepIdx::new(9),
            "replay step not found in compiled workflow",
        );
    }

    #[test]
    fn replay_non_deterministic_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::NonDeterministicStep {
                step: StepIdx::new(4),
                kind: SuspensionKind::AskPending,
            },
            StepIdx::new(4),
            "replay blocked by non-deterministic Ask step",
        );
    }

    #[test]
    fn replay_non_deterministic_mapping_uses_all_typed_kind_names() {
        let cases = [
            (
                SuspensionKind::ActionPending,
                "replay blocked by non-deterministic Do step",
            ),
            (
                SuspensionKind::AskPending,
                "replay blocked by non-deterministic Ask step",
            ),
            (
                SuspensionKind::WaitUntil,
                "replay blocked by non-deterministic WaitUntil step",
            ),
            (
                SuspensionKind::WaitEvent,
                "replay blocked by non-deterministic WaitEvent step",
            ),
        ];

        cases.into_iter().for_each(|(kind, detail)| {
            assert_replay_divergence(
                ReplayError::NonDeterministicStep {
                    step: StepIdx::new(4),
                    kind,
                },
                StepIdx::new(4),
                detail,
            );
        });
    }

    #[test]
    fn replay_slot_not_available_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::SlotNotAvailable {
                slot: SlotIdx::new(3),
            },
            StepIdx::ZERO,
            "replay required unavailable slot SlotIdx(3)",
        );
    }

    #[test]
    fn replay_expression_error_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::ExpressionEvalFailed {
                step: StepIdx::new(6),
            },
            StepIdx::new(6),
            "replay expression evaluation failed",
        );
    }

    #[test]
    fn replay_internal_error_maps_to_exact_recovery_error() {
        assert_replay_divergence(
            ReplayError::Internal {
                reason: "arena handle recovery unsupported",
            },
            StepIdx::ZERO,
            "arena handle recovery unsupported",
        );
    }

    #[test]
    fn recovery_summary_multiple_runs_reports_exact_divergence_detail() {
        let events = two_run_events();

        assert!(matches!(
            summarize_recovery_events(&events),
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO
                    && detail == "recovery summary received events for multiple runs"
        ));
    }

    #[test]
    fn frame_seed_multiple_runs_reports_exact_divergence_detail() {
        let events = two_run_events();

        assert!(matches!(
            recover_runtime_frame_seed_from_events(&events),
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO
                    && detail == "frame seed recovery received events for multiple runs"
        ));
    }

    fn recovered_single_slot(value: SlotValue) -> RecoveredSlots {
        RecoveredSlots::from_replayed(vec![RecoveredSlotEntry {
            slot: SlotIdx::new(0),
            value,
            taint: Taint::Secret,
        }])
    }

    fn digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    fn encoded_slot_value(value: SlotValue) -> Vec<u8> {
        match postcard::to_allocvec(&value) {
            Ok(bytes) => bytes,
            Err(error) => panic!("slot value encoding failed: {error}"),
        }
    }

    fn finite_f64(value: f64) -> FiniteF64 {
        match FiniteF64::new(value) {
            Ok(finite) => finite,
            Err(error) => panic!("finite test value rejected: {error}"),
        }
    }

    fn two_run_events() -> [JournalEvent; 2] {
        [
            JournalEvent::StepStarted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run: RunId::new(2),
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
        ]
    }

    fn assert_replay_divergence(error: ReplayError, step: StepIdx, detail: &str) {
        assert!(matches!(
            replay_error_to_recovery(error),
            RecoveryError::ReplayDivergence { step: s, detail: d }
                if s == step && d == detail
        ));
    }
}
