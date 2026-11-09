// SPDX-License-Identifier: MIT
//
// Proptest: RuntimeJournalEvent::Resumed → JournalEvent::RunResumed mapping
// is invariant across in-range timestamp values, and out-of-range
// timestamps fail with the typed timestamp error instead of a fabricated
// event.

use chrono::{DateTime, Utc};

const MAX_VALID_TIMESTAMP: u64 = 253_402_300_799;

fn assert_resumed_maps_to_run_resumed(timestamp: u64) {
    let run = RunId::new(1);
    let seq = EventSeq::new(7);
    let mapped = StorageRuntimeJournal::storage_event(
        RuntimeJournalEvent::Resumed { run, timestamp },
        seq,
    )
    .expect("in-range resumed maps");
    let seconds = i64::try_from(timestamp).expect("in-range timestamp fits i64");
    let expected = DateTime::<Utc>::from_timestamp(seconds, 0).expect("in-range timestamp");
    assert_eq!(
        mapped,
        JournalEvent::RunResumed {
            run,
            seq,
            timestamp: expected,
        }
    );
}

fn assert_resumed_rejects_out_of_range(timestamp: u64) {
    let run = RunId::new(1);
    let seq = EventSeq::new(7);
    let error = StorageRuntimeJournal::storage_event(
        RuntimeJournalEvent::Resumed { run, timestamp },
        seq,
    )
    .expect_err("out-of-range resumed fails");
    assert_eq!(
        error,
        crate::error::RuntimeError::RuntimeJournalTimestampOutOfRange {
            event_kind: "Resumed",
            timestamp,
        }
    );
}

proptest! {
    #[test]
    fn resumed_replay_is_invariant(timestamp in 0u64..=MAX_VALID_TIMESTAMP) {
        assert_resumed_maps_to_run_resumed(timestamp);
    }

    #[test]
    fn resumed_replay_rejects_out_of_range_timestamps(
        timestamp in (i64::MAX as u64)..=u64::MAX,
    ) {
        assert_resumed_rejects_out_of_range(timestamp);
    }
}

#[test]
fn resumed_replay_rejects_max_timestamp() {
    assert_resumed_rejects_out_of_range(u64::MAX);
}
