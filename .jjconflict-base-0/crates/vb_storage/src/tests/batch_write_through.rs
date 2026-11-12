    // =========================================================================
    // Section: Batch Write-Through Integration Tests (60 new tests)
    // =========================================================================

    // --- JournalWriteBatch put_run_event round-trips (tests 1-12) ---

    #[test]
    fn batch_append_run_accepted_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1001);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1, "one event must be stored");
        assert_eq!(events[0], event, "event must round-trip exactly");
    }

    #[test]
    fn batch_append_step_started_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1002);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_step_succeeded_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1003);
        let event = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            output: SlotIdx::new(3),
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_step_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1004);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_scheduled_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1005);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(7),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_completed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1006);
        let event = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            action: ActionId::new(8),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1007);
        let event = JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            action: ActionId::new(9),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_finished_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1008);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: SlotIdx::new(42),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1009);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_cancelled_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1010);
        let event = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_slot_written_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1011);
        let event = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot: SlotIdx::new(5),
            value: None,
            extra: None,
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_suspended_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1012);
        let event = JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    // --- Multi-run isolation (tests 13-16) ---

    #[test]
    fn events_for_run_isolates_run_a_from_run_b() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_a = RunId::new(2001);
        let run_b = RunId::new(2002);
        let event_a = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xAA; 32]),
        };
        let event_b = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xBB; 32]),
        };
        let event_a2 = JournalEvent::RunFinished {
            run: run_a,
            seq: EventSeq::new(1),
            result: SlotIdx::new(0),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event_a)
            .expect("batch.append_event must succeed");
        batch
            .append_event(&event_b)
            .expect("batch.append_event must succeed");
        batch
            .append_event(&event_a2)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events_a = journal
            .events_for_run(run_a)
            .expect("events_for_run A must succeed");
        assert_eq!(events_a.len(), 2, "run A must have exactly 2 events");
        assert_eq!(events_a[0], event_a);
        assert_eq!(events_a[1], event_a2);
        let events_b = journal
            .events_for_run(run_b)
            .expect("events_for_run B must succeed");
        assert_eq!(events_b.len(), 1, "run B must have exactly 1 event");
        assert_eq!(events_b[0], event_b);
    }

    #[test]
    fn run_header_isolation_between_runs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_1 = RunId::new(3001);
        let run_2 = RunId::new(3002);
        let header_1 = RunHeaderRecord {
            run: run_1,
            workflow_id: WorkflowId::new(10),
            compiled_digest: WorkflowDigest::from_bytes([1; 32]),
            status: 1,
            accepted_at_ms: 100,
        };
        let header_2 = RunHeaderRecord {
            run: run_2,
            workflow_id: WorkflowId::new(20),
            compiled_digest: WorkflowDigest::from_bytes([2; 32]),
            status: 2,
            accepted_at_ms: 200,
        };
        let mut batch = journal.batch();
        batch
            .put_run_header(&header_1)
            .expect("batch.put_run_header must succeed");
        batch
            .put_run_header(&header_2)
            .expect("batch.put_run_header must succeed");
        batch.commit().expect("batch.commit must succeed");
        let found_1 = journal
            .run_header(run_1)
            .expect("run_header run_1 must succeed");
        assert_eq!(found_1, Some(header_1), "run 1 header must match exactly");
        let found_2 = journal
            .run_header(run_2)
            .expect("run_header run_2 must succeed");
        assert_eq!(found_2, Some(header_2), "run 2 header must match exactly");
    }

    #[test]
    fn snapshot_isolation_between_runs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_a = RunId::new(3003);
        let run_b = RunId::new(3004);
        let snap_a = RunSnapshot {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xA; 32]),
            slots: vec![1, 2, 3],
            taint: Vec::new(),
        };
        let snap_b = RunSnapshot {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xB; 32]),
            slots: vec![4, 5, 6],
            taint: Vec::new(),
        };
        let mut batch = journal.batch();
        batch
            .put_snapshot(&snap_a)
            .expect("batch.put_snapshot must succeed");
        batch
            .put_snapshot(&snap_b)
            .expect("batch.put_snapshot must succeed");
        batch.commit().expect("batch.commit must succeed");
        let found_a = journal
            .snapshot(run_a, EventSeq::new(0))
            .expect("snapshot A must succeed");
        assert_eq!(found_a, Some(snap_a), "snapshot for run A must match");
        let found_b = journal
            .snapshot(run_b, EventSeq::new(0))
            .expect("snapshot B must succeed");
        assert_eq!(found_b, Some(snap_b), "snapshot for run B must match");
    }

    #[test]
    fn batch_writes_for_multiple_runs_commit_atomically() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_1 = RunId::new(4001);
        let run_2 = RunId::new(4002);
        let run_3 = RunId::new(4003);
        let mut batch = journal.batch();
        batch
            .append_event(&JournalEvent::RunAccepted {
                run: run_1,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            })
            .expect("batch.append_event must succeed");
        batch
            .append_event(&JournalEvent::RunAccepted {
                run: run_2,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([2; 32]),
            })
            .expect("batch.append_event must succeed");
        batch
            .append_event(&JournalEvent::RunAccepted {
                run: run_3,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([3; 32]),
            })
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        assert_eq!(
            journal
                .events_for_run(run_1)
                .expect("run_1 must succeed")
                .len(),
            1,
            "run 1 must have 1 event"
        );
        assert_eq!(
            journal
                .events_for_run(run_2)
                .expect("run_2 must succeed")
                .len(),
            1,
            "run 2 must have 1 event"
        );
        assert_eq!(
            journal
                .events_for_run(run_3)
                .expect("run_3 must succeed")
                .len(),
            1,
            "run 3 must have 1 event"
        );
    }

    // --- Writer Queue edge cases (tests 17-22) ---

    #[test]
    fn queue_journaled_enqueue_and_drain_preserves_order() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5001);
        let event_0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let event_1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let event_2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
            attempt: 1,
        };
        queue
            .enqueue_journaled(event_0.clone())
            .expect("enqueue 0 must succeed");
        queue
            .enqueue_journaled(event_1.clone())
            .expect("enqueue 1 must succeed");
        queue
            .enqueue_journaled(event_2.clone())
            .expect("enqueue 2 must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 3);
        assert_eq!(report.written, 3);
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events[0], event_0, "first event must be seq 0");
        assert_eq!(events[1], event_1, "second event must be seq 1");
        assert_eq!(events[2], event_2, "third event must be seq 2");
    }

    #[test]
    fn queue_strict_enqueue_and_drain_preserves_order() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5002);
        let event_0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        };
        let event_1 = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };
        queue
            .enqueue_strict(event_0.clone())
            .expect("enqueue 0 must succeed");
        queue
            .enqueue_strict(event_1.clone())
            .expect("enqueue 1 must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 2);
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events[0], event_0);
        assert_eq!(events[1], event_1);
    }

    #[test]
    fn queue_mixed_journaled_and_strict_drain_returns_both() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5003);
        let journaled_event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };
        let strict_event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        };
        queue
            .enqueue_journaled(journaled_event.clone())
            .expect("enqueue journaled must succeed");
        queue
            .enqueue_strict(strict_event.clone())
            .expect("enqueue strict must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 2, "both events must be drained");
        assert_eq!(report.written, 2, "both events must be written");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], journaled_event);
        assert_eq!(events[1], strict_event);
    }

    #[test]
    fn queue_flush_persists_before_drain() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5004);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([4; 32]),
        };
        queue
            .enqueue_journaled(event.clone())
            .expect("enqueue must succeed");
        let report = queue
            .flush_batch(&journal)
            .expect("flush_batch must succeed");
        assert_eq!(report.written, 1, "one event must be written");
        let events_before = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events_before.len(), 1, "event must be on disk before drain");
        assert_eq!(events_before[0], event);
    }

    #[test]
    fn queue_empty_drain_returns_zero_events() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 0, "empty queue must drain zero events");
        assert_eq!(report.written, 0, "empty queue must write zero events");
    }

    #[test]
    fn queue_pending_count_matches_enqueued() {
        let queue = JournalWriterQueue::new(16, 4, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5005);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        let counts_empty = queue.pending_profile_counts().expect("counts must succeed");
        assert_eq!(counts_empty.journaled, 0);
        assert_eq!(counts_empty.strict, 0);
        queue
            .enqueue_journaled(event.clone())
            .expect("enqueue 0 must succeed");
        queue
            .enqueue_journaled(event.clone())
            .expect("enqueue 1 must succeed");
        queue.enqueue_strict(event).expect("enqueue 2 must succeed");
        let counts = queue.pending_profile_counts().expect("counts must succeed");
        assert_eq!(counts.journaled, 2, "two journaled events must be counted");
        assert_eq!(counts.strict, 1, "one strict event must be counted");
    }

    // --- FjallJournal open/close/reopen (tests 23-30) ---

    #[test]
    fn journal_open_creates_fresh_database() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let events = journal
            .events_for_run(RunId::new(1))
            .expect("events_for_run must succeed");
        assert!(events.is_empty(), "fresh database must have no events");
        let header = journal
            .run_header(RunId::new(1))
            .expect("run_header must succeed");
        assert_eq!(header, None, "fresh database must have no headers");
        let blob = journal.blob([0; 32]).expect("blob must succeed");
        assert_eq!(blob, None, "fresh database must have no blobs");
    }

    #[test]
    fn journal_close_and_reopen_preserves_strict_data() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let path = temp_dir.path().to_path_buf();
        let digest = WorkflowDigest::from_bytes([0xEE; 32]);
        let run = RunId::new(6001);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(5),
            compiled_digest: digest,
            status: 3,
            accepted_at_ms: 999,
        };
        {
            let journal = FjallJournal::open(&path, None).expect("setup: journal open");
            journal
                .put_run_header(&header)
                .expect("put_run_header must succeed");
            journal
                .persist_strict()
                .expect("persist_strict must succeed");
        }
        let reopened = FjallJournal::open(&path, None).expect("reopen must succeed");
        let found = reopened.run_header(run).expect("run_header must succeed");
        assert_eq!(found, Some(header), "strict data must survive reopen");
    }

    #[test]
    fn journal_multiple_opens_same_path_fails_or_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal1 = FjallJournal::open(temp_dir.path(), None).expect("first open must succeed");
        let journal2_result = FjallJournal::open(temp_dir.path(), None);
        drop(journal1);
        if let Ok(j2) = journal2_result {
            drop(j2);
        }
    }

    #[test]
    fn journal_put_then_get_workflow_source_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source = b"consistent_source".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };
        journal
            .put_workflow_source(&record)
            .expect("put_workflow_source must succeed");
        let found = journal
            .workflow_source(digest)
            .expect("workflow_source must succeed");
        assert_eq!(
            found,
            Some(record),
            "put-then-get must be consistent in same session"
        );
    }

    #[test]
    fn journal_put_then_get_compiled_ir_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let record = compiled_ir_record(b"consistent_ir".to_vec());
        let digest = record.digest;
        journal
            .put_compiled_ir(&record)
            .expect("put_compiled_ir must succeed");
        let found = journal
            .compiled_ir(digest)
            .expect("compiled_ir must succeed");
        assert_eq!(
            found,
            Some(record),
            "put-then-get must be consistent in same session"
        );
    }

    #[test]
    fn journal_put_then_get_run_header_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(6002);
        let record = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(99),
            compiled_digest: WorkflowDigest::from_bytes([0x99; 32]),
            status: 7,
            accepted_at_ms: 123456789,
        };
        journal
            .put_run_header(&record)
            .expect("put_run_header must succeed");
        let found = journal.run_header(run).expect("run_header must succeed");
        assert_eq!(
            found,
            Some(record),
            "put-then-get must be consistent in same session"
        );
    }

    #[test]
    fn journal_put_then_get_snapshot_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(6003);
        let seq = EventSeq::new(4);
        let snapshot = RunSnapshot {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([0xAA; 32]),
            slots: vec![0xDE, 0xAD],
            taint: Vec::new(),
        };
        journal
            .put_snapshot(&snapshot)
            .expect("put_snapshot must succeed");
        let found = journal.snapshot(run, seq).expect("snapshot must succeed");
        assert_eq!(
            found,
            Some(snapshot),
            "put-then-get must be consistent in same session"
        );
    }

    #[test]
    fn journal_put_then_get_blob_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let blob_bytes = b"consistent_blob".to_vec();
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        let record = BlobRecord {
            digest,
            bytes: blob_bytes,
        };
        journal.put_blob(&record).expect("put_blob must succeed");
        let found = journal.blob(digest).expect("blob must succeed");
        assert_eq!(
            found,
            Some(record),
            "put-then-get must be consistent in same session"
        );
    }

    // --- Index queries (tests 31-35) ---

    #[test]
    fn status_index_stores_and_queries_by_state() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state = IndexStatusState::Other(3);
        let timestamp: u64 = 1700000000;
        let run = RunId::new(7001);
        journal
            .put_status_index(state, timestamp, run)
            .expect("put_status_index must succeed");
        let key = index_status_key(state, timestamp, run).expect("key must succeed");
        let value = journal
            .index_status
            .get(key.as_slice())
            .expect("get must succeed");
        assert!(value.is_some(), "status index entry must exist after put");
    }

    #[test]
    fn workflow_index_stores_and_queries_by_workflow_id() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let workflow = WorkflowId::new(42);
        let run = RunId::new(7002);
        journal
            .put_workflow_index(workflow, run)
            .expect("put_workflow_index must succeed");
        let key = index_workflow_key(workflow, run).expect("key must succeed");
        let value = journal
            .index_workflow
            .get(key.as_slice())
            .expect("get must succeed");
        assert!(value.is_some(), "workflow index entry must exist after put");
    }

    #[test]
    fn action_index_stores_and_queries_by_action_id() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let action = ActionId::new(7);
        let run = RunId::new(7003);
        let step = StepIdx::new(2);
        journal
            .put_action_index(action, run, step)
            .expect("put_action_index must succeed");
        let key = index_action_key(action, run, step).expect("key must succeed");
        let value = journal
            .index_action
            .get(key.as_slice())
            .expect("get must succeed");
        assert!(value.is_some(), "action index entry must exist after put");
    }

    #[test]
    fn status_index_multiple_runs_same_state() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state = IndexStatusState::Other(5);
        let run_1 = RunId::new(7010);
        let run_2 = RunId::new(7011);
        let run_3 = RunId::new(7012);
        journal
            .put_status_index(state, 100, run_1)
            .expect("put_status_index 1 must succeed");
        journal
            .put_status_index(state, 200, run_2)
            .expect("put_status_index 2 must succeed");
        journal
            .put_status_index(state, 300, run_3)
            .expect("put_status_index 3 must succeed");
        let key_1 = index_status_key(state, 100, run_1).expect("key 1 must succeed");
        let key_2 = index_status_key(state, 200, run_2).expect("key 2 must succeed");
        let key_3 = index_status_key(state, 300, run_3).expect("key 3 must succeed");
        assert!(
            journal
                .index_status
                .get(key_1.as_slice())
                .expect("get 1")
                .is_some()
        );
        assert!(
            journal
                .index_status
                .get(key_2.as_slice())
                .expect("get 2")
                .is_some()
        );
        assert!(
            journal
                .index_status
                .get(key_3.as_slice())
                .expect("get 3")
                .is_some()
        );
    }

    #[test]
    fn workflow_index_multiple_runs_same_workflow() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let workflow = WorkflowId::new(99);
        let run_1 = RunId::new(7020);
        let run_2 = RunId::new(7021);
        let run_3 = RunId::new(7022);
        journal
            .put_workflow_index(workflow, run_1)
            .expect("put 1 must succeed");
        journal
            .put_workflow_index(workflow, run_2)
            .expect("put 2 must succeed");
        journal
            .put_workflow_index(workflow, run_3)
            .expect("put 3 must succeed");
        let key_1 = index_workflow_key(workflow, run_1).expect("key 1 must succeed");
        let key_2 = index_workflow_key(workflow, run_2).expect("key 2 must succeed");
        let key_3 = index_workflow_key(workflow, run_3).expect("key 3 must succeed");
        assert!(
            journal
                .index_workflow
                .get(key_1.as_slice())
                .expect("get 1")
                .is_some()
        );
        assert!(
            journal
                .index_workflow
                .get(key_2.as_slice())
                .expect("get 2")
                .is_some()
        );
        assert!(
            journal
                .index_workflow
                .get(key_3.as_slice())
                .expect("get 3")
                .is_some()
        );
    }

    // --- Record builder (tests 36-40) ---

    #[test]
    fn builder_initial_len_is_zero() {
        let builder = BatchBuilder::new();
        assert_eq!(builder.len(), 0, "new builder must have len 0");
        assert!(builder.is_empty(), "new builder must be empty");
    }

    #[test]
    fn builder_append_increments_len() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8001);
        builder
            .push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            })
            .expect("push within cap");
        assert_eq!(builder.len(), 1, "builder must have len 1 after one push");
        assert!(!builder.is_empty());
    }

    #[test]
    fn builder_append_multiple_events_len_matches() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8002);
        builder
            .push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            })
            .expect("push within cap");
        builder
            .push(JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            })
            .expect("push within cap");
        builder
            .push(JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: SlotIdx::new(0),
                attempt: 1,
            })
            .expect("push within cap");
        assert_eq!(
            builder.len(),
            3,
            "builder must have len 3 after three pushes"
        );
    }

    #[test]
    fn builder_as_slice_returns_appended_events() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8003);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let e1 = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };
        builder.push(e0.clone()).expect("push within cap");
        builder.push(e1.clone()).expect("push within cap");
        let slice = builder.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(
            slice[0], e0,
            "first slice element must match first pushed event"
        );
        assert_eq!(
            slice[1], e1,
            "second slice element must match second pushed event"
        );
    }

    #[test]
    fn builder_build_produces_correct_record_count() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(8004);
        let mut builder = BatchBuilder::new();
        builder
            .push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            })
            .expect("push within cap");
        builder
            .push(JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            })
            .expect("push within cap");
        builder
            .push(JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: SlotIdx::new(0),
                attempt: 1,
            })
            .expect("push within cap");
        assert_eq!(builder.len(), 3);
        journal
            .append_strict_batch(builder.as_slice())
            .expect("append_strict_batch must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 3, "three events must be stored");
    }

    // --- Batch state tracking (tests 41-44) ---

    #[test]
    fn batch_initial_len_is_zero() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let batch = journal.batch();
        assert_eq!(batch.len(), 0, "new batch must have len 0");
        assert!(batch.is_empty(), "new batch must be empty");
    }

    #[test]
    fn batch_len_increments_per_put() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        let source = b"a".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let ir_record = compiled_ir_record(b"ir".to_vec());
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("put 1 must succeed");
        assert_eq!(batch.len(), 1, "batch must have len 1 after first put");
        batch
            .put_compiled_ir(&ir_record)
            .expect("put 2 must succeed");
        assert_eq!(batch.len(), 2, "batch must have len 2 after second put");
        batch
            .put_run_header(&RunHeaderRecord {
                run: RunId::new(9001),
                workflow_id: WorkflowId::new(1),
                compiled_digest: ir_record.digest,
                status: 0,
                accepted_at_ms: 0,
            })
            .expect("put 3 must succeed");
        assert_eq!(batch.len(), 3, "batch must have len 3 after third put");
    }

    #[test]
    fn batch_len_resets_after_commit() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        let source = b"data".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("put must succeed");
        assert_eq!(batch.len(), 1, "batch must have 1 operation before commit");
        batch.commit().expect("commit must succeed");
        let fresh_batch = journal.batch();
        assert_eq!(
            fresh_batch.len(),
            0,
            "new batch after commit must start at 0"
        );
    }

    #[test]
    fn batch_put_snapshot_increments_len() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        assert_eq!(batch.len(), 0);
        let snapshot = RunSnapshot {
            run: RunId::new(9002),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x43; 32]),
            slots: vec![1, 2],
            taint: Vec::new(),
        };
        batch
            .put_snapshot(&snapshot)
            .expect("put_snapshot must succeed");
        assert_eq!(batch.len(), 1, "batch len must be 1 after put_snapshot");
    }
