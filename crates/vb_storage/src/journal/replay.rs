use crate::{
    codec::decode_journal_event,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    journal::{EventReplayLimit, FjallJournal},
    keys::{run_event_key, run_prefix_key},
    types::EventSeq,
};
use fjall::Readable;

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
    ///
    /// This is the **snapshot+tail** reader: when a durable snapshot exists
    /// for the run, the lower bound is set to the snapshot sequence + 1 so
    /// that pre-snapshot events are not re-replayed (they are summarized
    /// inside the snapshot). Use [`Self::events_for_run_full`] for the
    /// full-journal counterpart.
    pub fn events_for_run(&self, run: vb_core::RunId) -> Result<Vec<JournalEvent>, JournalError> {
        self.events_for_run_bounded(run, EventReplayLimit::DEFAULT)
    }

    /// Replays one run's events from `EventSeq::new(0)` to the highest
    /// observed seq for the run.
    ///
    /// This is the **full-journal** reader for the recovery boundary (master
    /// §18 persistence invariant 3, "Recovery replays snapshots plus tail
    /// journal or full journal deterministically"). It does NOT consult
    /// `latest_durable_snapshot_seq`; every event from seq 0 is returned
    /// regardless of any durable snapshot, so the caller can verify the
    /// complete run history including the `RunAccepted`, `RunAdmission`,
    /// pre-snapshot step / slot / action lifecycle events that the
    /// snapshot+tail reader (`events_for_run`) skips.
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
    ///
    /// Snapshot+tail semantics: starts after `latest_durable_snapshot_seq`
    /// when present, otherwise from `EventSeq::new(0)`.
    pub fn events_for_run_bounded(
        &self,
        run: vb_core::RunId,
        limit: EventReplayLimit,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        let (start_seq, first_event) = match self.latest_durable_snapshot_seq(run)? {
            Some(seq) => {
                let tail_start = crate::codec::next_seq(seq)?;
                (tail_start, tail_start)
            }
            None => (EventSeq::new(0), EventSeq::new(0)),
        };
        self.events_for_run_from(run, start_seq, first_event, limit)
    }

    /// Replays one run's events from `EventSeq::new(0)` with an explicit
    /// event collection bound. Full-journal semantics: pre-snapshot events
    /// are returned; the durable snapshot is not consulted for sequencing.
    pub fn events_for_run_full_bounded(
        &self,
        run: vb_core::RunId,
        limit: EventReplayLimit,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        let start = EventSeq::new(0);
        self.events_for_run_from(run, start, start, limit)
    }

    /// Returns events for a run starting from a given sequence, with validation.
    pub(crate) fn events_for_run_from(
        &self,
        run: vb_core::RunId,
        start_seq: EventSeq,
        first_event: EventSeq,
        limit: EventReplayLimit,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        let mut replay = Vec::new();
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
            let (_, event) = decode_journal_event(
                value.as_ref(),
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )?;
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
    let expected_seq = match *expected {
        Some(seq) => seq,
        None => event.seq(),
    };
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
