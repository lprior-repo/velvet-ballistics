use crate::{
    codec::decode_record,
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    journal::{EventReplayLimit, FjallJournal},
    keys::run_prefix_key,
    types::EventSeq,
};
use fjall::Readable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirstReplayEvent {
    AnyAtOrAfterStart,
    Exact(EventSeq),
}

impl FjallJournal {
    /// Replays one run's events in contiguous per-run sequence order.
    pub fn events_for_run(&self, run: vb_core::RunId) -> Result<Vec<JournalEvent>, JournalError> {
        self.events_for_run_bounded(run, EventReplayLimit::DEFAULT)
    }

    /// Replays one run's events with an explicit event collection bound.
    pub fn events_for_run_bounded(
        &self,
        run: vb_core::RunId,
        limit: EventReplayLimit,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        let (start_seq, first_event) = match self.latest_durable_snapshot_seq(run)? {
            Some(seq) => (seq, FirstReplayEvent::Exact(seq)),
            None => (EventSeq::new(0), FirstReplayEvent::AnyAtOrAfterStart),
        };
        self.events_for_run_from(run, start_seq, first_event, limit)
    }

    /// Returns events for a run starting from a given sequence, with validation.
    pub(crate) fn events_for_run_from(
        &self,
        run: vb_core::RunId,
        start_seq: EventSeq,
        first_event: FirstReplayEvent,
        limit: EventReplayLimit,
    ) -> Result<Vec<JournalEvent>, JournalError> {
        let mut replay = Vec::new();
        let mut expected = match first_event {
            FirstReplayEvent::AnyAtOrAfterStart => None,
            FirstReplayEvent::Exact(seq) => Some(seq),
        };
        let snap = self.database.snapshot();

        for item in snap.prefix(&self.events, run_prefix_key(run)?) {
            let value = item.value()?;
            let (_, event): (_, JournalEvent) = decode_record(
                value.as_ref(),
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )?;
            // Skip events before the start sequence (already captured in snapshot)
            if event.seq().get() < start_seq.get() {
                continue;
            }
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
    let observed = replay
        .len()
        .checked_add(1)
        .ok_or(JournalError::TooManyEvents {
            run,
            limit: limit.max_events(),
            observed: usize::MAX,
        })?;
    if observed > limit.max_events() {
        return Err(JournalError::TooManyEvents {
            run,
            limit: limit.max_events(),
            observed,
        });
    }
    replay
        .try_reserve(1)
        .map_err(|_| JournalError::ReplayAllocationFailed {
            run,
            requested: observed,
        })?;
    replay.push(event);
    Ok(())
}
