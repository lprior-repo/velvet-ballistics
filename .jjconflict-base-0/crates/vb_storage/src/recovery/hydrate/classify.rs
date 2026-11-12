#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use crate::JournalEvent;
use crate::recovery::types::{
    RecoveryCannotResumeState, RecoveryError, RecoveryResult, RunSnapshot,
};
use vb_core::{RunId, StepIdx};

/// Rejects a snapshot+tail pair that fails the typed cannot-resume gate.
///
/// Returns `Ok(())` when the pair has enough evidence to rebuild a live
/// frame; otherwise returns `RecoveryError::UnsupportedFrameSeed`
/// carrying the precise priority-ordered reason string.
pub(crate) fn reject_if_cannot_resume(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<()> {
    let cannot_resume = classify_snapshot_tail_cannot_resume(snapshot, tail_events);
    if cannot_resume.is_resumable() {
        return Ok(());
    }
    Err(RecoveryError::UnsupportedFrameSeed {
        run: run_id,
        reason: String::from(cannot_resume.unsupported_reason()),
    })
}

/// Walks the snapshot+tail pair and classifies the cannot-resume
/// surface that the typed boundary would otherwise flag. Returns a
/// [`RecoveryCannotResumeState`] whose `is_resumable()` reflects
/// whether the pair carries enough evidence to rebuild a live frame.
///
/// Flags set by this helper:
/// - `slot_values` — any `SlotWrittenEvent { value: None, .. }` in
///   the tail (defense-in-depth alongside `apply_slot_written`).
/// - `pending_actions` — any `ActionScheduled` /
///   `ActionScheduledTicket` whose step still appears in the tail
///   without a matching `ActionCompleted*` / `ActionFailedEvent` /
///   `ActionAbandoned`.
/// - `pending_timers` — any `WaitScheduledEvent` whose step still
///   appears without a matching `WaitResolvedEvent`.
/// - `pending_asks` — any `AskScheduledEvent` whose step still
///   appears without a matching `AskAnsweredEvent` /
///   `AskTimedOutEvent`.
///
/// The frame-seed-only `*_missing` flags are NOT set by this helper:
/// they describe the absence of full RunState evidence, which the
/// snapshot+tail path rebuilds from durable snapshot bytes.
///
/// The `_snapshot` parameter is reserved for future flag-set
/// expansion: today the typed `*_missing` flags are stamped
/// unconditionally by [`RecoveryCannotResumeState::from_seed`], but
/// this helper will eventually need to inspect the snapshot's
/// `slots`/`taint`/`workflow` bytes to distinguish
/// `snapshot_supports_resume` from
/// `snapshot_bytes_diverge_from_tail`. Keeping the parameter in the
/// public signature avoids a future API break. Underscore prefix is
/// the standard Rust convention for intentional non-use.
#[must_use]
pub fn classify_snapshot_tail_cannot_resume(
    _snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
) -> RecoveryCannotResumeState {
    let mut state = RecoveryCannotResumeState::RESUMABLE;
    let mut pending_waits: BTreeSet<StepIdx> = BTreeSet::new();
    let mut pending_asks: BTreeSet<StepIdx> = BTreeSet::new();
    let mut pending_actions: BTreeSet<StepIdx> = BTreeSet::new();
    for event in tail_events {
        mark_slot_value_none(&mut state, event);
        drain_classify_event_to_pending_sets(
            &mut pending_waits,
            &mut pending_asks,
            &mut pending_actions,
            event,
        );
    }
    set_cannot_resume_from_pending_sets(
        &mut state,
        &pending_waits,
        &pending_asks,
        &pending_actions,
    );
    state
}

/// Sets `state.slot_values = true` when the event is a
/// `SlotWrittenEvent { value: None, .. }`; otherwise no-op.
fn mark_slot_value_none(state: &mut RecoveryCannotResumeState, event: &JournalEvent) {
    if let JournalEvent::SlotWrittenEvent { value: None, .. } = event {
        state.slot_values = true;
    }
}

/// Distributes a single event into the per-domain pending sets.
fn drain_classify_event_to_pending_sets(
    pending_waits: &mut BTreeSet<StepIdx>,
    pending_asks: &mut BTreeSet<StepIdx>,
    pending_actions: &mut BTreeSet<StepIdx>,
    event: &JournalEvent,
) {
    match event {
        JournalEvent::RetryTimerScheduled { step, .. }
        | JournalEvent::WaitScheduledEvent { step, .. } => {
            pending_waits.insert(*step);
        }
        JournalEvent::WaitResolvedEvent { step, .. } => {
            pending_waits.remove(step);
        }
        JournalEvent::AskScheduledEvent { step, .. } => {
            pending_asks.insert(*step);
        }
        JournalEvent::AskAnsweredEvent { step, .. } => {
            pending_asks.remove(step);
        }
        JournalEvent::AskTimedOutEvent { step, .. } => {
            pending_asks.remove(step);
        }
        JournalEvent::ActionScheduled { step, .. } => {
            pending_actions.insert(*step);
        }
        JournalEvent::ActionCompletedEvent { step, .. } => {
            pending_actions.remove(step);
        }
        JournalEvent::ActionFailedEvent { step, .. } => {
            pending_actions.remove(step);
        }
        JournalEvent::ActionScheduledTicket { ticket, .. } => {
            pending_actions.insert(ticket.step);
        }
        JournalEvent::ActionCompletedEnvelope { ticket, .. } => {
            pending_actions.remove(&ticket.step);
        }
        JournalEvent::ActionAbandoned { ticket, .. } => {
            pending_actions.remove(&ticket.step);
        }
        _ => {}
    }
}

/// Transfers the "non-empty pending set" markers from the per-domain
/// sets into the typed cannot-resume witness. Only the boolean
/// emptiness flags are set; the `slot_values` flag is set by
/// [`mark_slot_value_none`] because it does not require accumulation.
fn set_cannot_resume_from_pending_sets(
    state: &mut RecoveryCannotResumeState,
    waits: &BTreeSet<StepIdx>,
    asks: &BTreeSet<StepIdx>,
    actions: &BTreeSet<StepIdx>,
) {
    state.pending_timers = !waits.is_empty();
    state.pending_asks = !asks.is_empty();
    state.pending_actions = !actions.is_empty();
}
