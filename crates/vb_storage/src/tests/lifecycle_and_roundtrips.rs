    // --- Section 4: Journal Lifecycle BDD Tests ---

    fn open_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("journal should open");
        (temp_dir, journal)
    }

    fn test_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    #[test]
    fn journal_open_creates_fresh_instance_with_no_data() {
        // Given a temporary directory
        // When FjallJournal::open is called
        // Then the journal has no events for any run
        let (_guard, journal) = open_journal();
        let events = journal
            .events_for_run(RunId::new(1))
            .expect("events_for_run should succeed on empty journal");
        assert!(events.is_empty());
    }

    #[test]
    fn append_strict_writes_submitted_event_with_correct_run_id() {
        // Given an open journal
        // When append_strict is called with a RunAccepted event for run 42
        // Then the stored event has run_id 42
        let (_guard, journal) = open_journal();
        let run = RunId::new(42);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].run_id(), run);
    }

    #[test]
    fn append_strict_writes_accepted_event_after_submitted() {
        // Given an open journal with a RunAccepted event at seq 0
        // When a StepStarted event at seq 1 is appended
        // Then both events are retrieved in order
        let (_guard, journal) = open_journal();
        let run = RunId::new(1);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let started = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        journal
            .append_strict(&accepted)
            .expect("journal.append_strict must succeed");
        journal
            .append_strict(&started)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], accepted);
        assert_eq!(events[1], started);
    }

    #[test]
    fn append_strict_writes_step_started_event_with_correct_step() {
        // Given an open journal
        // When a StepStarted event with step 5 is appended and retrieved
        // Then the event carries step 5
        let (_guard, journal) = open_journal();
        let run = RunId::new(10);
        let step = StepIdx::new(5);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::StepStarted {
            step: found_step, ..
        } = events[0]
        else {
            panic!("expected StepStarted event");
        };
        assert_eq!(found_step, step);
    }

    #[test]
    fn append_strict_writes_step_ended_event_with_correct_step() {
        // Given an open journal
        // When a StepSucceeded event with step 3 is appended and retrieved
        // Then the event carries step 3 and output slot 7
        let (_guard, journal) = open_journal();
        let run = RunId::new(11);
        let step = StepIdx::new(3);
        let output = vb_core::SlotIdx::new(7);
        let event = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step,
            output,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::StepSucceeded {
            step: found_step,
            output: found_output,
            ..
        } = events[0]
        else {
            panic!("expected StepSucceeded event");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_output, output);
    }

    #[test]
    fn append_strict_writes_slot_written_event_with_correct_slot() {
        // Given an open journal
        // When a SlotWrittenEvent with slot 9 is appended and retrieved
        // Then the event carries slot 9
        let (_guard, journal) = open_journal();
        let run = RunId::new(12);
        let slot = vb_core::SlotIdx::new(9);
        let event = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot,
            value: None,
            extra: None,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::SlotWrittenEvent {
            slot: found_slot, ..
        } = events[0]
        else {
            panic!("expected SlotWrittenEvent");
        };
        assert_eq!(found_slot, slot);
    }

    #[test]
    fn append_strict_writes_action_scheduled_event_with_correct_step() {
        // Given an open journal
        // When an ActionScheduled event with step 4 is appended and retrieved
        // Then the event carries step 4 and action 2
        let (_guard, journal) = open_journal();
        let run = RunId::new(13);
        let step = StepIdx::new(4);
        let action = ActionId::new(2);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::ActionScheduled {
            step: found_step,
            action: found_action,
            ..
        } = events[0]
        else {
            panic!("expected ActionScheduled event");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_action, action);
    }

    #[test]
    fn append_strict_writes_action_completed_event_with_correct_step() {
        // Given an open journal
        // When an ActionCompletedEvent with step 6 is appended and retrieved
        // Then the event carries step 6 and action 3
        let (_guard, journal) = open_journal();
        let run = RunId::new(14);
        let step = StepIdx::new(6);
        let action = ActionId::new(3);
        let event = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::ActionCompletedEvent {
            step: found_step,
            action: found_action,
            ..
        } = events[0]
        else {
            panic!("expected ActionCompletedEvent");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_action, action);
    }

    #[test]
    fn append_strict_writes_run_finished_event_with_correct_result() {
        // Given an open journal
        // When a RunFinished event with result slot 15 is appended and retrieved
        // Then the event carries result 15
        let (_guard, journal) = open_journal();
        let run = RunId::new(15);
        let result = vb_core::SlotIdx::new(15);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::RunFinished {
            result: found_result,
            ..
        } = events[0]
        else {
            panic!("expected RunFinished event");
        };
        assert_eq!(found_result, result);
    }

    #[test]
    fn append_strict_writes_run_failed_event() {
        // Given an open journal
        // When a RunFailedEvent is appended and retrieved
        // Then the event carries the correct run
        let (_guard, journal) = open_journal();
        let run = RunId::new(16);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].run_id(), run);
    }

    #[test]
    fn append_strict_assigns_monotonically_increasing_sequences() {
        // Given an open journal
        // When three events are appended with seq 0, 1, 2
        // Then events_for_run returns them in contiguous order
        let (_guard, journal) = open_journal();
        let run = RunId::new(17);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let e2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        };
        journal
            .append_strict(&e0)
            .expect("journal.append_strict must succeed");
        journal
            .append_strict(&e1)
            .expect("journal.append_strict must succeed");
        journal
            .append_strict(&e2)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].seq(), EventSeq::new(0));
        assert_eq!(events[1].seq(), EventSeq::new(1));
        assert_eq!(events[2].seq(), EventSeq::new(2));
    }

    #[test]
    fn append_strict_rejects_duplicate_sequence() {
        // Given an open journal with an event at seq 0 for run 50
        // When the same event is appended again
        // Then DuplicateEvent is returned with exact run and seq
        let (_guard, journal) = open_journal();
        let run = RunId::new(50);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let result = journal.append_strict(&event);
        let Err(JournalError::DuplicateEvent {
            run: dup_run,
            seq: dup_seq,
        }) = result
        else {
            panic!("expected DuplicateEvent, got {:?}", result);
        };
        assert_eq!(dup_run, run);
        assert_eq!(dup_seq, EventSeq::new(0));
    }

    #[test]
    fn events_for_run_returns_events_in_sequence_order() {
        // Given a journal with 5 events for a run
        // When events_for_run is called
        // Then events are returned in ascending sequence order
        let (_guard, journal) = open_journal();
        let run = RunId::new(18);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let e2 = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: vb_core::SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        };
        let e3 = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: vb_core::SlotIdx::new(1),
        };
        let e4 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(4),
            result: vb_core::SlotIdx::new(1),
            attempt: 1,
        };
        journal
            .append_journaled(&e0)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&e1)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&e2)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&e3)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&e4)
            .expect("journal.append_journaled must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 5);
        assert_eq!(events[0], e0);
        assert_eq!(events[1], e1);
        assert_eq!(events[2], e2);
        assert_eq!(events[3], e3);
        assert_eq!(events[4], e4);
    }

    #[test]
    fn events_for_run_returns_empty_for_run_with_no_events() {
        // Given an open journal with events for run 1
        // When events_for_run is called for run 2
        // Then it returns an empty vec
        let (_guard, journal) = open_journal();
        let run_a = RunId::new(1);
        let event = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        journal
            .append_journaled(&event)
            .expect("journal.append_journaled must succeed");

        let events = journal
            .events_for_run(RunId::new(2))
            .expect("events_for_run should succeed");
        assert!(events.is_empty());
    }

    #[test]
    fn append_strict_handles_concurrent_runs_interleaved() {
        // Given a journal with interleaved events from run A and run B
        // When events_for_run is called for run A
        // Then only run A events are returned in order
        let (_guard, journal) = open_journal();
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);

        let a0 = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let b0 = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: test_digest(2),
        };
        let a1 = JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let b1 = JournalEvent::StepStarted {
            run: run_b,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let a2 = JournalEvent::RunFinished {
            run: run_a,
            seq: EventSeq::new(2),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        };

        journal
            .append_journaled(&a0)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&b0)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&a1)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&b1)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&a2)
            .expect("journal.append_journaled must succeed");

        let events_a = journal
            .events_for_run(run_a)
            .expect("events_for_run A should succeed");
        assert_eq!(events_a.len(), 3);
        assert_eq!(events_a[0], a0);
        assert_eq!(events_a[1], a1);
        assert_eq!(events_a[2], a2);

        let events_b = journal
            .events_for_run(run_b)
            .expect("events_for_run B should succeed");
        assert_eq!(events_b.len(), 2);
        assert_eq!(events_b[0], b0);
        assert_eq!(events_b[1], b1);
    }

    #[test]
    fn append_journaled_succeeds_without_flush() {
        // Given an open journal
        // When append_journaled is called
        // Then the event is readable immediately
        let (_guard, journal) = open_journal();
        let run = RunId::new(30);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        journal
            .append_journaled(&event)
            .expect("journal.append_journaled must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn run_header_record_roundtrip_with_large_timestamp() {
        // Given a run header with a large accepted_at_ms value
        // When put and retrieved
        // Then the timestamp survives exactly
        let (_guard, journal) = open_journal();
        let record = RunHeaderRecord {
            run: RunId::new(1),
            workflow_id: WorkflowId::new(2),
            compiled_digest: test_digest(5),
            status: 0,
            accepted_at_ms: u64::MAX / 2,
        };
        journal
            .put_run_header(&record)
            .expect("journal.put_run_header must succeed");

        let retrieved = journal
            .run_header(RunId::new(1))
            .expect("lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn snapshot_record_roundtrip_with_nonempty_slots() {
        // Given a snapshot with non-empty slot data
        // When stored and retrieved
        // Then the slot bytes survive exactly
        let (_guard, journal) = open_journal();
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(7),
            slots: vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE],
            taint: Vec::new(),
        };
        journal
            .put_snapshot(&snapshot)
            .expect("journal.put_snapshot must succeed");

        let retrieved = journal
            .snapshot(RunId::new(1), EventSeq::new(0))
            .expect("lookup should succeed");
        assert_eq!(retrieved, Some(snapshot));
    }

    #[test]
    fn compiled_ir_returns_none_when_different_digest_queried() {
        // Given an open journal with a compiled IR stored at digest [1;32]
        // When a different digest [2;32] is queried
        // Then it returns None
        let (_guard, journal) = open_journal();
        let record = compiled_ir_record(vec![1, 2, 3]);
        journal
            .put_compiled_ir(&record)
            .expect("journal.put_compiled_ir must succeed");

        let result = journal
            .compiled_ir(test_digest(2))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn workflow_source_returns_none_for_different_digest() {
        // Given an open journal with one workflow source stored
        // When a different digest is queried
        // Then it returns None
        let (_guard, journal) = open_journal();
        let source = vec![1];
        let stored_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest: stored_digest,
            source,
        };
        journal
            .put_workflow_source(&record)
            .expect("journal.put_workflow_source must succeed");

        let result = journal
            .workflow_source(test_digest(11))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    fn sample_action_ticket(run: RunId, seq: EventSeq) -> ActionTicket {
        ActionTicket {
            run,
            step: StepIdx::new(0),
            seq: SeqNo::new(seq.get()),
            action: ActionId::new(1),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: 7,
            capacity: 3,
            fencing_token: 0,
        }
    }

    fn journal_event_variants_for_run_and_seq(
        run: RunId,
        seq: EventSeq,
    ) -> Vec<(JournalEvent, RecordKind)> {
        let ticket = sample_action_ticket(run, seq);
        let timestamp = chrono::Utc::now();
        vec![
            (
                JournalEvent::RunAccepted {
                    run,
                    seq,
                    workflow: test_digest(1),
                },
                RecordKind::RunAccepted,
            ),
            (
                JournalEvent::RunAdmission {
                    input_digest: crate::RunInputBinding::LegacyUnbound,
                    run,
                    seq,
                    artifact_digest: test_digest(2),
                    granted_capabilities: CapabilitySet::empty(),
                    policy: RuntimePolicy::Strict,
                    policy_pin: crate::RunPolicyPin::Unverified,
                },
                RecordKind::RunAdmission,
            ),
            (
                JournalEvent::StepStarted {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                RecordKind::StepStarted,
            ),
            (
                JournalEvent::StepSucceeded {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    output: SlotIdx::new(0),
                },
                RecordKind::StepSucceeded,
            ),
            (
                JournalEvent::StepFailed {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                RecordKind::StepFailed,
            ),
            (
                JournalEvent::ActionScheduled {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    action: ActionId::new(1),
                    attempt: 1,
                },
                RecordKind::ActionScheduled,
            ),
            (
                JournalEvent::ActionCompletedEvent {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    action: ActionId::new(1),
                    attempt: 1,
                },
                RecordKind::ActionCompleted,
            ),
            (
                JournalEvent::ActionScheduledTicket {
                    run,
                    seq,
                    ticket,
                    input: SlotIdx::new(1),
                    output: SlotIdx::new(2),
                    action_abi_digest: test_digest(3),
                },
                RecordKind::ActionScheduledTicket,
            ),
            (
                JournalEvent::ActionCompletedEnvelope {
                    run,
                    seq,
                    ticket,
                    output: SlotIdx::new(2),
                    outcome: DurableActionOutcome::Ready,
                    value: vec![1, 2, 3],
                    encoded_len: 3,
                    taint: Taint::Clean,
                    value_digest: [4; 32],
                    action_abi_digest: test_digest(4),
                },
                RecordKind::ActionCompletedEnvelope,
            ),
            (
                JournalEvent::ActionFailedEvent {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    action: ActionId::new(1),
                    attempt: 1,
                },
                RecordKind::ActionFailed,
            ),
            (
                JournalEvent::ActionAbandoned { run, seq, ticket },
                RecordKind::ActionAbandoned,
            ),
            (
                JournalEvent::SlotWrittenEvent {
                    run,
                    seq,
                    slot: SlotIdx::new(0),
                    value: None,
                    extra: None,
                    attempt: 1,
                },
                RecordKind::SlotWritten,
            ),
            (
                JournalEvent::WaitScheduledEvent {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                RecordKind::WaitScheduled,
            ),
            (
                JournalEvent::AskScheduledEvent {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                RecordKind::AskScheduled,
            ),
            (
                JournalEvent::AskAnsweredEvent {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                RecordKind::AskAnswered,
            ),
            (
                JournalEvent::WaitResolvedEvent {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                RecordKind::WaitResolved,
            ),
            (
                JournalEvent::RetryScheduledEvent {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                RecordKind::RetryScheduled,
            ),
            (
                JournalEvent::RunCancelled {
                    run,
                    seq,
                    attempt: 1,
                    reason: None,
                },
                RecordKind::RunCancelled,
            ),
            (
                JournalEvent::RunKilled {
                    run,
                    seq,
                    attempt: 1,
                },
                RecordKind::RunKilled,
            ),
            (
                JournalEvent::RunFinished {
                    run,
                    seq,
                    result: SlotIdx::new(0),
                    attempt: 1,
                },
                RecordKind::RunFinished,
            ),
            (
                JournalEvent::RunFailedEvent {
                    run,
                    seq,
                    attempt: 1,
                },
                RecordKind::RunFailed,
            ),
            (
                JournalEvent::RunResumed {
                    run,
                    seq,
                    timestamp: timestamp.to_owned(),
                },
                RecordKind::RunResumed,
            ),
            (
                JournalEvent::RunRetried {
                    run,
                    seq,
                    timestamp: timestamp.to_owned(),
                },
                RecordKind::RunRetried,
            ),
            (
                JournalEvent::RunAnswered {
                    run,
                    seq,
                    slot_idx: SlotIdx::new(0),
                    answer: ConstValue::Null,
                    timestamp,
                },
                RecordKind::RunAnswered,
            ),
            (
                JournalEvent::AskTimedOutEvent {
                    run,
                    seq,
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                RecordKind::AskTimedOut,
            ),
        ]
    }

    fn record_kind_count(cases: &[(JournalEvent, RecordKind)], kind: RecordKind) -> usize {
        cases
            .iter()
            .map(|(_, expected_kind)| *expected_kind)
            .filter(|expected_kind| *expected_kind == kind)
            .count()
    }

    #[test]
    fn journal_event_variant_fixture_covers_each_record_kind_once() {
        let cases = journal_event_variants_for_run_and_seq(RunId::new(7), EventSeq::new(8));

        assert_eq!(cases.len(), 25);
        assert_eq!(record_kind_count(&cases, RecordKind::RunAccepted), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RunAdmission), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::StepStarted), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::StepSucceeded), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::StepFailed), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::ActionScheduled), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::ActionCompleted), 1);
        assert_eq!(
            record_kind_count(&cases, RecordKind::ActionScheduledTicket),
            1
        );
        assert_eq!(
            record_kind_count(&cases, RecordKind::ActionCompletedEnvelope),
            1
        );
        assert_eq!(record_kind_count(&cases, RecordKind::ActionFailed), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::ActionAbandoned), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::SlotWritten), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::WaitScheduled), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::AskScheduled), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::AskAnswered), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::WaitResolved), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RetryScheduled), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RunCancelled), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RunKilled), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RunFinished), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RunFailed), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RunResumed), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RunRetried), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::RunAnswered), 1);
        assert_eq!(record_kind_count(&cases, RecordKind::AskTimedOut), 1);
    }

    #[test]
    fn record_kind_magic_family_keeps_journal_kinds_under_journal_magic() {
        assert_eq!(
            RecordKind::RunAdmission.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::RunResumed.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::RunRetried.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::RunAnswered.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::WaitResolved.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::ActionAbandoned.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::StepSucceeded.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::ActionScheduledTicket.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::ActionCompletedEnvelope.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            true
        );
        assert_eq!(
            RecordKind::RunKilled.belongs_to_magic(MAGIC_SNAPSHOT),
            false
        );
        assert_eq!(RecordKind::RunKilled.belongs_to_magic(MAGIC_BLOB), false);
        assert_eq!(
            RecordKind::WaitResolved.belongs_to_magic(MAGIC_SNAPSHOT),
            false
        );
        assert_eq!(
            RecordKind::ActionCompletedEnvelope.belongs_to_magic(MAGIC_BLOB),
            false
        );
        assert_eq!(
            RecordKind::Snapshot.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            false
        );
        assert_eq!(
            RecordKind::Blob.belongs_to_magic(MAGIC_JOURNAL_EVENT),
            false
        );
        assert_eq!(
            RecordKind::RunHeader.belongs_to_magic(MAGIC_INDEX_RECORD),
            true
        );
        assert_eq!(
            RecordKind::IndexUpdate.belongs_to_magic(MAGIC_INDEX_RECORD),
            true
        );
    }

    #[test]
    fn journal_event_run_id_returns_correct_run_for_all_variants() {
        // Given every JournalEvent variant with run_id 99
        // When run_id() is called
        // Then each returns RunId::new(99)
        let run = RunId::new(99);
        for (event, _) in journal_event_variants_for_run_and_seq(run, EventSeq::new(0)) {
            assert_eq!(event.run_id(), run);
        }
    }

    #[test]
    fn journal_event_seq_returns_correct_seq_for_all_variants() {
        // Given every JournalEvent variant with seq 42
        // When seq() is called
        // Then each returns EventSeq::new(42)
        let seq = EventSeq::new(42);
        let run = RunId::new(1);
        for (event, _) in journal_event_variants_for_run_and_seq(run, seq) {
            assert_eq!(event.seq(), seq);
        }
    }

    #[test]
    fn journal_event_record_kind_returns_correct_kind_for_all_variants() {
        // Given every JournalEvent variant
        // When record_kind() is called
        // Then each returns the expected RecordKind
        let run = RunId::new(1);
        let seq = EventSeq::new(0);
        for (event, expected_kind) in journal_event_variants_for_run_and_seq(run, seq) {
            assert_eq!(event.record_kind(), expected_kind);
        }
    }

    // --- Section 5: Encode/Decode Roundtrip Tests ---

    #[test]
    fn encode_decode_roundtrip_for_run_accepted_record() {
        // Given a RunAccepted event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(42),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_step_started_record() {
        // Given a StepStarted event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::StepStarted {
            run: RunId::new(2),
            seq: EventSeq::new(1),
            step: StepIdx::new(5),
            attempt: 1,
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::StepStarted, 1, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_step_ended_record() {
        // Given a StepSucceeded event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::StepSucceeded {
            run: RunId::new(3),
            seq: EventSeq::new(2),
            step: StepIdx::new(5),
            output: vb_core::SlotIdx::new(10),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::StepSucceeded,
            2,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_slot_written_record() {
        // Given a SlotWrittenEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(4),
            seq: EventSeq::new(3),
            slot: vb_core::SlotIdx::new(7),
            value: None,
            extra: None,
            attempt: 1,
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::SlotWritten, 3, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_action_scheduled_record() {
        // Given an ActionScheduled event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionScheduled {
            run: RunId::new(5),
            seq: EventSeq::new(4),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionScheduled,
            4,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_action_completed_record() {
        // Given an ActionCompletedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionCompletedEvent {
            run: RunId::new(6),
            seq: EventSeq::new(5),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionCompleted,
            5,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_run_finished_record() {
        // Given a RunFinished event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunFinished {
            run: RunId::new(7),
            seq: EventSeq::new(6),
            result: vb_core::SlotIdx::new(99),
            attempt: 1,
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunFinished, 6, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_run_failed_record() {
        // Given a RunFailedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunFailedEvent {
            run: RunId::new(8),
            seq: EventSeq::new(7),
            attempt: 1,
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunFailed, 7, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_record_rejects_record_exceeding_max_payload() {
        // Given a workflow source with 200 bytes of source data
        // When encode_record is called with max_payload_len of 10
        // Then it returns PayloadTooLarge
        let source = WorkflowSourceRecord {
            digest: test_digest(1),
            source: vec![0u8; 200],
        };
        let result = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &source,
            10,
        );
        let Err(JournalError::PayloadTooLarge { len, max }) = result else {
            panic!("expected PayloadTooLarge, got {:?}", result);
        };
        assert_eq!(max, 10);
        assert!(len > 10);
    }

    #[test]
    fn encode_decode_roundtrip_for_action_failed_record() {
        // Given an ActionFailedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionFailedEvent {
            run: RunId::new(9),
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            action: ActionId::new(4),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionFailed,
            3,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    // --- Section 6: JournalError Variant Tests ---

    #[test]
    fn journal_error_encode_from_postcard_error() {
        // Given a payload that causes a postcard encoding error
        // When encode_record encounters the error
        // Then JournalError::Encode is returned
        // This is tested indirectly: encode_record with a valid payload succeeds,
        // and the Encode variant exists as a From<postcard::Error> conversion.
        // We verify the variant exists by checking the error display.
        let err = JournalError::Encode(postcard::Error::DeserializeBadVarint);
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    #[test]
    fn journal_error_key_capacity_display() {
        // Given a JournalError::KeyCapacity
        // When displayed
        // Then the message is non-empty
        let err = JournalError::KeyCapacity;
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    #[test]
    fn journal_error_write_lock_poisoned_display() {
        // Given a JournalError::WriteLockPoisoned
        // When displayed
        // Then the message mentions poisoned
        let err = JournalError::WriteLockPoisoned;
        let msg = format!("{}", err);
        assert!(msg.contains("poisoned"));
    }

    #[test]
    fn journal_error_wrong_run_display() {
        // Given a JournalError::WrongRun with expected and actual
        // When displayed
        // Then the message contains both run values
        let err = JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("1"));
        assert!(msg.contains("2"));
    }

    #[test]
    fn journal_error_sequence_overflow_display() {
        // Given a JournalError::SequenceOverflow
        // When displayed
        // Then the message mentions overflow
        let err = JournalError::SequenceOverflow;
        let msg = format!("{}", err);
        assert!(msg.contains("overflow"));
    }

    #[test]
    fn journal_error_postcard_decode_failed_display() {
        // Given a JournalError::PostcardDecodeFailed
        // When displayed
        // Then the message mentions postcard
        let err = JournalError::PostcardDecodeFailed(postcard::Error::DeserializeBadVarint);
        let msg = format!("{}", err);
        assert!(msg.contains("postcard"));
    }

    #[test]
    fn journal_error_unexpected_eof_display() {
        // Given a JournalError::UnexpectedEof
        // When displayed
        // Then the message mentions end of record
        let err = JournalError::UnexpectedEof;
        let msg = format!("{}", err);
        assert!(msg.contains("end"));
    }

    #[test]
    fn journal_error_payload_digest_mismatch_display() {
        // Given a JournalError::PayloadDigestMismatch
        // When displayed
        // Then the message mentions digest
        let err = JournalError::PayloadDigestMismatch;
        let msg = format!("{}", err);
        assert!(msg.contains("digest"));
    }

    #[test]
    fn record_envelope_fields_match_encoded_values() {
        // Given an encoded event
        // When decoded
        // Then the envelope contains magic, schema_version, record_kind, and sequence
        let event = JournalEvent::RunAccepted {
            run: RunId::new(77),
            seq: EventSeq::new(3),
            workflow: test_digest(5),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 3, &event, 128)
            .expect("encoding should succeed");
        let (envelope, _) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(envelope.sequence, 3);
    }
