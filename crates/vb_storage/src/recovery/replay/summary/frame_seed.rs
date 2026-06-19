#![forbid(unsafe_code)]
//! Frame seed accumulator, envelope view, and frame seed recovery.
//!
//! Provides:
//! - `RecoveryFrameSeedBuilder` — compatibility adapter for incremental recovery
//! - `FrameSeedAccumulator` — event → frame seed state machine
//! - `ActionEnvelopeView` — extracted view of `ActionCompletedEnvelope` data
//! - `recover_runtime_frame_seed_from_events` — public frame seed recovery
//! - `recover_runtime_frame_seed_from_events_with_workflow` — workflow-backed recovery
//! - `reject_workflow_digest_mismatch` — digest validation

use std::collections::{HashMap, HashSet};

use super::runtime_summary::apply_summary_event;
use crate::recovery::action_digest::{verified_action_envelope_digest, verify_action_ticket_event};
use crate::recovery::types::ActionReplayEffect;
use crate::recovery::{
    ActionReplayTracker, RecoveryError, RecoveryFrameSeed, RecoveryResult, RecoveryRuntimeSummary,
    UnsupportedRecoveryState,
};
use crate::{EventSeq, JournalEvent};
use vb_core::replay::ReplayEngine;
use vb_core::value_store::ValueStore;
use vb_core::{
    ActionId, CompiledWorkflow, RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest,
};

// ── RecoveryFrameSeedBuilder ────────────────────────────────────────────────

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

// ── Public frame seed recovery ──────────────────────────────────────────────

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

/// Validates that the workflow digest in the first accepted run event matches
/// the expected compiled workflow digest.
pub fn reject_workflow_digest_mismatch(
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

// ── Frame seed construction helpers ─────────────────────────────────────────

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
    let slots = crate::recovery::replay::summary::slots::recover_slots(&accumulator, workflow)?;
    let unsupported = seed_unsupported_state(&accumulator, &slots);
    let steps = recovered_steps(accumulator.step_states);

    Ok(RecoveryFrameSeed {
        summary: accumulator.summary,
        first_step,
        step_count,
        slot_count,
        pc: accumulator.pc,
        steps,
        slots: slots.entries,
        unsupported,
    })
}

fn seed_unsupported_state(
    accumulator: &FrameSeedAccumulator,
    slots: &crate::recovery::replay::summary::slots::RecoveredSlots,
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
        if accumulator.envelope_event_seen {
            // Ticket envelopes carry action payload bodies that the current runtime
            // rehydration boundary cannot re-attach to a live frame, so the seed
            // must explicitly mark these as unsupported.
            UnsupportedRecoveryState::action_payloads_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        },
    ]
    .into_iter()
    .fold(
        UnsupportedRecoveryState::SUPPORTED,
        UnsupportedRecoveryState::union,
    )
}

// ── ActionEnvelopeView ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(super) struct ActionEnvelopeView<'a> {
    pub(super) run: RunId,
    pub(super) ticket: vb_core::ActionTicket,
    pub(super) output: SlotIdx,
    pub(super) outcome: crate::DurableActionOutcome,
    pub(super) value: &'a [u8],
    pub(super) encoded_len: u32,
    pub(super) taint: Taint,
    pub(super) value_digest: [u8; 32],
}

// ── FrameSeedAccumulator ────────────────────────────────────────────────────

#[derive(Debug)]
pub(super) struct FrameSeedAccumulator {
    pub(super) run: RunId,
    pub(super) summary: RecoveryRuntimeSummary,
    pub(super) step_states: HashMap<StepIdx, crate::recovery::RecoveredStepState>,
    pub(crate) slot_values: HashMap<SlotIdx, SlotValue>,
    pub(crate) slot_taint: HashMap<SlotIdx, Taint>,
    pending_actions: HashSet<(ActionId, StepIdx)>,
    action_tracker: ActionReplayTracker,
    pub(super) max_step_idx: Option<StepIdx>,
    min_step_idx: Option<StepIdx>,
    pub(super) max_slot_idx: Option<SlotIdx>,
    pub(super) pc: StepIdx,
    pub(super) last_succeeded_step: Option<StepIdx>,
    pub(super) missing_slot_values: bool,
    pub(super) event_slot_taint_unsupported: bool,
    envelope_event_seen: bool,
}

impl FrameSeedAccumulator {
    pub(super) fn new(run: RunId, first_seq: crate::EventSeq) -> Self {
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
            envelope_event_seen: false,
        }
    }

    pub(super) fn apply(mut self, event: &JournalEvent) -> RecoveryResult<Self> {
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
                Ok(self.record_step(*step, crate::recovery::RecoveredStepState::Running))
            }
            JournalEvent::StepSucceeded { step, .. } => Ok(self
                .record_step(*step, crate::recovery::RecoveredStepState::Succeeded)
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
                Ok(self.record_step(*step, crate::recovery::RecoveredStepState::Waiting))
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                Ok(self.record_step(*step, crate::recovery::RecoveredStepState::Asking))
            }
            JournalEvent::SlotWrittenEvent {
                slot, value, extra, ..
            } => self.record_slot_write(*slot, value, extra),
            JournalEvent::RunFinished { result, .. } => Ok(self.record_slot(*result)),
            _ => Ok(self),
        }
    }

    fn record_step(mut self, step: StepIdx, state: crate::recovery::RecoveredStepState) -> Self {
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
                let recovered_taint =
                    crate::recovery::replay::summary::slots::recovered_slot_taint(
                        slot, slot_value, extra,
                    )?;
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
        self.envelope_event_seen = true;
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
        self.record_step(
            envelope.ticket.step,
            crate::recovery::RecoveredStepState::Succeeded,
        )
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
        self.envelope_event_seen = true;
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

    pub(super) fn first_step(&self) -> StepIdx {
        self.min_step_idx.map_or(StepIdx::ZERO, |step| step)
    }
}

// ── Dimension helpers ───────────────────────────────────────────────────────

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

// ── Public proof surfaces ───────────────────────────────────────────────────

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

// ── Steps conversion ────────────────────────────────────────────────────────

fn recovered_steps(
    step_states: HashMap<StepIdx, crate::recovery::RecoveredStepState>,
) -> Vec<crate::recovery::RecoveredStepEntry> {
    step_states
        .into_iter()
        .map(|(step, state)| crate::recovery::RecoveredStepEntry { step, state })
        .collect()
}
