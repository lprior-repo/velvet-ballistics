#![forbid(unsafe_code)]
//! Frame seed accumulator — struct, state machine, and envelope view.
//!
//! Provides:
//! - `FrameSeedAccumulator` — event → frame seed state machine
//! - `ActionEnvelopeView` — extracted view of `ActionCompletedEnvelope` data

use std::collections::{HashMap, HashSet};

use crate::recovery::action_digest::verify_action_ticket_event;
use crate::recovery::replay::summary::runtime_summary::apply_summary_event;
use crate::recovery::{ActionReplayTracker, RecoveryError, RecoveryResult, RecoveryRuntimeSummary};
use crate::{EventSeq, JournalEvent};
use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx, Taint};

// ── Dimension helpers (needed by accumulator impl) ──────────────────────────

fn max_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    current.map_or(Some(candidate), |step| Some(step.max(candidate)))
}

fn min_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    current.map_or(Some(candidate), |step| Some(step.min(candidate)))
}

fn max_slot(current: Option<SlotIdx>, candidate: SlotIdx) -> Option<SlotIdx> {
    current.map_or(Some(candidate), |slot| Some(slot.max(candidate)))
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
pub(crate) struct FrameSeedAccumulator {
    pub(super) run: RunId,
    pub(super) summary: RecoveryRuntimeSummary,
    pub(super) step_states: HashMap<StepIdx, crate::recovery::RecoveredStepState>,
    pub(crate) slot_values: HashMap<SlotIdx, SlotValue>,
    pub(crate) slot_taint: HashMap<SlotIdx, Taint>,
    pub(super) pending_actions: HashSet<(ActionId, StepIdx)>,
    pub(super) action_tracker: ActionReplayTracker,
    pub(crate) max_step_idx: Option<StepIdx>,
    min_step_idx: Option<StepIdx>,
    pub(crate) max_slot_idx: Option<SlotIdx>,
    pub(super) pc: StepIdx,
    pub(crate) last_succeeded_step: Option<StepIdx>,
    pub(super) missing_slot_values: bool,
    pub(super) event_slot_taint_unsupported: bool,
    pub(super) envelope_event_seen: bool,
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

    pub(super) fn record_step(
        mut self,
        step: StepIdx,
        state: crate::recovery::RecoveredStepState,
    ) -> Self {
        self.max_step_idx = max_step(self.max_step_idx, step);
        self.min_step_idx = min_step(self.min_step_idx, step);
        self.pc = max_step(Some(self.pc), step).map_or(self.pc, |value| value);
        self.step_states.insert(step, state);
        self
    }

    pub(super) fn record_last_succeeded(mut self, step: StepIdx) -> Self {
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

    pub(super) fn first_step(&self) -> StepIdx {
        self.min_step_idx.map_or(StepIdx::ZERO, |step| step)
    }
}
