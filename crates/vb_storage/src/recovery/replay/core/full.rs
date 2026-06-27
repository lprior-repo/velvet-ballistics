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
/// When `expected_policy_digests` is empty and `RunAdmission` is absent,
/// return `PolicyDigestMismatch` because the policy digest cannot be verified.
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

    let has_run_admission = events
        .iter()
        .any(|e| matches!(e, JournalEvent::RunAdmission { .. }));

    if !has_run_admission && !expected_policy_digests.is_empty() {
        return Err(RecoveryError::PolicyDigestMismatch {
            step: StepIdx::ZERO,
        });
    }

    let missing_required_action_schedule = !expected_action_abi_digests.is_empty()
        && !events
            .iter()
            .any(|e| matches!(e, JournalEvent::ActionScheduled { .. }));
    if missing_required_action_schedule
        && let Some((action_id, _)) = expected_action_abi_digests.first()
    {
        return Err(RecoveryError::ActionAbiMismatch {
            action_id: *action_id,
        });
    }

    replay_events(&events, tracker, expected_action_abi_digests)
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
) -> RecoveryResult<Vec<JournalEvent>> {
    // Verify snapshot consistency
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

    replay_events_with_schedule_requirement(tail_events, tracker, false)
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
