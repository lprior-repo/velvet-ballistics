#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod snapshot_tests {
    use crate::{DIGEST_BYTES, EventSeq, FjallJournal, RunSnapshot};
    use vb_core::{RunId, WorkflowDigest};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_snapshot(run: RunId, seq: u64, digest: WorkflowDigest) -> RunSnapshot {
        RunSnapshot {
            run,
            seq: EventSeq::new(seq),
            workflow: digest,
            slots: vec![0x01, 0x02, 0x03],
            taint: vec![0x00],
        }
    }

    #[test]
    fn put_snapshot_stores_and_retrieves_compact_snapshot() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1);
        let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);
        let snapshot = make_snapshot(run, 5, digest);

        journal
            .put_snapshot(&snapshot)
            .expect("put_snapshot should succeed");

        let loaded = journal
            .snapshot(run, EventSeq::new(5))
            .expect("snapshot lookup should succeed")
            .expect("snapshot should exist");
        assert_eq!(loaded.run, run);
        assert_eq!(loaded.seq, EventSeq::new(5));
        assert_eq!(loaded.workflow, digest);
        assert_eq!(loaded.slots, vec![0x01, 0x02, 0x03]);
        assert_eq!(loaded.taint, vec![0x00]);
    }

    #[test]
    fn snapshot_returns_none_for_missing_run() {
        let (_temp, journal) = temp_journal();
        let missing_run = RunId::new(999);
        let result = journal.snapshot(missing_run, EventSeq::new(0));
        let found = result.expect("lookup should succeed");
        assert!(found.is_none(), "should return None for missing snapshot");
    }

    #[test]
    fn snapshot_returns_none_for_missing_sequence() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2);
        let digest = WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]);
        let snapshot = make_snapshot(run, 3, digest);
        journal.put_snapshot(&snapshot).expect("put should succeed");

        let result = journal.snapshot(run, EventSeq::new(7));
        let found = result.expect("lookup should succeed");
        assert!(
            found.is_none(),
            "should return None for non-existent sequence"
        );
    }

    #[test]
    fn multiple_snapshots_for_same_run_are_stored_independently() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3);
        let d1 = WorkflowDigest::from_bytes([0x33; DIGEST_BYTES]);
        let d2 = WorkflowDigest::from_bytes([0x44; DIGEST_BYTES]);

        let s1 = RunSnapshot {
            run,
            seq: EventSeq::new(1),
            workflow: d1,
            slots: vec![1],
            taint: vec![],
        };
        let s2 = RunSnapshot {
            run,
            seq: EventSeq::new(3),
            workflow: d2,
            slots: vec![2],
            taint: vec![10],
        };

        journal.put_snapshot(&s1).expect("put s1 should succeed");
        journal.put_snapshot(&s2).expect("put s2 should succeed");

        let loaded1 = journal
            .snapshot(run, EventSeq::new(1))
            .expect("get s1")
            .expect("s1 should exist");
        assert_eq!(loaded1.workflow, d1);
        assert_eq!(loaded1.slots, vec![1]);

        let loaded2 = journal
            .snapshot(run, EventSeq::new(3))
            .expect("get s2")
            .expect("s2 should exist");
        assert_eq!(loaded2.workflow, d2);
        assert_eq!(loaded2.slots, vec![2]);
    }

    #[test]
    fn write_snapshot_convenience_wrapper_works() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4);
        let digest = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(10),
            workflow: digest,
            slots: vec![0xAA],
            taint: vec![],
        };

        crate::write_snapshot(&journal, &snapshot).expect("write_snapshot should succeed");

        let loaded = journal
            .snapshot(run, EventSeq::new(10))
            .expect("get should succeed")
            .expect("snapshot should exist");
        assert_eq!(loaded.slots, vec![0xAA]);
    }

    #[test]
    fn snapshot_with_empty_slots_and_taint_is_stored_and_retrieved() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(5);
        let digest = WorkflowDigest::from_bytes([0x66; DIGEST_BYTES]);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
            slots: vec![],
            taint: vec![],
        };

        journal.put_snapshot(&snapshot).expect("put should succeed");
        let loaded = journal
            .snapshot(run, EventSeq::new(0))
            .expect("get should succeed")
            .expect("should exist");
        assert!(loaded.slots.is_empty());
        assert!(loaded.taint.is_empty());
    }

    // SC-002 / master §18: EventSeq::MAX is a reserved sentinel and must NOT
    // be encodable. `put_snapshot` calls `run_snapshot_key` (which routes to
    // `sequenced_run_key`); that encoder currently accepts MAX. When the
    // encoder is patched to reject MAX, `put_snapshot` will return
    // `Err(JournalError::SequenceOverflow)` and this test will pass.
    #[test]
    fn put_snapshot_rejects_event_seq_max_sentinel() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(6);
        let digest = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);
        let snapshot = make_snapshot(run, u64::MAX, digest);

        let result = journal.put_snapshot(&snapshot);
        assert!(
            matches!(result, Err(crate::JournalError::SequenceOverflow)),
            "put_snapshot must reject EventSeq::MAX sentinel (SC-002), got {:?}",
            result
        );
    }

    #[test]
    fn snapshot_run_mismatch_still_stores_and_retrieves_correctly() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(7);
        let run_b = RunId::new(8);
        let digest = WorkflowDigest::from_bytes([0x88; DIGEST_BYTES]);

        let sa = make_snapshot(run_a, 1, digest);
        let sb = make_snapshot(run_b, 1, digest);

        journal.put_snapshot(&sa).expect("put run_a snapshot");
        journal.put_snapshot(&sb).expect("put run_b snapshot");

        let la = journal
            .snapshot(run_a, EventSeq::new(1))
            .expect("get run_a")
            .expect("run_a should exist");
        let lb = journal
            .snapshot(run_b, EventSeq::new(1))
            .expect("get run_b")
            .expect("run_b should exist");
        assert_eq!(la.run, run_a);
        assert_eq!(lb.run, run_b);
    }

    // Round 10 issue 7: regression test that an overlong key in the
    // snapshot keyspace is rejected by `latest_durable_snapshot_seq`
    // with `TrimError::IncompleteTrim { deleted_count: 0 }`, NOT
    // returned as a successful seq (which would otherwise silently
    // delete the wrong pre-snapshot events).
    #[test]
    fn latest_durable_snapshot_seq_rejects_malformed_overlong_key() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1);
        let digest = WorkflowDigest::from_bytes([0x33; DIGEST_BYTES]);
        let valid = make_snapshot(run, 5, digest);
        journal
            .put_snapshot(&valid)
            .expect("put_snapshot should succeed");

        // 9 prefix+run bytes + 4 extra trailing bytes = 13 bytes total,
        // longer than the 17-byte `RunSnapshot` key shape but with a
        // valid `0x12` (RunSnapshot) prefix.
        let mut overlong: Vec<u8> = vec![0x12, 0, 0, 0, 0, 0, 0, 0, 1];
        overlong.extend_from_slice(b"XYZ0");
        journal
            .run_snapshot
            .insert(overlong, vec![0u8; 8])
            .expect("raw insert should succeed");

        let result = journal.latest_durable_snapshot_seq(run);
        match result {
            Err(crate::trimming::TrimError::IncompleteTrim { deleted_count: 0 }) => {}
            other => panic!(
                "overlong key must yield IncompleteTrim {{ deleted_count: 0 }}, got {:?}",
                other
            ),
        }

        // Direct `snapshot(...)` round-trip still works for the valid seq.
        let loaded = journal
            .snapshot(run, EventSeq::new(5))
            .expect("snapshot lookup should succeed")
            .expect("snapshot should exist");
        assert_eq!(loaded.seq, EventSeq::new(5));
    }

    // Round 10 issue 4: regression test that `put_snapshot` commits with
    // a strict durability barrier, so the snapshot key is visible to a
    // fresh journal opening the same on-disk path (no extra fsync from
    // the test process).
    #[test]
    fn put_snapshot_persists_strictly() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let run = RunId::new(42);
        let digest = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);
        let snapshot = make_snapshot(run, 10, digest);

        {
            let journal =
                FjallJournal::open(temp.path(), None).expect("journal open should succeed");
            journal
                .put_snapshot(&snapshot)
                .expect("put_snapshot should succeed");
        } // journal dropped without explicit close

        // Fresh open on the same on-disk path; if `put_snapshot` had
        // skipped the strict barrier, Fjall's lazy WAL would leave the
        // snapshot unobservable after the drop. A strict `SyncAll`
        // barrier guarantees the snapshot is durable here.
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen should succeed");
        let loaded = reopened
            .snapshot(run, EventSeq::new(10))
            .expect("snapshot lookup should succeed")
            .expect("snapshot must survive reopen");
        assert_eq!(loaded.run, run);
        assert_eq!(loaded.seq, EventSeq::new(10));
        assert_eq!(loaded.workflow, digest);
        assert_eq!(loaded.slots, vec![0x01, 0x02, 0x03]);
    }
}
