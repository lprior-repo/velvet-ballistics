#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, StepIdx};
use vb_storage::recovery::replay::core::{replay_attempt_is_current, replay_attempt_is_stale};
use vb_storage::{EventReplayLimit, EventSeq, FjallJournal, JournalError, JournalEvent};

const MAX_REPLAY_EVENTS: usize = 8;
const MIN_REPLAY_EVENTS: usize = 2;

fuzz_target!(|data: &[u8]| {
    let attempt = data.first().copied().map(u16::from);
    let max_attempt = data.get(1).copied().map_or(1, u16::from);
    let observed = attempt.unwrap_or(1);
    assert_eq!(
        replay_attempt_is_current(attempt, max_attempt),
        observed >= max_attempt
    );
    assert_eq!(
        replay_attempt_is_stale(attempt, max_attempt),
        observed < max_attempt
    );

    let run = RunId::new(u64::from(data.get(2).copied().unwrap_or(0)).saturating_add(1));
    let Some(count) = replay_event_count(data.get(3).copied().unwrap_or(0)) else {
        return;
    };
    let Some(events) = build_replay_events(run, count) else {
        return;
    };
    observe_close_reopen_replay(run, &events);
});

fn replay_event_count(seed: u8) -> Option<usize> {
    let span = MAX_REPLAY_EVENTS.checked_sub(1)?;
    usize::from(seed)
        .checked_rem(span)?
        .checked_add(MIN_REPLAY_EVENTS)
}

fn build_replay_events(run: RunId, count: usize) -> Option<Vec<JournalEvent>> {
    let mut events = Vec::with_capacity(count);
    for event_index in 0..count {
        let seq = u64::try_from(event_index);
        assert!(
            seq.is_ok(),
            "bounded replay event index must fit u64: {:?}",
            seq.as_ref().err()
        );
        let Ok(seq) = seq else {
            return None;
        };
        let step = u16::try_from(event_index);
        assert!(
            step.is_ok(),
            "bounded replay event index must fit u16: {:?}",
            step.as_ref().err()
        );
        let Ok(step) = step else {
            return None;
        };
        events.push(JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
            attempt: 1,
        });
    }
    Some(events)
}

fn observe_close_reopen_replay(run: RunId, events: &[JournalEvent]) {
    let dir = tempfile::tempdir();
    assert!(
        dir.is_ok(),
        "temporary journal directory setup failed: {:?}",
        dir.as_ref().err()
    );
    let Ok(dir) = dir else {
        return;
    };
    {
        let journal = FjallJournal::open(dir.path(), None);
        assert!(
            journal.is_ok(),
            "initial journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        assert!(
            journal.append_strict_batch(events).is_ok(),
            "strict batch append must persist generated contiguous events"
        );
        assert!(
            journal.close().is_ok(),
            "journal close must report strict durability"
        );
    }
    {
        let journal = FjallJournal::open(dir.path(), None);
        assert!(
            journal.is_ok(),
            "reopened journal must open: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        let replayed = journal.events_for_run_full(run);
        assert!(
            matches_replayed_events(&replayed, events),
            "close/reopen full replay must return the exact generated event stream"
        );
        assert_replay_limit(run, &journal, events);
        assert!(
            journal.close().is_ok(),
            "reopened journal close must succeed"
        );
    }
    assert_persisted_corrupt_event_fails_closed(dir.path(), next_run(run));
}

fn assert_persisted_corrupt_event_fails_closed(path: &std::path::Path, run: RunId) {
    {
        let journal = FjallJournal::open(path, None);
        assert!(
            journal.is_ok(),
            "corrupt-event setup journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        assert!(
            journal.inject_seq_gap(run, EventSeq::new(0)).is_ok(),
            "raw non-event payload injection must persist for corruption oracle"
        );
        assert!(
            journal.close().is_ok(),
            "corrupt-event setup journal close must succeed"
        );
    }
    {
        let journal = FjallJournal::open(path, None);
        assert!(
            journal.is_ok(),
            "corrupt-event replay journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        let result = journal.events_for_run_full(run);
        assert!(
            matches!(result, Err(JournalError::PostcardDecodeFailed(_))),
            "persisted non-event payload must fail closed as PostcardDecodeFailed"
        );
        assert!(
            journal.close().is_ok(),
            "corrupt-event replay journal close must succeed"
        );
    }
}

fn next_run(run: RunId) -> RunId {
    match run.get().checked_add(1) {
        Some(value) => RunId::new(value),
        None => RunId::new(1),
    }
}

fn matches_replayed_events(
    result: &Result<Vec<JournalEvent>, JournalError>,
    expected: &[JournalEvent],
) -> bool {
    match result {
        Ok(replayed) => replayed.as_slice() == expected,
        Err(_error) => false,
    }
}

fn assert_replay_limit(run: RunId, journal: &FjallJournal, events: &[JournalEvent]) {
    let limit_value = events.len().saturating_sub(1);
    let limit = EventReplayLimit::new(limit_value);
    assert!(limit.is_some(), "generated replay limit must be non-zero");
    let Some(limit) = limit else {
        return;
    };
    let result = journal.events_for_run_full_bounded(run, limit);
    assert!(
        matches!(
            result,
            Err(JournalError::TooManyEvents {
                run: error_run,
                limit: error_limit,
                observed,
            }) if error_run == run && error_limit == limit_value && observed > error_limit
        ),
        "bounded replay below event count must return TooManyEvents"
    );
}
