#![forbid(unsafe_code)]
//! Recovery operations: full journal replay and snapshot-based recovery.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::records::RecoveryStampRecord;
use crate::{EventSeq, FjallJournal, JournalError, JournalEvent};
use vb_core::RunId;
use vb_core::{ActionId, StepIdx, WorkflowDigest};

use super::core::replay_events;

/// Replays a full journal for a run. Reads the entire event history from
/// sequence zero, including pre-snapshot events, regardless of whether a
/// durable snapshot exists for the run.
///
/// Returns the ordered sequence of journal events and populates the action tracker.
///
/// ## GAP-3: Policy Digest Verification
///
/// Missing durable `RunAdmission` evidence fails closed instead of replaying
/// without proof or fabricating a digest value.
///
/// ## SR-001: Full-history replay
///
/// `events_for_run` is snapshot-tail optimized (it skips events at or before
/// the latest durable snapshot seq). `recover_full_journal` must verify
/// `RunAdmission` evidence that lives at the start of the stream, so it
/// reads via `events_for_run_full` which starts at `EventSeq::ZERO`.
///
/// ## Recovery stamp integration (vb-k6iwh-r)
///
/// Before replay, the recovery stamp keyspace is consulted at `(run, last_seq)`.
/// A present stamp indicates a prior recovery attempt has already replayed
/// the journal up to `last_seq`; replay is still run (to refresh the action
/// tracker for the caller) but no new stamp is written, preserving the
/// existing marker. The skip-replay semantic — using the stamp to short-circuit
/// replay when the journal tail is unchanged — is delegated to a follow-up
/// bead that will compare the stamp's `last_seq` against the journal tail.
///
/// After a successful replay with no prior stamp, a fresh `RecoveryStampRecord`
/// is persisted at `(run, last_seq)` so a subsequent recovery invocation can
/// detect that the replay for this run has already been performed.
pub fn recover_full_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut crate::recovery::ActionReplayTracker,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> crate::recovery::RecoveryResult<Vec<JournalEvent>> {
    let events = journal.events_for_run_full(run)?;
    if events.is_empty() {
        return Err(crate::recovery::RecoveryError::NoRecoveryData { run });
    }

    super::admission::verify_run_admission_evidence(&events, run, expected_policy_digests)?;

    let last_seq = events.last().map_or(EventSeq::ZERO, JournalEvent::seq);
    let replayed = replay_events(&events, tracker, expected_action_abi_digests)?;

    if journal.get_recovery_stamp(run, last_seq)?.is_none() {
        write_recovery_stamp(journal, run, last_seq)?;
    }
    Ok(replayed)
}

/// Persists a `RecoveryStampRecord` for `(run, last_seq)` after successful replay.
///
/// The millisecond timestamp is derived from the wall clock and narrowed to
/// `u64` with checked arithmetic; a clock that exceeds `u64::MAX` ms (year
/// ~584 billion) saturates to `u64::MAX` so the stamp is always writable.
fn write_recovery_stamp(
    journal: &FjallJournal,
    run: RunId,
    last_seq: EventSeq,
) -> crate::recovery::RecoveryResult<()> {
    let written_at_ms = current_unix_millis_saturating();
    let stamp = RecoveryStampRecord {
        run,
        last_seq,
        written_at_ms,
    };
    journal.put_recovery_stamp(run, last_seq, stamp)?;
    Ok(())
}

/// Returns the current wall-clock time in milliseconds since the Unix epoch,
/// saturating to `u64::MAX` on overflow. The conversion is checked, not lossy.
fn current_unix_millis_saturating() -> u64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = duration.as_millis();
    if millis > u128::from(u64::MAX) {
        u64::MAX
    } else {
        u64::try_from(millis).unwrap_or(u64::MAX)
    }
}

/// Loads a snapshot from the journal, distinguishing missing snapshots from
/// corrupted ones.
///
/// A successful read returns the snapshot. If the journal has no row at the
/// requested `(run, seq)` pair, returns `MissingSnapshot` so callers can
/// recover or skip without treating the absence as corruption. Decode failures
/// (`PostcardDecodeFailed`) and other integrity evidence gaps remain mapped to
/// `CorruptSnapshot` so callers fail closed.
pub fn load_snapshot(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
) -> crate::recovery::RecoveryResult<crate::recovery::RunSnapshot> {
    match journal.snapshot(run, seq) {
        Ok(Some(snapshot)) => Ok(snapshot),
        Ok(None) => Err(crate::recovery::RecoveryError::MissingSnapshot { run, seq }),
        Err(JournalError::PostcardDecodeFailed) => {
            Err(crate::recovery::RecoveryError::CorruptSnapshot { run, seq })
        }
        Err(other) => Err(crate::recovery::RecoveryError::Journal(other)),
    }
}

/// Replays from a snapshot plus tail events.
/// The snapshot provides the base state, and tail events are replayed on top.
pub fn recover_snapshot_plus_tail(
    snapshot: &crate::recovery::RunSnapshot,
    tail_events: &[JournalEvent],
    tracker: &mut crate::recovery::ActionReplayTracker,
) -> crate::recovery::RecoveryResult<Vec<JournalEvent>> {
    // Verify snapshot consistency: per-event strict-greater, plus SR-006 cross-snapshot
    // ordering check (first tail event seq must be strictly after snapshot.seq;
    // gaps are permitted because events between snapshot and tail are already
    // covered by the snapshot itself).
    let snapshot_seq = snapshot.seq;
    if let Some(first) = tail_events.first()
        && first.seq() <= snapshot_seq
    {
        return Err(crate::recovery::RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: format!(
                "tail event seq {} is not after snapshot seq {}",
                first.seq().get(),
                snapshot_seq.get()
            ),
        });
    }
    for event in tail_events {
        if event.seq() <= snapshot_seq {
            return Err(crate::recovery::RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "tail event seq {} is not after snapshot seq {}",
                    event.seq().get(),
                    snapshot_seq.get()
                ),
            });
        }
    }

    super::core::replay_events_with_schedule_requirement(tail_events, tracker, false)
}
