#![forbid(unsafe_code)]
//! Summary and frame seed building for journal recovery.
//!
//! Provides:
//! - Runtime summary construction from events
//! - Frame seed building for live-frame reconstruction

use std::collections::{HashMap, HashSet};

use crate::recovery::hydrate_support::{
    verified_action_envelope_digest, verify_action_ticket_event,
};
use crate::recovery::types::{
    ActionReplayEffect, ActionReplayTracker, RecoveredPendingAction, RecoveredRunAdmission,
    RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryError, RecoveryFrameSeed,
    RecoveryHydration, RecoveryResult, RecoveryRuntimeSummary, UnsupportedRecoveryState,
};
use crate::slot_extra::DecodedSlotWrittenExtra;
use crate::{EventSeq, JournalEvent};
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
        JournalEvent::ActionScheduledTicket { .. } => {
            summary.actions_scheduled = summary.actions_scheduled.saturating_add(1);
        }
        JournalEvent::ActionCompletedEvent { .. } | JournalEvent::ActionFailedEvent { .. } => {
            summary.actions_resolved = summary.actions_resolved.saturating_add(1);
        }
        JournalEvent::ActionCompletedEnvelope { .. } => {
            summary.actions_resolved = summary.actions_resolved.saturating_add(1);
            summary.steps_succeeded = summary.steps_succeeded.saturating_add(1);
            summary.slots_written = summary.slots_written.saturating_add(1);
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
        JournalEvent::AskTimedOutEvent { .. } => {
            summary.steps_succeeded = summary.steps_succeeded.saturating_add(1);
        }
        JournalEvent::RunCancelled { .. } => {
            summary.terminal = Some(crate::recovery::types::RecoveryTerminalState::Cancelled);
        }
        JournalEvent::RunKilled { .. } => {
            summary.terminal = Some(crate::recovery::types::RecoveryTerminalState::Killed);
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
    let mut tracker = ActionReplayTracker::new();

    for event in events {
        if event.run_id() != run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "recovery summary received events for multiple runs".to_owned(),
            });
        }
        if event.seq() == EventSeq::MAX {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "overflow sentinel sequence {} is not valid",
                    event.seq().get()
                ),
            });
        }
        summary.last_seq = event.seq();
        apply_summary_event_checked(&mut summary, event, &mut tracker)?;
    }

    Ok(RecoveryHydration::Summary(summary))
}

fn apply_summary_event_checked(
    summary: &mut RecoveryRuntimeSummary,
    event: &JournalEvent,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<()> {
    match event {
        JournalEvent::ActionScheduled { action, step, .. } => {
            reject_resolved_summary_action(tracker, *action, *step)?;
            apply_summary_event(summary, event);
            Ok(())
        }
        JournalEvent::ActionScheduledTicket {
            run,
            ticket,
            input,
            output,
            ..
        } => {
            verify_action_ticket_event(*run, *ticket)?;
            let effect = tracker.mark_scheduled_ticket_effect(*ticket, *input, *output)?;
            if effect == ActionReplayEffect::Apply {
                apply_summary_event(summary, event);
            }
            Ok(())
        }
        JournalEvent::ActionCompletedEvent { action, step, .. } => {
            reject_resolved_summary_action(tracker, *action, *step)?;
            tracker.mark_completed(*action, *step);
            apply_summary_event(summary, event);
            Ok(())
        }
        JournalEvent::ActionFailedEvent { action, step, .. } => {
            reject_resolved_summary_action(tracker, *action, *step)?;
            tracker.mark_failed(*action, *step);
            apply_summary_event(summary, event);
            Ok(())
        }
        JournalEvent::ActionCompletedEnvelope {
            run,
            ticket,
            output,
            outcome,
            value,
            encoded_len,
            taint,
            value_digest,
            ..
        } => {
            let verified_digest = verified_action_envelope_digest(
                *run,
                *ticket,
                *outcome,
                value,
                *encoded_len,
                *value_digest,
            )?;
            tracker.require_scheduled_ticket(*ticket, *output)?;
            let effect = tracker.mark_completed_envelope_effect(
                *ticket,
                *output,
                *encoded_len,
                *taint,
                verified_digest,
            )?;
            if effect == ActionReplayEffect::Apply {
                apply_summary_event(summary, event);
            }
            Ok(())
        }
        _ => {
            apply_summary_event(summary, event);
            Ok(())
        }
    }
}

fn reject_resolved_summary_action(
    tracker: &ActionReplayTracker,
    action: ActionId,
    step: StepIdx,
) -> RecoveryResult<()> {
    if tracker.is_resolved(action, step) {
        return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
    }
    Ok(())
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
    action_tracker: ActionReplayTracker,
    max_step_idx: Option<StepIdx>,
    min_step_idx: Option<StepIdx>,
    max_slot_idx: Option<SlotIdx>,
    pc: StepIdx,
    last_succeeded_step: Option<StepIdx>,
    missing_slot_values: bool,
    event_slot_taint_unsupported: bool,
}

#[derive(Debug, Clone, Copy)]
struct ActionEnvelopeView<'a> {
    run: RunId,
    ticket: vb_core::ActionTicket,
    output: SlotIdx,
    outcome: crate::DurableActionOutcome,
    value: &'a [u8],
    encoded_len: u32,
    taint: Taint,
    value_digest: [u8; 32],
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
            action_tracker: ActionReplayTracker::new(),
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
        if event.seq() == EventSeq::MAX {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "overflow sentinel sequence {} is not valid",
                    event.seq().get()
                ),
            });
        }
        self.summary.last_seq = event.seq();
        if !matches!(
            event,
            JournalEvent::ActionCompletedEnvelope { .. }
                | JournalEvent::ActionScheduledTicket { .. }
        ) {
            apply_summary_event(&mut self.summary, event);
        }
        self.apply_frame_event(event)
    }

    fn apply_frame_event(self, event: &JournalEvent) -> RecoveryResult<Self> {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                Ok(self.record_step(*step, RecoveredStepState::Running))
            }
            JournalEvent::StepSucceeded { step, .. } => Ok(self
                .record_step(*step, RecoveredStepState::Succeeded)
                .record_last_succeeded(*step)),
            JournalEvent::ActionScheduled { action, step, .. } => {
                self.record_action_scheduled(*action, *step)
            }
            JournalEvent::ActionScheduledTicket {
                run,
                ticket,
                input,
                output,
                ..
            } => {
                verify_action_ticket_event(*run, *ticket)?;
                self.record_action_scheduled_ticket(*ticket, *input, *output)
            }
            JournalEvent::ActionCompletedEvent { action, step, .. } => {
                self.record_action_completed(*action, *step)
            }
            JournalEvent::ActionFailedEvent { action, step, .. } => {
                self.record_action_failed(*action, *step)
            }
            JournalEvent::ActionCompletedEnvelope {
                run,
                ticket,
                output,
                outcome,
                value,
                encoded_len,
                taint,
                value_digest,
                ..
            } => self.record_action_completion_envelope(ActionEnvelopeView {
                run: *run,
                ticket: *ticket,
                output: *output,
                outcome: *outcome,
                value,
                encoded_len: *encoded_len,
                taint: *taint,
                value_digest: *value_digest,
            }),
            JournalEvent::WaitScheduledEvent { step, .. } => {
                Ok(self.record_step(*step, RecoveredStepState::Waiting))
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                Ok(self.record_step(*step, RecoveredStepState::Asking))
            }
            JournalEvent::AskTimedOutEvent { step, .. } => Ok(self
                .record_step(*step, RecoveredStepState::Succeeded)
                .record_last_succeeded(*step)),
            JournalEvent::SlotWrittenEvent {
                slot, value, extra, ..
            } => self.record_slot_write(*slot, value, extra),
            JournalEvent::RunFinished { result, .. } => Ok(self.record_slot(*result)),
            _ => Ok(self),
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
    ) -> RecoveryResult<Self> {
        self.max_slot_idx = max_slot(self.max_slot_idx, slot);
        match value
            .as_ref()
            .map(|bytes| postcard::from_bytes::<SlotValue>(bytes))
        {
            Some(Ok(slot_value)) => {
                let recovered_taint = recovered_slot_taint(slot, slot_value, extra)?;
                self.slot_values.insert(slot, slot_value);
                self.slot_taint.insert(slot, recovered_taint.taint);
                self.event_slot_taint_unsupported |= recovered_taint.unsupported;
                Ok(self)
            }
            Some(Err(_)) | None => {
                self.missing_slot_values = true;
                Ok(self)
            }
        }
    }

    fn record_action_completion_envelope(
        mut self,
        envelope: ActionEnvelopeView<'_>,
    ) -> RecoveryResult<Self> {
        let verified_digest = verified_action_envelope_digest(
            envelope.run,
            envelope.ticket,
            envelope.outcome,
            envelope.value,
            envelope.encoded_len,
            envelope.value_digest,
        )?;
        self.action_tracker
            .require_scheduled_ticket(envelope.ticket, envelope.output)?;
        let effect = self.action_tracker.mark_completed_envelope_effect(
            envelope.ticket,
            envelope.output,
            envelope.encoded_len,
            envelope.taint,
            verified_digest,
        )?;
        if effect == ActionReplayEffect::Duplicate {
            return Ok(self);
        }
        self.summary.actions_resolved = self.summary.actions_resolved.saturating_add(1);
        self.summary.steps_succeeded = self.summary.steps_succeeded.saturating_add(1);
        self.summary.slots_written = self.summary.slots_written.saturating_add(1);
        self.pending_actions
            .remove(&(envelope.ticket.action, envelope.ticket.step));
        self.record_step(envelope.ticket.step, RecoveredStepState::Succeeded)
            .record_last_succeeded(envelope.ticket.step)
            .record_envelope_slot(envelope.output, envelope.value, envelope.taint)
    }

    fn record_envelope_slot(
        mut self,
        slot: SlotIdx,
        value: &[u8],
        taint: Taint,
    ) -> RecoveryResult<Self> {
        self.max_slot_idx = max_slot(self.max_slot_idx, slot);
        match postcard::from_bytes::<SlotValue>(value) {
            Ok(slot_value) => {
                self.slot_values.insert(slot, slot_value);
                self.slot_taint.insert(slot, taint);
                Ok(self)
            }
            Err(_) => Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!("slot value decode failed for slot {:?}", slot),
            }),
        }
    }

    fn record_action_scheduled(mut self, action: ActionId, step: StepIdx) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.pending_actions.insert((action, step));
        Ok(self)
    }

    fn record_action_scheduled_ticket(
        mut self,
        ticket: vb_core::ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
    ) -> RecoveryResult<Self> {
        let effect = self
            .action_tracker
            .mark_scheduled_ticket_effect(ticket, input, output)?;
        if effect == ActionReplayEffect::Apply {
            self.summary.actions_scheduled = self.summary.actions_scheduled.saturating_add(1);
            self.pending_actions.insert((ticket.action, ticket.step));
        }
        Ok(self)
    }

    fn record_action_completed(mut self, action: ActionId, step: StepIdx) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.action_tracker.mark_completed(action, step);
        self.pending_actions.remove(&(action, step));
        Ok(self)
    }

    fn record_action_failed(mut self, action: ActionId, step: StepIdx) -> RecoveryResult<Self> {
        if self.action_tracker.is_resolved(action, step) {
            return Err(RecoveryError::NonIdempotentActionBlocked { action, step });
        }
        self.action_tracker.mark_failed(action, step);
        self.pending_actions.remove(&(action, step));
        Ok(self)
    }

    fn first_step(&self) -> StepIdx {
        self.min_step_idx.map_or(StepIdx::ZERO, |step| step)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecoveredSlotTaint {
    taint: Taint,
    unsupported: bool,
}

fn recovered_slot_taint(
    slot: SlotIdx,
    value: SlotValue,
    extra: &Option<Vec<u8>>,
) -> RecoveryResult<RecoveredSlotTaint> {
    match extra {
        Some(bytes) => decoded_slot_taint(slot, value, bytes),
        None => Ok(legacy_recovered_slot_taint(value)),
    }
}

fn decoded_slot_taint(
    slot: SlotIdx,
    value: SlotValue,
    bytes: &[u8],
) -> RecoveryResult<RecoveredSlotTaint> {
    match crate::slot_extra::decode_slot_written_extra(bytes) {
        Ok(DecodedSlotWrittenExtra::Envelope(envelope)) => Ok(RecoveredSlotTaint {
            taint: envelope.taint,
            unsupported: false,
        }),
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_)) => {
            Ok(legacy_frame_extra_recovered_slot_taint(value))
        }
        Err(_) => Err(RecoveryError::CorruptSlotTaint { slot }),
    }
}

fn legacy_recovered_slot_taint(value: SlotValue) -> RecoveredSlotTaint {
    RecoveredSlotTaint {
        taint: legacy_slot_taint(value),
        unsupported: false,
    }
}

fn legacy_frame_extra_recovered_slot_taint(value: SlotValue) -> RecoveredSlotTaint {
    RecoveredSlotTaint {
        taint: legacy_slot_taint(value),
        unsupported: true,
    }
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

/// Production proof surface for turning a maximum zero-based dimension into a count.
pub fn recovery_dimension_count_from_index(
    max_index: Option<u16>,
    run: RunId,
) -> RecoveryResult<u16> {
    max_index
        .map(|value| {
            value
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .map_or(Ok(0), |result| result)
}

/// Production proof surface for successful non-empty/evidence-bearing seed dimensions.
#[must_use]
pub const fn recovery_seed_dimensions_positive(seed: &RecoveryFrameSeed) -> bool {
    seed.step_count > 0 && seed.slot_count > 0
}

/// Production proof surface for an observed dimension requiring positive count.
#[must_use]
pub const fn recovery_observed_dimension_is_positive(max_index: Option<u16>, count: u16) -> bool {
    match max_index {
        Some(_) => count > 0,
        None => count == 0,
    }
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
        (Some(plan), Some(target)) => Ok(merge_recovered_slots(
            recover_slots_through_step(plan, target)?,
            recovered_event_slots(accumulator),
        )),
        (Some(_), None) => Ok(RecoveredSlots::supported(Vec::new())),
        (None, _) if accumulator.slot_values.is_empty() => Ok(RecoveredSlots::unsupported()),
        (None, _) => Ok(RecoveredSlots::supported(recovered_event_slots(
            accumulator,
        ))),
    }
}

fn merge_recovered_slots(
    mut base: RecoveredSlots,
    overrides: Vec<RecoveredSlotEntry>,
) -> RecoveredSlots {
    for override_entry in overrides {
        match base
            .entries
            .iter_mut()
            .find(|entry| entry.slot == override_entry.slot)
        {
            Some(entry) => *entry = override_entry,
            None => base.entries.push(override_entry),
        }
    }
    base
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
    let engine = ReplayEngine::new(plan);
    let frame = match engine.replay_frame_through(target, &mut store) {
        Ok(frame) => frame,
        Err(ReplayError::NonDeterministicStep { step, .. }) if step == target => {
            let mut store = ValueStore::new();
            engine
                .replay_frame_up_to(target, &mut store)
                .map_err(replay_error_to_recovery)?
        }
        Err(error) => return Err(replay_error_to_recovery(error)),
    };
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
        // `ReplayError` is `#[non_exhaustive]`; unknown variants
        // map to a generic replay divergence error.
        _ => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: "unknown replay error".to_owned(),
        },
    }
}

#[cfg(test)]
#[path = "summary/tests.rs"]
mod tests;
