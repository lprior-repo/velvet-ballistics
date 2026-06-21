#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]

#[cfg(test)]
mod edge_case_tests {
    use crate::{
        BlobRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalError, JournalEvent,
        JournalWriterQueue, MAGIC_BLOB, MAGIC_INDEX_RECORD, MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES, RecordKind, RunHeaderRecord,
        RunSnapshot, StorageLimits, WorkflowSourceRecord, decode_record, encode_record,
    };
    use std::sync::Arc;
    use std::thread;
    use vb_core::{RunId, StepIdx, WorkflowDigest, WorkflowId};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open");
        (temp, journal)
    }

    // =========================================================================
    // Disk full simulation
    // =========================================================================

    #[test]
    fn persist_strict_handles_simulated_failure() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        journal
            .append_journaled(&event)
            .expect("append should succeed");

        journal.fail_next_persist_for_test();
        let result = journal.persist_strict();
        assert!(
            matches!(result, Err(JournalError::StrictDurabilityFailed)),
            "simulated disk failure must yield StrictDurabilityFailed"
        );

        let replayed = journal
            .events_for_run(run)
            .expect("replay after failure should succeed");
        assert_eq!(replayed.len(), 1, "event should still be in WAL");
    }

    #[test]
    fn persist_strict_recovers_after_simulated_failure() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x11; 32]),
        };

        journal.fail_next_persist_for_test();
        let result = journal.append_strict(&event);
        assert!(
            matches!(result, Err(JournalError::StrictDurabilityFailed)),
            "first persist should simulate failure"
        );

        journal.append_strict(&event).expect("retry should succeed");
        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), 1);
    }

    // =========================================================================
    // Concurrent access patterns
    // =========================================================================

    #[test]
    fn multiple_threads_append_to_different_runs() {
        let (_temp, journal) = temp_journal();
        let journal = Arc::new(journal);
        let mut handles = Vec::new();

        for thread_idx in 0..8 {
            let j = Arc::clone(&journal);
            let handle = thread::spawn(move || {
                let run = RunId::new(u64::from(thread_idx as u64) + 1);
                for seq in 0..5u64 {
                    let event = JournalEvent::StepStarted {
                        run,
                        seq: EventSeq::new(seq),
                        step: StepIdx::new(seq as u16),
                        attempt: 1,
                    };
                    j.append_journaled(&event)
                        .expect("append from thread should succeed");
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("thread should not panic");
        }

        for thread_idx in 0..8 {
            let run = RunId::new((thread_idx as u64) + 1);
            let events = journal.events_for_run(run).expect("replay should succeed");
            assert_eq!(
                events.len(),
                5,
                "thread {} should have 5 events",
                thread_idx
            );
            for (i, ev) in events.iter().enumerate() {
                assert_eq!(ev.seq(), EventSeq::new(i as u64));
                assert_eq!(ev.run_id(), run);
            }
        }
    }

    #[test]
    fn concurrent_enqueue_to_writer_queue() {
        let (_temp, journal) = temp_journal();
        let queue = Arc::new(
            JournalWriterQueue::new(32, 16, StorageLimits::DEFAULT).expect("queue creation"),
        );
        let journal = Arc::new(journal);
        let mut handles = Vec::new();

        for thread_idx in 0..4 {
            let q = Arc::clone(&queue);
            let j = Arc::clone(&journal);
            let handle = thread::spawn(move || {
                let run = RunId::new((thread_idx as u64) * 100 + 1);
                for seq in 0..5u64 {
                    let event = JournalEvent::StepStarted {
                        run,
                        seq: EventSeq::new(seq),
                        step: StepIdx::new(seq as u16),
                        attempt: 1,
                    };
                    q.enqueue_journaled(event)
                        .expect("enqueue from thread should succeed");
                }
                q.drain_all(&j).expect("drain_all should succeed");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("thread should not panic");
        }

        for thread_idx in 0..4 {
            let run = RunId::new((thread_idx as u64) * 100 + 1);
            let events = journal.events_for_run(run).expect("replay should succeed");
            assert_eq!(events.len(), 5);
        }
    }

    #[test]
    fn concurrent_batch_writes_from_multiple_threads() {
        let (_temp, journal) = temp_journal();
        let journal = Arc::new(journal);
        let mut handles = Vec::new();

        for thread_idx in 0..4 {
            let j = Arc::clone(&journal);
            let handle = thread::spawn(move || {
                let run = RunId::new((thread_idx as u64) + 1000);
                let mut batch = j.batch();
                for seq in 0..3u64 {
                    let event = JournalEvent::StepStarted {
                        run,
                        seq: EventSeq::new(seq),
                        step: StepIdx::new(seq as u16),
                        attempt: 1,
                    };
                    batch
                        .append_event(&event)
                        .expect("batch append should succeed");
                }
                batch.commit().expect("batch commit should succeed");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("thread should not panic");
        }

        for thread_idx in 0..4 {
            let run = RunId::new((thread_idx as u64) + 1000);
            let events = journal.events_for_run(run).expect("replay should succeed");
            assert_eq!(events.len(), 3);
        }
    }

    #[test]
    fn concurrent_read_while_another_writes() {
        let (_temp, journal) = temp_journal();
        let journal = Arc::new(journal);
        let run = RunId::new(5000);

        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([0u8; 32]),
            })
            .expect("initial append");

        let j_reader = Arc::clone(&journal);
        let read_handle = thread::spawn(move || {
            for _ in 0..10 {
                let events = j_reader.events_for_run(run);
                if let Ok(ev) = events {
                    assert!(!ev.is_empty(), "reader should find events");
                }
            }
        });

        let j_writer = Arc::clone(&journal);
        let write_handle = thread::spawn(move || {
            for seq in 1..6u64 {
                let event = JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(seq),
                    step: StepIdx::new(seq as u16),
                    attempt: 1,
                };
                j_writer
                    .append_journaled(&event)
                    .expect("write should succeed");
            }
        });

        read_handle.join().expect("reader should not panic");
        write_handle.join().expect("writer should not panic");

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert!(events.len() >= 1, "should have at least initial event");
    }

    // =========================================================================
    // Very large values
    // =========================================================================

    #[test]
    fn very_large_blob_payload() {
        let (_temp, journal) = temp_journal();
        let large = vec![0x42u8; 1024 * 1024];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&large).into();
        let record = BlobRecord {
            digest,
            bytes: large.clone(),
        };
        journal.put_blob(&record).expect("large blob put");
        let loaded = journal
            .blob(digest)
            .expect("get should succeed")
            .expect("present");
        assert_eq!(loaded.bytes, large);
    }

    #[test]
    fn very_large_compiled_ir_payload() {
        let (_temp, journal) = temp_journal();
        let large_ir = vec![0xAAu8; 512 * 1024];
        let record = crate::try_accepted_compiled_ir_record_for_test(large_ir).expect("test fixture should encode");
        let digest = record.digest;
        journal.put_compiled_ir(&record).expect("large ir put");
        let loaded = journal
            .compiled_ir(digest)
            .expect("get should succeed")
            .expect("present");
        assert_eq!(loaded, record);
    }

    #[test]
    fn very_large_workflow_source_payload() {
        let (_temp, journal) = temp_journal();
        let large_source = vec![b'x'; 128 * 1024];
        let digest = WorkflowDigest::from_bytes(blake3::hash(&large_source).into());
        let record = WorkflowSourceRecord {
            digest,
            source: large_source.clone(),
        };
        journal
            .put_workflow_source(&record)
            .expect("large source put");
        let loaded = journal
            .workflow_source(digest)
            .expect("get should succeed")
            .expect("present");
        assert_eq!(loaded.source, large_source);
    }

    #[test]
    fn very_large_snapshot_with_many_slots() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1001);
        let slots = vec![1u8; 10_000];
        let taint = vec![0u8; 10_000];
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xCC; 32]),
            slots: slots.clone(),
            taint: taint.clone(),
        };
        journal.put_snapshot(&snapshot).expect("large snapshot put");
        let loaded = journal
            .snapshot(run, EventSeq::new(0))
            .expect("get should succeed")
            .expect("present");
        assert_eq!(loaded.slots, slots);
        assert_eq!(loaded.taint, taint);
    }

    #[test]
    fn very_large_run_header_values() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(u64::MAX);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(u32::MAX),
            compiled_digest: WorkflowDigest::from_bytes([0xFF; 32]),
            status: 255,
            accepted_at_ms: u64::MAX,
        };
        journal
            .put_run_header(&header)
            .expect("max-value header put");
        let loaded = journal
            .run_header(run)
            .expect("get should succeed")
            .expect("present");
        assert_eq!(loaded.run, run);
        assert_eq!(loaded.status, 255);
        assert_eq!(loaded.accepted_at_ms, u64::MAX);
    }

    #[test]
    fn many_events_per_run() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(9999);

        let count = 200u64;
        for i in 0..count {
            let ev = JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i),
                step: StepIdx::new((i % 256) as u16),
                attempt: 1,
            };
            journal
                .append_journaled(&ev)
                .expect("append should succeed");
        }

        let events = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(events.len(), count as usize);
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.seq(), EventSeq::new(i as u64));
        }
    }

    // =========================================================================
    // Rapid open/close cycles
    // =========================================================================

    #[test]
    fn rapid_open_close_cycles_preserve_data() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().to_path_buf();

        for cycle in 0..10 {
            let mut journal = FjallJournal::open(&path, None).expect("open");
            let run = RunId::new((cycle as u64) + 1);
            let event = JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(cycle),
                attempt: 1,
            };
            journal.append_journaled(&event).expect("append");
            journal.close().expect("close should succeed");
        }

        let journal = FjallJournal::open(&path, None).expect("reopen");
        for cycle in 0..10 {
            let run = RunId::new((cycle as u64) + 1);
            let events = journal.events_for_run(run).expect("replay should succeed");
            assert_eq!(events.len(), 1, "cycle {} should have 1 event", cycle);
            assert_eq!(events[0].seq(), EventSeq::new(0));
        }
    }

    #[test]
    fn rapid_open_close_without_writes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().to_path_buf();

        for _ in 0..20 {
            let mut journal = FjallJournal::open(&path, None).expect("open");
            journal.close().expect("close should succeed");
        }

        let journal = FjallJournal::open(&path, None).expect("reopen");
        let events = journal.events_for_run(RunId::new(1)).expect("replay");
        assert_eq!(events.len(), 0, "empty journal after close cycles");
    }

    #[test]
    fn open_append_close_reopen_verify() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().to_path_buf();

        {
            let mut journal = FjallJournal::open(&path, None).expect("open 1");
            let run = RunId::new(42);
            journal
                .append_journaled(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: WorkflowDigest::from_bytes([0x99; 32]),
                })
                .expect("append");
            journal.close().expect("close 1");
        }

        {
            let mut journal = FjallJournal::open(&path, None).expect("open 2");
            let run = RunId::new(42);
            journal
                .append_journaled(&JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(0),
                    attempt: 1,
                })
                .expect("append 2");
            journal.close().expect("close 2");
        }

        let journal = FjallJournal::open(&path, None).expect("open final");
        let events = journal.events_for_run(RunId::new(42)).expect("replay");
        assert_eq!(events.len(), 2, "should have 2 events across close/reopen");
        assert_eq!(events[0].seq(), EventSeq::new(0));
        assert_eq!(events[1].seq(), EventSeq::new(1));
    }

    // =========================================================================
    // Record kind boundary tests
    // =========================================================================

    #[test]
    fn encode_rejects_unknown_magic() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            source: vec![1],
        };
        let result = encode_record(0xFFFF_0000, RecordKind::WorkflowSource, 0, &record, 128);
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "unknown magic with valid kind must be rejected"
        );
    }

    #[test]
    fn encode_accepts_run_header_with_index_magic() {
        let record = RunHeaderRecord {
            run: RunId::new(1),
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0u8; 32]),
            status: 0,
            accepted_at_ms: 100,
        };
        let result = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            1,
            &record,
            MAX_RUN_HEADER_BYTES,
        );
        assert!(
            result.is_ok(),
            "RunHeader kind 3 must be accepted by MAGIC_INDEX_RECORD"
        );
    }

    #[test]
    fn encode_accepts_index_update_with_index_magic() {
        let record = RunHeaderRecord {
            run: RunId::new(1),
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0u8; 32]),
            status: 0,
            accepted_at_ms: 100,
        };
        let result = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::IndexUpdate,
            1,
            &record,
            MAX_RUN_HEADER_BYTES,
        );
        assert!(
            result.is_ok(),
            "IndexUpdate kind 50 must be accepted by MAGIC_INDEX_RECORD"
        );
    }

    #[test]
    fn decode_rejects_zero_max_payload_with_nonzero_payload() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode should succeed");

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 0);
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "zero max_payload_len with nonzero payload must be rejected"
        );
    }

    #[test]
    fn encode_accepts_zero_length_payload_and_round_trips() {
        // `encode_record` does not reject empty `bytes`; the encoded envelope
        // still carries the BlobRecord's 32-byte digest header so the on-wire
        // record is non-empty. Verify the round trip preserves the empty payload.
        let record = BlobRecord {
            digest: [0u8; 32],
            bytes: vec![],
        };
        let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 1024)
            .expect("empty payload must encode successfully");
        assert!(
            !encoded.is_empty(),
            "envelope must carry the 32-byte digest header even when bytes is empty"
        );
        let (envelope, decoded): (crate::RecordEnvelope, BlobRecord) =
            decode_record(&encoded, MAGIC_BLOB, 1024).expect("round trip decode must succeed");
        assert_eq!(decoded.bytes, vec![], "empty payload must round-trip as empty");
        assert_eq!(decoded.digest, [0u8; 32], "digest must round-trip unchanged");
        assert_eq!(envelope.kind, RecordKind::Blob);
    }

    // =========================================================================
    // Batch edge cases
    // =========================================================================

    #[test]
    fn batch_commit_then_second_batch_with_same_run_seq_rejected() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
        };

        let mut batch1 = journal.batch();
        batch1.append_event(&event).expect("first batch append");
        batch1.commit().expect("first commit");

        let mut batch2 = journal.batch();
        let result = batch2.append_event(&event);
        assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "duplicate event in separate batch must be rejected"
        );
    }

    #[test]
    fn batch_len_zero_after_digest_mismatch_abort() {
        let (_temp, journal) = temp_journal();
        let source = b"real".to_vec();
        let wrong = WorkflowDigest::from_bytes([0xFF; 32]);
        let record = WorkflowSourceRecord {
            digest: wrong,
            source,
        };

        let mut batch = journal.batch();
        let _ = batch.put_workflow_source(&record);
        assert_eq!(
            batch.len(),
            0,
            "batch should report 0 after digest mismatch abort"
        );
    }

    #[test]
    fn empty_batch_strict_commits_successfully() {
        let (_temp, journal) = temp_journal();
        let batch = journal.batch();
        let batch = batch.strict();
        assert!(batch.is_empty());
        batch.commit().expect("empty strict batch commit");
    }

    // =========================================================================
    // Queue edge cases
    // =========================================================================

    #[test]
    fn queue_capacity_one_single_enqueue_dequeue() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(42);
        queue
            .enqueue_journaled(JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            })
            .expect("enqueue");
        queue.flush_batch(&journal).expect("flush");
        let events = journal.events_for_run(run).expect("replay");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn queue_drain_all_with_large_batch_relative_to_capacity() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(4, 10, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(100);
        for i in 0..4u64 {
            queue
                .enqueue_journaled(JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(i),
                    step: StepIdx::new(i as u16),
                    attempt: 1,
                })
                .expect("enqueue");
        }
        queue.drain_all(&journal).expect("drain_all");
        let events = journal.events_for_run(run).expect("replay");
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn queue_rejects_all_writes_after_shutdown() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(1);

        queue
            .enqueue_journaled(JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            })
            .expect("enqueue");

        queue.shutdown(&journal).expect("shutdown");

        let result_j = queue.enqueue_journaled(JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        });
        assert!(matches!(result_j, Err(JournalError::QueueShutdown)));

        let result_s = queue.enqueue_strict(JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            attempt: 1,
        });
        assert!(matches!(result_s, Err(JournalError::QueueShutdown)));
    }
}
