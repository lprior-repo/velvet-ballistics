//! Proptest properties for journal idempotency — append/replay semantics.
//!
//! Bead: vb-xuzwz
//! Obligation: PO-vb-xuzwz-RJ-PROP-001
//!
//! Target: FjallJournal::append_strict_batch + FjallJournal::events_for_run
//!
//! Proves that appending events and then replaying them yields the exact same
//! sequence, establishing idempotent replay as a property of the journal layer.

use proptest::prelude::*;
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

proptest! {
    /// PO-vb-xuzwz-RJ-PROP-001: Append/replay idempotency
    /// Appending a sequence of events and replaying must yield the identical
    /// sequence with matching run, seq, and content.
    #[test]
    fn proptest_journal_append_replay_idempotency(
        run_val in 1u64..10000u64,
        event_count in 1u16..50u16,
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open");
        let run = RunId::new(run_val);

        // Build a sequence of events to append
        let mut events = Vec::new();
        for i in 0..event_count {
            let ev = JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i as u64),
                step: StepIdx::new(i),
                attempt: 1,
            };
            events.push(ev);
        }

        // Append and replay
        journal.append_strict_batch(&events).expect("append batch");
        let replayed = journal.events_for_run(run).expect("replay");

        // Idempotency: replay must match exactly what was appended
        assert_eq!(replayed.len(), event_count as usize,
            "replayed event count must match appended count");
        for (i, ev) in replayed.iter().enumerate() {
            assert_eq!(ev.run_id(), run,
                "replayed event must have same run id");
            assert_eq!(ev.seq(), EventSeq::new(i as u64),
                "replayed event must have correct sequence");
            assert_eq!(ev, &events[i],
                "replayed event must be identical to appended event");
        }
    }

    /// PO-vb-xuzwz-RJ-PROP-002: Persistence across journal reopen
    /// Events appended must survive journal close/reopen — replay after
    /// reopening must be identical to the original append.
    #[test]
    fn proptest_journal_replay_persists_across_reopen(
        run_val in 1u64..1000u64,
        event_count in 1u16..20u16,
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_val);

        // Build events
        let events: Vec<JournalEvent> = (0..event_count)
            .map(|i| JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i as u64),
                step: StepIdx::new(i),
                attempt: 1,
            })
            .collect();

        // First open: append
        {
            let journal = FjallJournal::open(temp.path(), None).expect("journal open");
            journal.append_strict_batch(&events).expect("append batch");
        } // journal dropped

        // Second open: replay
        let journal = FjallJournal::open(temp.path(), None).expect("journal reopen");
        let replayed = journal.events_for_run(run).expect("replay after reopen");

        assert_eq!(replayed.len(), event_count as usize,
            "replay after reopen must yield correct count");
        for (i, ev) in replayed.iter().enumerate() {
            assert_eq!(ev, &events[i],
                "replayed event after reopen must be identical");
        }
    }

    /// PO-vb-xuzwz-RJ-PROP-003: Mixed event kinds replay faithfully
    /// A realistic workflow event sequence (RunAccepted → StepStarted → SlotWritten → RunFinished)
    /// must replay identically to the append order.
    #[test]
    fn proptest_journal_mixed_kinds_replay_idempotency(
        run_val in 1u64..1000u64,
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open");
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([0x42; 32]);

        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: SlotIdx::new(0),
                value: None,
                extra: None,
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(3),
                result: SlotIdx::new(0),
                attempt: 1,
            },
        ];

        journal.append_strict_batch(&events).expect("append mixed batch");
        let replayed = journal.events_for_run(run).expect("replay mixed");

        assert_eq!(replayed.len(), 4,
            "mixed-kinds replay must yield all four events");
        for (i, ev) in replayed.iter().enumerate() {
            assert_eq!(ev, &events[i],
                "mixed-kinds replay event {} must be identical", i);
        }
    }

    /// PO-vb-xuzwz-RJ-PROP-004: Duplicate append is not idempotent (distinct keys)
    /// Appending the same event twice (with different seqs) must result in both
    /// being present — idempotency is a property of replay, not of dedup.
    #[test]
    fn proptest_journal_duplicate_append_yields_distinct_events(
        run_val in 1u64..1000u64,
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open");
        let run = RunId::new(run_val);

        let ev1 = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        };

        let ev2 = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        };

        journal.append_journaled(&ev1).expect("append first");
        journal.append_journaled(&ev2).expect("append duplicate content");

        let replayed = journal.events_for_run(run).expect("replay duplicates");

        assert_eq!(replayed.len(), 2,
            "both duplicate-content events must be replayed");
        assert_eq!(replayed[0].seq(), EventSeq::new(0));
        assert_eq!(replayed[1].seq(), EventSeq::new(1));
    }
}