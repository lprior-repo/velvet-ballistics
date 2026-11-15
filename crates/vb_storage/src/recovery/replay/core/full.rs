use crate::recovery::replay::attempt::compute_max_attempt;
use crate::recovery::types::{ActionReplayTracker, RecoveryError, RecoveryResult};
use crate::{EventSeq, FjallJournal, JournalEvent};
use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest};

use super::{replay_events, replay_events_with_schedule_requirement};

/// Replays a full journal for a run when no snapshot is available.
/// Returns the ordered sequence of journal events and populates the action tracker.
///
/// ## GAP-3: Policy Digest Verification
///
/// Nonempty policy expectations must match consistent pinned admission metadata.
/// Missing, unverified, conflicting, or mismatched policy identities are refused.
/// vb-xk9y9: Action ABI digest verification. When expected_action_abi_digests
/// is non-empty, the journal must contain at least one ActionScheduled event,
/// otherwise the action ABI digests cannot be verified and we return
/// ActionAbiMismatch for the first expected action. This is the symmetric
/// contract to GAP-3 (policy digest verification).
pub fn recover_full_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>> {
    let events = journal.events_for_run_full(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }

    verify_policy_expectations(&events, expected_policy_digests)?;
    verify_action_schedule(&events, expected_action_abi_digests)?;

    replay_events(&events, tracker, expected_action_abi_digests)
}

fn verify_policy_expectations(
    events: &[JournalEvent],
    expected: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<()> {
    let Some((first_step, _)) = expected.first() else {
        return Ok(());
    };
    let pin =
        consistent_policy_pin(events, *first_step)?.ok_or(RecoveryError::PolicyDigestMismatch {
            step: StepIdx::ZERO,
        })?;
    expected.iter().try_for_each(|(step, expected)| {
        if *expected == pin {
            Ok(())
        } else {
            Err(RecoveryError::PolicyDigestMismatch { step: *step })
        }
    })
}

fn consistent_policy_pin(
    events: &[JournalEvent],
    step: StepIdx,
) -> RecoveryResult<Option<WorkflowDigest>> {
    let failure = || RecoveryError::PolicyDigestMismatch { step };
    events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::RunAdmission { policy_pin, .. }
            | JournalEvent::RunAdmissionBudget { policy_pin, .. } => Some(*policy_pin),
            _ => None,
        })
        .try_fold(None, |previous, pin| match (previous, pin) {
            (None, crate::RunPolicyPin::Pinned(digest)) => Ok(Some(digest)),
            (Some(previous), crate::RunPolicyPin::Pinned(digest)) if previous == digest => {
                Ok(Some(previous))
            }
            _ => Err(failure()),
        })
}

fn verify_action_schedule(
    events: &[JournalEvent],
    expected: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<()> {
    let Some((action_id, _)) = expected.first() else {
        return Ok(());
    };
    if events.iter().any(|event| {
        matches!(
            event,
            JournalEvent::ActionScheduled { .. } | JournalEvent::ActionScheduledTicket { .. }
        )
    }) {
        Ok(())
    } else {
        Err(RecoveryError::ActionAbiMismatch {
            action_id: *action_id,
        })
    }
}

/// Loads a snapshot from the journal.
///
/// Translates the journal result into a typed recovery error:
/// - `Ok(Some(snapshot))` → snapshot returned.
/// - `Ok(None)` (no record in keyspace) → `RecoveryError::MissingSnapshot`
///   so callers can pick snapshot-plus-tail recovery or full-journal
///   recovery without conflating "absent" with "unreadable".
/// - `Err(JournalError::PostcardDecodeFailed(_))` (record present but
///   envelope / payload bytes undecodable) → `RecoveryError::CorruptSnapshot`.
/// - any other journal error → `RecoveryError::Journal(other)`.
pub fn load_snapshot(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
) -> RecoveryResult<crate::recovery::types::RunSnapshot> {
    match journal.snapshot(run, seq) {
        Ok(Some(snapshot)) => Ok(snapshot),
        Ok(None) => Err(RecoveryError::MissingSnapshot { run, seq }),
        Err(crate::JournalError::PostcardDecodeFailed(_)) => {
            Err(RecoveryError::CorruptSnapshot { run, seq })
        }
        Err(other) => Err(RecoveryError::Journal(other)),
    }
}

/// Replays from a snapshot plus tail events.
/// The snapshot provides the base state, and tail events are replayed on top.
pub fn recover_snapshot_plus_tail(
    snapshot: &crate::recovery::types::RunSnapshot,
    tail_events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>> {
    let snapshot_seq = snapshot.seq;
    for event in tail_events {
        if event.seq() <= snapshot_seq {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "tail event seq {} is not after snapshot seq {}",
                    event.seq().get(),
                    snapshot_seq.get()
                ),
            });
        }
    }
    replay_events_with_schedule_requirement(
        tail_events,
        tracker,
        false,
        expected_action_abi_digests,
    )
}
/// Checks whether a run has reached a terminal state.
///
/// The set of terminal variants matches every variant of
/// [`crate::recovery::types::RecoveryTerminalState`]; an inconsistency here
/// causes `extract_terminal` to drop `RunKilled` events, `has_terminal_event`
/// to misclassify killed runs as in-progress for retention-policy purposes,
/// and `recover_all_incomplete_runs` to incorrectly include them.
#[must_use]
pub fn is_terminal_event(event: &JournalEvent) -> bool {
    matches!(
        event,
        JournalEvent::RunFinished { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunKilled { .. }
            | JournalEvent::RunFailedEvent { .. }
    )
}

/// Extracts the terminal event from a replay sequence, if any.
///
/// Only considers terminal events from the latest execution attempt.
/// Terminal events from older (stale) attempts are ignored.
pub fn extract_terminal(events: &[JournalEvent]) -> Option<&JournalEvent> {
    let max_attempt = compute_max_attempt(events);
    events
        .iter()
        .rev()
        .find(|event| is_terminal_event(event) && event.attempt().unwrap_or(1) == max_attempt)
}
