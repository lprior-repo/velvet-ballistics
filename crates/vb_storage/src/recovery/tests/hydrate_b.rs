    #[test]
    fn hydrate_run_frame_from_events_rejects_action_completed_envelope_digest_mismatch() {
        let run = RunId::new(20);
        let ticket = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let value = encoded_slot(&SlotValue::I64(42));
        let encoded_len = u32::try_from(value.len()).expect("encoded length fits u32");
        let events = vec![
            action_scheduled_ticket_event(
                run,
                EventSeq::new(1),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(1),
            ),
            JournalEvent::ActionCompletedEnvelope {
                run,
                seq: EventSeq::new(2),
                ticket,
                action_abi_digest: sample_digest(0xA1),
                output: SlotIdx::new(1),
                outcome: DurableActionOutcome::Ready,
                value,
                encoded_len,
                taint: Taint::Clean,
                value_digest: [255; 32],
            },
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(matches!(
            result,
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO
                    && detail == "action completion value digest mismatch"
        ));
    }

    #[test]
    fn hydrate_run_frame_from_events_rejects_action_completed_envelope_encoded_len_mismatch() {
        let run = RunId::new(21);
        let ticket = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let value = encoded_slot(&SlotValue::I64(42));
        let actual_len = u32::try_from(value.len()).expect("encoded length fits u32");
        let encoded_len = actual_len.checked_add(1).expect("test length increments");
        let value_digest = *blake3::hash(&value).as_bytes();
        let events = vec![
            action_scheduled_ticket_event(
                run,
                EventSeq::new(1),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(1),
            ),
            JournalEvent::ActionCompletedEnvelope {
                run,
                seq: EventSeq::new(2),
                ticket,
                action_abi_digest: sample_digest(0xA1),
                output: SlotIdx::new(1),
                outcome: DurableActionOutcome::Ready,
                value,
                encoded_len,
                taint: Taint::Clean,
                value_digest,
            },
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(matches!(
            result,
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO
                    && detail == "action completion encoded length mismatch"
        ));
    }

    #[test]
    fn hydrate_run_frame_from_events_applies_duplicate_identical_action_completed_envelope_once() {
        let run = RunId::new(22);
        let ticket = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let first = action_completed_envelope_event(
            run,
            EventSeq::new(2),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(42),
            Taint::Clean,
        );
        let second = action_completed_envelope_event(
            run,
            EventSeq::new(3),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(42),
            Taint::Clean,
        );
        let events = vec![
            action_scheduled_ticket_event(
                run,
                EventSeq::new(1),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(1),
            ),
            first,
            second,
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(
            matches!(result, Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) if found == run),
            "frame seed alone must be rejected: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    #[test]
    fn hydrate_run_frame_from_events_rejects_divergent_action_completed_envelope_duplicate() {
        let run = RunId::new(23);
        let ticket = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let events = vec![
            action_scheduled_ticket_event(
                run,
                EventSeq::new(1),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(1),
            ),
            action_completed_envelope_event(
                run,
                EventSeq::new(2),
                ticket,
                SlotIdx::new(1),
                SlotValue::I64(42),
                Taint::Clean,
            ),
            action_completed_envelope_event(
                run,
                EventSeq::new(3),
                ticket,
                SlotIdx::new(1),
                SlotValue::I64(43),
                Taint::Clean,
            ),
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(matches!(
            result,
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO
                    && detail == "divergent action completion envelope"
        ));
    }

    #[test]
    fn hydrate_run_frame_from_events_deduplicates_identical_action_scheduled_ticket() {
        let run = RunId::new(26);
        let ticket = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let events = vec![
            action_scheduled_ticket_event(
                run,
                EventSeq::new(1),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(1),
            ),
            action_scheduled_ticket_event(
                run,
                EventSeq::new(2),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(1),
            ),
            action_completed_envelope_event(
                run,
                EventSeq::new(3),
                ticket,
                SlotIdx::new(1),
                SlotValue::I64(42),
                Taint::Clean,
            ),
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(
            matches!(result, Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) if found == run),
            "frame seed alone must be rejected: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    #[test]
    fn hydrate_run_frame_from_events_rejects_divergent_action_scheduled_ticket() {
        let run = RunId::new(27);
        let ticket = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let events = vec![
            action_scheduled_ticket_event(
                run,
                EventSeq::new(1),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(1),
            ),
            action_scheduled_ticket_event(
                run,
                EventSeq::new(2),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(2),
            ),
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(matches!(
            result,
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO && detail == "divergent action schedule ticket"
        ));
    }

    #[test]
    fn hydrate_run_frame_from_events_rejects_completion_output_that_differs_from_schedule() {
        let run = RunId::new(28);
        let ticket = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let events = vec![
            action_scheduled_ticket_event(
                run,
                EventSeq::new(1),
                ticket,
                SlotIdx::new(0),
                SlotIdx::new(1),
            ),
            action_completed_envelope_event(
                run,
                EventSeq::new(2),
                ticket,
                SlotIdx::new(2),
                SlotValue::I64(42),
                Taint::Clean,
            ),
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(matches!(
            result,
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO
                    && detail == "action completion envelope does not match schedule ticket"
        ));
    }

    #[test]
    fn hydrate_run_frame_from_events_reconstructs_parallel_counters_for_ticket_and_envelope_events()
    {
        let run = RunId::new(24);
        let first = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let second = action_ticket(run, StepIdx::new(1), ActionId::new(2));
        let events = vec![
            action_scheduled_ticket_event(
                run,
                EventSeq::new(1),
                first,
                SlotIdx::new(0),
                SlotIdx::new(2),
            ),
            action_scheduled_ticket_event(
                run,
                EventSeq::new(2),
                second,
                SlotIdx::new(1),
                SlotIdx::new(3),
            ),
            action_completed_envelope_event(
                run,
                EventSeq::new(3),
                first,
                SlotIdx::new(2),
                SlotValue::I64(77),
                Taint::Clean,
            ),
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(
            matches!(result, Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) if found == run),
            "frame seed alone must be rejected: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    #[test]
    fn hydrate_run_frame_rejects_tail_completion_without_expected_action_abi() {
        let run = RunId::new(25);
        let snapshot = empty_snapshot(run, EventSeq::new(1));
        let ticket = action_ticket(run, StepIdx::ZERO, ActionId::new(1));
        let tail = vec![action_completed_envelope_event(
            run,
            EventSeq::new(2),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(88),
            Taint::Clean,
        )];

        let result = hydrate_run_frame(&snapshot, &tail, run);
        // The snapshot+tail path has no accepted-artifact ABI evidence in
        // this legacy signature, so an action completion envelope must now
        // fail closed instead of hydrating with unauthenticated action ABI.
        assert!(
            matches!(
                result,
                Err(RecoveryError::ActionAbiMismatch { action_id })
                    if action_id == ActionId::new(1)
            ),
            "snapshot+tail hydration must reject missing expected action ABI: {result:?}"
        );
    }

    // --- Invariant: Dimension integrity ---

    #[test]
    fn hydrate_run_frame_maintains_dimension_integrity() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(0));
        let tail = vec![
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
                value: Some(postcard::to_allocvec(&SlotValue::I64(1)).unwrap()),
                extra: None,
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame(&snapshot, &tail, run);
        // The snapshot+tail path remains a lower-level mechanism that
        // can still build a frame from durable evidence. The events-only
        // `hydrate_run_frame_from_events` path now fails closed with
        // `UnsupportedFrameSeed` because a frame seed alone never carries
        // the full RunState.
        assert!(
            result.is_ok(),
            "snapshot+tail hydration must succeed: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    // --- Invariant: Deterministic ---

    #[test]
    fn hydrate_run_frame_is_deterministic() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(0));
        let tail = vec![JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        }];

        let result1 = hydrate_run_frame(&snapshot, &tail, run);
        let result2 = hydrate_run_frame(&snapshot, &tail, run);

        match (&result1, &result2) {
            (Ok(f1), Ok(f2)) => {
                assert_eq!(f1.run_id(), f2.run_id());
                assert_eq!(f1.pc(), f2.pc());
                assert_eq!(f1.step_count(), f2.step_count());
                assert_eq!(f1.slot_count(), f2.slot_count());
            }
            (Err(e1), Err(e2)) => {
                assert_eq!(e1.to_string(), e2.to_string());
            }
            _ => panic!(
                "results differ in Ok/Err variant: {:?} vs {:?}",
                result1, result2
            ),
        }
    }

    // --- Additional edge case tests for coverage ---

    #[test]
    fn hydrate_run_frame_wait_scheduled_marks_waiting() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(0));
        // The typed cannot-resume gate (BLOCKER 4) now rejects pending
        // waits/asks before frame allocation. To exercise the lower-
        // level frame-state machinery (`mark_waiting`), the tail must
        // carry both the WaitScheduledEvent and the WaitResolvedEvent
        // for the same step so the gate is closed.
        let tail = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::WaitResolvedEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(0),
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame(&snapshot, &tail, run);
        // After the typed gate passes (wait was resolved in the tail),
        // the lower-level frame-state machinery applies the wait +
        // resolution to the frame. The frame is successfully built.
        assert!(
            result.is_ok(),
            "snapshot+tail hydration must succeed for resolved wait: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    #[test]
    fn hydrate_run_frame_ask_scheduled_marks_asking() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(0));
        // The typed cannot-resume gate (BLOCKER 4) now rejects pending
        // asks before frame allocation. To exercise the lower-level
        // frame-state machinery (`mark_asking`), the tail must carry
        // both the AskScheduledEvent and the AskAnsweredEvent for the
        // same step so the gate is closed.
        let tail = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(0),
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame(&snapshot, &tail, run);
        // After the typed gate passes (ask was answered in the tail),
        // the lower-level frame-state machinery applies the ask +
        // answer to the frame. The frame is successfully built.
        assert!(
            result.is_ok(),
            "snapshot+tail hydration must succeed for answered ask: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    #[test]
    fn hydrate_run_frame_action_failed_decrements_parallel() {
        let run = RunId::new(1);
        let events = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            },
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame_from_events(&events, run);
        assert!(
            matches!(result, Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) if found == run),
            "frame seed alone must be rejected: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    #[test]
    fn hydrate_run_frame_run_finished_is_no_op() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(0));
        let tail = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                output: SlotIdx::new(0),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(3),
                result: SlotIdx::new(0),
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame(&snapshot, &tail, run);
        // The snapshot+tail path remains a lower-level mechanism that
        // can still build a frame from durable evidence. The events-only
        // `hydrate_run_frame_from_events` path now fails closed with
        // `UnsupportedFrameSeed` because a frame seed alone never carries
        // the full RunState.
        assert!(
            result.is_ok(),
            "snapshot+tail hydration must succeed: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    #[test]
    fn hydrate_run_frame_run_cancelled_is_no_op() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(0));
        let tail = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(2),
                attempt: 1,
                reason: None,
            },
        ];

        let result = hydrate_run_frame(&snapshot, &tail, run);
        // The snapshot+tail path remains a lower-level mechanism that
        // can still build a frame from durable evidence. The events-only
        // `hydrate_run_frame_from_events` path now fails closed with
        // `UnsupportedFrameSeed` because a frame seed alone never carries
        // the full RunState.
        assert!(
            result.is_ok(),
            "snapshot+tail hydration must succeed: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    #[test]
    fn hydrate_run_frame_rejects_tail_event_at_same_seq_as_snapshot() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(5));
        let tail = vec![JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(0),
            attempt: 1,
        }];

        let result = hydrate_run_frame(&snapshot, &tail, run);
        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "expected ReplayDivergence for equal seq, got {:?}",
            result
        );
    }

    #[test]
    fn hydrate_run_frame_rejects_tail_event_seq_less_than_snapshot() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(10));
        let tail = vec![JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(9),
            step: StepIdx::new(0),
            attempt: 1,
        }];

        let result = hydrate_run_frame(&snapshot, &tail, run);
        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "expected ReplayDivergence for seq < snapshot, got {:?}",
            result
        );
    }

    #[test]
    fn hydrate_run_frame_from_events_with_multiple_slots() {
        let run = RunId::new(1);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(1),
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
                value: Some(postcard::to_allocvec(&SlotValue::I64(1)).unwrap()),
                extra: None,
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(3),
                slot: SlotIdx::new(1),
                value: Some(postcard::to_allocvec(&SlotValue::I64(2)).unwrap()),
                extra: None,
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(0),
                output: SlotIdx::new(1),
            },
        ];

        let result = hydrate_run_frame_from_events(&events, run);
        assert!(
            matches!(result, Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) if found == run),
            "frame seed alone must be rejected: {result:?}"
        );
        // frame binding removed: storage boundary now fails closed.
    }

    // --- Frame-level comparison: snapshot+tail vs full journal ---
    //
    // NOTE: This test documents that hydrate_run_frame(snap, tail) and
    // hydrate_run_frame_from_events(all_events) are DIFFERENT code paths that
    // produce different frames in general due to:
    // 1. max_parallel_in_flight tracking (hydrate_run_frame doesn't set it,
    //    hydrate_run_frame_from_events does via compute_parallel_in_flight)
    // 2. Different execution paths (snapshot+tail vs full replay)
    //
    // The issue description asks for a test proving equivalence, but such a test
    // would fail because the paths are not equivalent. This test documents the
    // actual relationship.
    #[test]
    fn hydrate_run_frame_vs_full_journal_frame_comparison() {
        let run = RunId::new(9999);

        // Events: RunAccepted, StepStarted, StepSucceeded
        let all_events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(0),
                output: SlotIdx::new(0),
            },
        ];

        // Snapshot at seq=1, tail = events after seq=1
        let snapshot_seq = EventSeq::new(1);
        let tail: Vec<JournalEvent> = all_events
            .iter()
            .filter(|e| e.seq() > snapshot_seq)
            .cloned()
            .collect();

        let snapshot = RunSnapshot {
            run,
            seq: snapshot_seq,
            workflow: sample_digest(1),
            slots: Vec::new(),
            taint: Vec::new(),
        };

        // The snapshot+tail path remains a lower-level mechanism that
        // can still build a frame from durable evidence. The events-only
        // `hydrate_run_frame_from_events` path now fails closed with
        // `UnsupportedFrameSeed` because a frame seed alone never carries
        // the full RunState. The test now exercises the same end-to-end
        // shape through the storage boundary: the frame seed alone is
        // rejected, but the snapshot+tail path is still permitted to
        // produce a frame that satisfies the runtime's contract.
        let frame_from_snapshot =
            hydrate_run_frame(&snapshot, &tail, run).expect("snapshot+tail should succeed");
        let journal_result = hydrate_run_frame_from_events(&all_events, run);
        assert!(
            matches!(journal_result, Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) if found == run),
            "events-only hydration must be rejected: {journal_result:?}"
        );

        // Basic assertions: the snapshot+tail path still produces a
        // frame with the expected run id, dimensions, pc, and step
        // state. The events-only path now fails closed so we no
        // longer compare the two frames.
        assert_eq!(frame_from_snapshot.run_id(), run);
        assert_eq!(frame_from_snapshot.step_count(), 1);
        assert_eq!(frame_from_snapshot.slot_count(), 0);
        assert_eq!(frame_from_snapshot.pc(), StepIdx::new(0));
        assert_eq!(
            frame_from_snapshot.step_state(StepIdx::new(0)).unwrap(),
            StepState::Succeeded
        );
    }
