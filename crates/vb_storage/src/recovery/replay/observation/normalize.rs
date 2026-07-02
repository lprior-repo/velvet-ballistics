#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Journal event to semantic observation normalization.
//!
//! Maps every `JournalEvent` variant into one or more semantic
//! observations. Skips events that do not carry semantic information
//! (currently none). The returned vector is in event order, never
//! re-sorted, and is deterministic for the same input sequence.
//!
//! [`push_event_observations`] is the top-level dispatcher; each
//! per-variant helper lives in [`super::normalize_push`].

use crate::JournalEvent;

use super::action::observe_action_event;
use super::helpers::observation_digest;
use super::normalize_push::{
    push_ask_answered, push_ask_scheduled, push_ask_timed_out, push_retry_scheduled,
    push_run_accepted, push_run_admission, push_run_answered, push_run_cancelled, push_run_failed,
    push_run_finished, push_run_killed, push_run_resumed, push_run_retried, push_slot_written,
    push_step_started, push_step_succeeded, push_wait_resolved, push_wait_scheduled,
};
use super::types::{
    JournalObservation, JournalObservationSignature, SEMANTIC_OBSERVATION_SCHEMA_VERSION,
};

/// Maximum observations emitted per single input event.
///
/// Today every event maps to at most two observations (e.g. an
/// `AskTimedOutEvent` yields an `Ask::TimedOut` plus a
/// `Timer::AskTimedOut`). The constant documents the upper bound so
/// downstream allocators can pre-size with `try_reserve`.
const MAX_OBSERVATIONS_PER_EVENT: usize = 2;

/// Build a stable semantic signature from an ordered slice of journal events.
///
/// The returned signature is deterministic: two equivalent event
/// sequences produce identical `(schema_version, observations, digest)`.
/// The vector of observations is in event order; the digest is the
/// BLAKE3 finalization over the canonical encoding of all observations.
#[must_use]
pub(crate) fn semantic_observation_signature(
    events: &[JournalEvent],
) -> JournalObservationSignature {
    let observations = observe_journal(events);
    let digest = observation_digest(&observations);
    JournalObservationSignature {
        schema_version: SEMANTIC_OBSERVATION_SCHEMA_VERSION,
        observations,
        digest,
    }
}

/// Project every event into its semantic observation(s).
///
/// Returns a vector in event order. Pre-allocates with `try_reserve` so
/// a bounded journal cannot fail to normalize; allocation failure
/// degrades to incremental growth (the std Vec will retry on each push).
#[must_use]
pub(crate) fn observe_journal(events: &[JournalEvent]) -> Vec<JournalObservation> {
    let capacity_hint = events
        .len()
        .saturating_mul(MAX_OBSERVATIONS_PER_EVENT)
        .saturating_add(8);
    let mut observations: Vec<JournalObservation> = Vec::new();
    // Pre-size via best-effort reservation; allocation failure falls
    // back to incremental growth from each subsequent push.
    let _reserve: Result<(), std::collections::TryReserveError> =
        observations.try_reserve(capacity_hint);
    for event in events {
        push_event_observations(event, &mut observations);
    }
    observations
}

/// Top-level dispatcher: invoke the correct per-variant helper for `event`.
///
/// Dispatches over the 20 `JournalEvent` variants. The match has
/// grown beyond the Farley 25-line ceiling because each `JournalEvent`
/// variant is a distinct structural case; collapsing arms would couple
/// unrelated event families and lose readability. Per-variant business
/// logic lives in [`super::normalize_push`], so this function stays
/// a pure dispatch shell.
fn push_event_observations(event: &JournalEvent, observations: &mut Vec<JournalObservation>) {
    use JournalEvent::*;
    match event {
        RunAccepted { workflow, .. } => push_run_accepted(observations, *workflow),
        RunAdmission {
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => push_run_admission(
            observations,
            *artifact_digest,
            granted_capabilities,
            *policy,
        ),
        RunResumed { .. } => push_run_resumed(observations),
        RunRetried { .. } => push_run_retried(observations),
        StepStarted { step, attempt, .. } => push_step_started(observations, *step, *attempt),
        StepSucceeded { step, output, .. } => push_step_succeeded(observations, *step, *output),
        ActionScheduled { .. }
        | ActionCompletedEvent { .. }
        | ActionScheduledTicket { .. }
        | ActionCompletedEnvelope { .. }
        | ActionFailedEvent { .. }
        | ActionAbandoned { .. } => push_action_event_inner(event, observations),
        SlotWrittenEvent {
            slot,
            value,
            extra,
            attempt,
            ..
        } => push_slot_written(
            observations,
            *slot,
            value.as_deref(),
            extra.as_deref(),
            *attempt,
        ),
        WaitScheduledEvent { step, attempt, .. } => {
            push_wait_scheduled(observations, *step, *attempt)
        }
        WaitResolvedEvent { step, attempt, .. } => {
            push_wait_resolved(observations, *step, *attempt)
        }
        AskScheduledEvent { step, attempt, .. } => {
            push_ask_scheduled(observations, *step, *attempt)
        }
        AskAnsweredEvent { step, attempt, .. } => push_ask_answered(observations, *step, *attempt),
        RunAnswered {
            slot_idx, answer, ..
        } => push_run_answered(observations, *slot_idx, *answer),
        AskTimedOutEvent { step, attempt, .. } => push_ask_timed_out(observations, *step, *attempt),
        RetryScheduledEvent { step, attempt, .. } => {
            push_retry_scheduled(observations, *step, *attempt)
        }
        RunCancelled {
            attempt, reason, ..
        } => push_run_cancelled(observations, *attempt, reason.as_deref()),
        RunKilled { attempt, .. } => push_run_killed(observations, *attempt),
        RunFinished {
            result, attempt, ..
        } => push_run_finished(observations, *result, *attempt),
        RunFailedEvent { attempt, .. } => push_run_failed(observations, *attempt),
    }
}

/// Inline helper for the merged action-event arm in [`push_event_observations`].
/// Kept private and short so the public dispatcher stays compact.
fn push_action_event_inner(event: &JournalEvent, observations: &mut Vec<JournalObservation>) {
    if let Some(observation) = observe_action_event(event) {
        observations.push(JournalObservation::Action(observation));
    }
}
