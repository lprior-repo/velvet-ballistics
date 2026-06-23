use crate::{
    constants::{
        MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_SNAPSHOT_BYTES,
    },
    error::JournalError,
    events::JournalEvent,
    journal::{EventReplayLimit, FjallJournal},
    keys::{run_event_key, run_prefix_key, run_seq_gap_key, run_snapshot_key},
    types::EventSeq,
};
use fjall::Readable;

/// Maximum initial Vec capacity allocated for replay collection.
///
/// The replay `Vec` starts with this hint and grows naturally via
/// `try_reserve` per push. Bounding the initial hint prevents
/// `Vec::with_capacity(usize::MAX)` from panicking with "capacity overflow"
/// when callers pass `EventReplayLimit::new(usize::MAX)` (e.g. for
/// "read everything" tooling). The actual event count returned is still
/// capped by the `EventReplayLimit` enforced at every push via
/// `classify_replay_push_len`; this constant only bounds the *initial*
/// allocation, not the total events collected.
const MAX_INITIAL_REPLAY_CAPACITY: usize = 4096;

/// Allocation-free replay collection admission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplayPushLimitDecision {
    /// Another event may be collected.
    Accept {
        /// Event count after accepting the next event.
        observed: usize,
    },
    /// Collecting another event would violate the replay limit.
    TooMany {
        /// Configured replay limit.
        limit: usize,
        /// Event count that crossed the limit, or `usize::MAX` on overflow.
        observed: usize,
    },
}

/// Classifies one replay collection push without allocating or touching storage.
pub(crate) fn classify_replay_push_len(
    current_len: usize,
    limit: EventReplayLimit,
) -> ReplayPushLimitDecision {
    let max_events = limit.max_events();
    let Some(observed) = current_len.checked_add(1) else {
        return ReplayPushLimitDecision::TooMany {
            limit: max_events,
            observed: usize::MAX,
        };
    };
    if observed > max_events {
        ReplayPushLimitDecision::TooMany {
            limit: max_events,
            observed,
        }
    } else {
        ReplayPushLimitDecision::Accept { observed }
    }
}

impl FjallJournal {
    /// Replays one run's events in contiguous per-run sequence order.
    pub fn events_for_run(&self, run: vb_core::RunId) -> Result<Vec<JournalEvent>, JournalError> {
        self.events_for_run_bounded(run, EventReplayLimit::DEFAULT)
    }

    /// Replays the full per-run event history, including events that occurred
    /// at or before the most recent durable snapshot.
    ///
    /// SR-002: public recovery APIs whose contract requires the full event
    /// history (admission, summary, frame seed, workflow digest verification)
    /// must use this reader. Default `events_for_run` / `events_for_run_bounded`
    /// skip pre-snapshot events, which silently corrupts summaries, seeds, and
    /// admission records for any run that has been snapshotted.
    pub fn events_for_run_full_bounded(
        &self,
        run: vb_core::RunId,
        limit: EventReplayLimit,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        self.events_for_run_from(run, EventSeq::new(0), EventSeq::new(0), limit)
    }

    /// Replays one run's full event history, starting at `EventSeq::ZERO`.
    ///
    /// Unlike [`Self::events_for_run_bounded`], this method always reads from
    /// the first event in the keyspace, regardless of any durable snapshot
    /// for the run. Use this for full-history recovery paths that must
    /// observe every event (e.g. `recover_full_journal`, which verifies
    /// durable `RunAdmission` evidence at the start of the stream).
    pub fn events_for_run_full(
        &self,
        run: vb_core::RunId,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        self.events_for_run_full_bounded(run, EventReplayLimit::DEFAULT)
    }

    /// Returns the raw bytes for a specific event by (run, seq) key.
    ///
    /// This is a public query API to support external verification of event writes
    /// from outside the `vb_storage` crate (e.g., integration tests).
    pub fn get_event_bytes(
        &self,
        run: vb_core::RunId,
        seq: EventSeq,
    ) -> Result<Option<Vec<u8>>, JournalError> {
        let key = run_event_key(run, seq)?;
        let result: Result<Option<fjall::Slice>, fjall::Error> = self.events.get(key);
        Ok(result?.map(|s| s.to_vec()))
    }

    /// Replays one run's events with an explicit event collection bound.
    pub fn events_for_run_bounded(
        &self,
        run: vb_core::RunId,
        limit: EventReplayLimit,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        let (start_seq, first_event) = match self.latest_durable_snapshot_seq(run)? {
            Some(seq) => {
                // vb-XXX / SR-013: validate the latest snapshot's value before
                // trusting its seq for the tail-replay boundary. A corrupt
                // snapshot (bad magic, payload digest mismatch, or postcard
                // decode failure) must fail closed instead of silently
                // skipping pre-snapshot events and replaying from a stale seq.
                let key = run_snapshot_key(run, seq)?;
                let _: Option<crate::recovery::RunSnapshot> = self.decode_optional(
                    &self.run_snapshot,
                    key.as_slice(),
                    MAGIC_SNAPSHOT,
                    MAX_SNAPSHOT_BYTES,
                )?;
                let tail_start = crate::codec::next_seq(seq)?;
                (tail_start, tail_start)
            }
            None => (EventSeq::new(0), EventSeq::new(0)),
        };
        // vb-qagk2: surface disaster-recovery gap markers (written by
        // `inject_seq_gap`) as a replay-corruption error. A marker at
        // `start_seq` means "the event at this seq was lost" — replay must
        // not silently return an empty or stale event list, otherwise
        // downstream lifecycle derivation would mask the corruption. Without
        // this check, `events_for_run_bounded` would happily return `Ok(vec![])`
        // for a run whose first event was lost, and `derive_lifecycle_state_from_events`
        // would default to `Pending`, hiding the disaster-recovery signal.
        let gap_key = run_seq_gap_key(run, start_seq)?;
        if self.run_seq_gap.contains_key(gap_key)? {
            return Err(JournalError::SequenceGap {
                expected: start_seq,
                actual: start_seq,
            });
        }
        self.events_for_run_from(run, start_seq, first_event, limit)
    }

    /// Returns events for a run starting from a given sequence, with validation.
    pub(crate) fn events_for_run_from(
        &self,
        run: vb_core::RunId,
        start_seq: EventSeq,
        first_event: EventSeq,
        limit: EventReplayLimit,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        // Cap the initial allocation at MAX_INITIAL_REPLAY_CAPACITY so that
        // callers passing `EventReplayLimit::new(usize::MAX)` do not trigger
        // `Vec::with_capacity(usize::MAX)` → "capacity overflow" panic. The
        // Vec grows naturally via `try_reserve` inside `push_replay_event`,
        // and the total count is still bounded by the per-push
        // `classify_replay_push_len` check that converts limit overflows
        // into `JournalError::TooManyEvents`. This satisfies the T8-BS-07
        // contract: an overflow limit must not panic, hang, or OOM.
        let mut replay = Vec::with_capacity(limit.max_events().min(MAX_INITIAL_REPLAY_CAPACITY));
        let mut expected = Some(first_event);
        let start_key = run_event_key(run, start_seq)?;
        let run_prefix = run_prefix_key(run)?;
        let snap = self.database.snapshot();

        // The lower-bound range starts at the first required key, so replay never
        // linearly skips pre-snapshot events. The prefix check terminates at the
        // first lexicographic key for another run in this keyspace.
        for item in snap.range(&self.events, start_key..) {
            let (_, value) = item.into_inner_if(|key| key.as_ref().starts_with(&run_prefix))?;
            let Some(value) = value else {
                break;
            };
            let (_, event) = crate::codec::decode_validated_journal_record(
                value.as_ref(),
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )?
            .into_parts();
            validate_replay_sequence(run, &mut expected, &event)?;
            push_replay_event(&mut replay, run, limit, event)?;
        }

        Ok(replay)
    }
}

fn validate_replay_sequence(
    run: vb_core::RunId,
    expected: &mut Option<EventSeq>,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    let expected_seq = expected.unwrap_or_else(|| event.seq());
    crate::codec::validate_replayed_event(run, expected_seq, event)?;
    *expected = Some(crate::codec::next_seq(expected_seq)?);
    Ok(())
}

fn push_replay_event(
    replay: &mut Vec<JournalEvent>,
    run: vb_core::RunId,
    limit: EventReplayLimit,
    event: JournalEvent,
) -> Result<(), JournalError> {
    let observed = match classify_replay_push_len(replay.len(), limit) {
        ReplayPushLimitDecision::Accept { observed } => observed,
        ReplayPushLimitDecision::TooMany { limit, observed } => {
            return Err(JournalError::TooManyEvents {
                run,
                limit,
                observed,
            });
        }
    };
    replay
        .try_reserve(1)
        .map_err(|_| JournalError::ReplayAllocationFailed {
            run,
            requested: observed,
        })?;
    replay.push(event);
    Ok(())
}
