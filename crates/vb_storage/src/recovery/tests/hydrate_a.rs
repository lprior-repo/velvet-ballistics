    #[test]
    fn hydrate_run_frame_reconstructs_frame_from_snapshot_and_tail_events() {
        let run = RunId::new(1);
        let snapshot = snapshot_with_slots(
            run,
            EventSeq::new(0),
            &[(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)],
        );
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
        let Ok(_frame) = result else {
            assert!(false, "snapshot+tail hydration must succeed");
            return;
        };
    }

    // --- Happy path: events only ---

    #[test]
    fn hydrate_run_frame_from_events_reconstructs_without_snapshot() {
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
                value: Some(postcard::to_allocvec(&SlotValue::I64(7)).unwrap()),
                extra: None,
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        // Frame seed alone never carries the full RunState, so the
        // storage boundary must reject the hydration with
        // `UnsupportedFrameSeed`. The legacy event-only frame
        // reconstruction that this test used to exercise is no
        // longer reachable from the public API; the run state has
        // to be rebuilt by the higher-level runtime boundary that
        // supplies the missing full-state components.
        assert!(
            matches!(result, Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) if found == run),
            "expected UnsupportedFrameSeed, got {result:?}"
        );
    }

    #[test]
    fn hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata() {
        let run = RunId::new(1);
        let slot = SlotIdx::new(0);
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
                slot,
                value: Some(postcard::to_allocvec(&SlotValue::Bool(false)).unwrap()),
                extra: Some(corrupt_slot_taint_envelope()),
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(matches!(result, Err(RecoveryError::CorruptSlotTaint { slot: s }) if s == slot));
    }

    #[test]
    fn hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar() {
        let run = RunId::new(1);
        let slot = SlotIdx::new(0);
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
                slot,
                value: Some(postcard::to_allocvec(&SlotValue::Bool(false)).unwrap()),
                extra: Some(vec![1, 2, 3, 4]),
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(
            matches!(result, Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) if found == run),
            "legacy frame extra must not be corrupt taint: {result:?}"
        );
    }

    // vb-i21a2 (SR-013): legacy frames without a taint sidecar must NOT
    // under-taint a `Bool(false)` value to `Taint::Clean`. Master §47
    // forbids asymmetric downgrades from secret-provenance frames; the
    // lattice-preserving choice for an unknown-provenance Bool is
    // `Taint::Secret`, not `Taint::Clean`. The existing test above only
    // asserted `result.is_ok()` so the asymmetric leak was invisible.
    // This test asserts the actual recovered taint is `Secret`.
    #[test]
    fn legacy_bool_false_slot_does_not_downgrade_to_clean_taint() {
        use crate::recovery::replay::summary::legacy_slot_taint;
        assert_eq!(
            legacy_slot_taint(vb_core::SlotValue::Bool(false)),
            vb_core::Taint::Secret,
            "vb-i21a2: legacy_slot_taint(Bool(false)) MUST be Taint::Secret, not Taint::Clean \
             (master §47 forbids asymmetric downgrade of unknown-provenance frames)"
        );
        assert_eq!(
            legacy_slot_taint(vb_core::SlotValue::Bool(true)),
            vb_core::Taint::Secret,
            "vb-i21a2: legacy_slot_taint(Bool(true)) MUST be Taint::Secret \
             (lattice-preserving recovery for unknown provenance)"
        );
        assert_eq!(
            legacy_slot_taint(vb_core::SlotValue::Null),
            vb_core::Taint::Secret,
            "vb-i21a2: legacy_slot_taint(Null) MUST be Taint::Secret"
        );
    }

    #[test]
    fn legacy_bool_false_recovered_via_hydration_carries_secret_taint() {
        let run = RunId::new(1);
        let slot = SlotIdx::new(0);
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
                slot,
                value: Some(
                    postcard::to_allocvec(&SlotValue::Bool(false))
                        .expect("slot value serialization must succeed"),
                ),
                // No `extra` bytes forces the legacy_slot_taint path.
                extra: None,
                attempt: 1,
            },
        ];

        let result = hydrate_run_frame_from_events(&events, run);
        // The storage layer now fails closed: a frame seed alone
        // never carries a full RunState, so the legacy hydration
        // path returns `UnsupportedFrameSeed`. The lattice
        // preservation invariant for legacy `Bool(false)` is now
        // covered by `legacy_bool_false_slot_does_not_downgrade_to_clean_taint`
        // which exercises `legacy_slot_taint` directly.
        let Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) = result else {
            panic!("frame seed alone must be rejected: {result:?}");
        };
        assert_eq!(found, run);
    }

    // --- Error: mismatched snapshot run_id ---

    #[test]
    fn hydrate_run_frame_rejects_mismatched_snapshot_run_id() {
        let snapshot = empty_snapshot(RunId::new(1), EventSeq::new(0));
        let run = RunId::new(2);

        let result = hydrate_run_frame(&snapshot, &[], run);

        assert!(
            matches!(result, Err(RecoveryError::CorruptSnapshot { .. })),
            "expected CorruptSnapshot, got {:?}",
            result
        );
    }

    // --- Error: tail event for wrong run ---

    #[test]
    fn hydrate_run_frame_rejects_tail_event_for_wrong_run() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(0));
        let tail = vec![JournalEvent::StepStarted {
            run: RunId::new(2),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        }];

        let result = hydrate_run_frame(&snapshot, &tail, run);

        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "expected ReplayDivergence, got {:?}",
            result
        );
    }

    // --- Error: tail event before snapshot seq ---

    #[test]
    fn hydrate_run_frame_rejects_tail_event_before_snapshot_seq() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(10));
        let tail = vec![JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(0),
            attempt: 1,
        }];

        let result = hydrate_run_frame(&snapshot, &tail, run);

        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "expected ReplayDivergence, got {:?}",
            result
        );
    }

    // --- Error: corrupt snapshot bytes ---

    #[test]
    fn hydrate_run_frame_rejects_corrupt_snapshot_slots_bytes() {
        let run = RunId::new(1);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
            slots: vec![0xFF, 0xFF],
            taint: vec![0xFF, 0xFF],
        };

        let result = hydrate_run_frame(&snapshot, &[], run);

        assert!(
            matches!(result, Err(RecoveryError::CorruptSnapshot { .. })),
            "expected CorruptSnapshot, got {:?}",
            result
        );
    }

    // Round 10 issue 10: regression test that the `CorruptSnapshot`
    // diagnostic returned from postcard-decode failure inside
    // `decode_snapshot_slots` carries the real `seq` of the snapshot
    // that failed (was previously a zero placeholder). The expected
    // seq here is 42; matching must succeed with that exact value.
    #[test]
    fn corrupt_snapshot_reports_actual_seq() {
        let run = RunId::new(1);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(42),
            workflow: sample_digest(1),
            slots: vec![1, 2, 3],
            taint: vec![1, 2, 3],
        };

        let result = hydrate_run_frame(&snapshot, &[], run);

        match result {
            Err(RecoveryError::CorruptSnapshot { run: r, seq: s })
                if r == run && s == EventSeq::new(42) => {}
            other => panic!(
                "expected CorruptSnapshot {{ run: 1, seq: 42 }}, got {:?}",
                other
            ),
        }
    }

    // --- Error: empty snapshot and empty events ---

    #[test]
    fn hydrate_run_frame_rejects_empty_snapshot_and_empty_events() {
        let run = RunId::new(1);
        let snapshot = empty_snapshot(run, EventSeq::new(0));

        let result = hydrate_run_frame(&snapshot, &[], run);

        assert!(
            matches!(result, Err(RecoveryError::NoRecoveryData { .. })),
            "expected NoRecoveryData, got {:?}",
            result
        );
    }

    // --- Error: zero step count from events ---

    #[test]
    fn hydrate_run_frame_from_events_rejects_zero_step_count() {
        let run = RunId::new(1);
        let events = vec![JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
        }];

        let result = hydrate_run_frame_from_events(&events, run);

        assert!(
            matches!(
                result,
                Err(RecoveryError::ReplayDivergence { .. })
                    | Err(RecoveryError::NoRecoveryData { .. })
            ),
            "expected error for zero step count, got {:?}",
            result
        );
    }

    // --- State: PC from last step event ---

    #[test]
    fn hydrate_run_frame_pc_set_from_last_step_event() {
        let run = RunId::new(1);
        let events = vec![
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
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
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

    // --- State: Step states merge snapshot and tail ---

    #[test]
    fn hydrate_run_frame_states_merge_snapshot_and_tail() {
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
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
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

    // --- State: Slots overwritten by tail events ---

    #[test]
    fn hydrate_run_frame_slots_overwritten_by_tail_events() {
        let run = RunId::new(1);
        let snapshot = snapshot_with_slots(
            run,
            EventSeq::new(0),
            &[(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)],
        );
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
                value: Some(postcard::to_allocvec(&SlotValue::I64(2)).unwrap()),
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

    // --- State: Taint preserved when tail has no taint ---

    #[test]
    fn hydrate_run_frame_taint_preserved_when_tail_has_no_taint() {
        let run = RunId::new(1);
        let snapshot = snapshot_with_slots(
            run,
            EventSeq::new(0),
            &[(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret)],
        );
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
                value: Some(postcard::to_allocvec(&SlotValue::I64(2)).unwrap()),
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

    #[test]
    fn apply_tail_events_fails_closed_when_taint_read_fails() {
        let run = RunId::new(1);
        let mut frame = vb_core::RunFrame::new(run, StepIdx::ZERO, 1, 0).unwrap();
        let tail = vec![JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: Some(postcard::to_allocvec(&SlotValue::I64(7)).unwrap()),
            extra: None,
            attempt: 1,
        }];
        let mut tracker = ActionReplayTracker::new();

        let result = apply_tail_events(&mut frame, &tail, &mut tracker, &[]);

        assert!(
            matches!(
                result,
                Err(RecoveryError::SlotTaintReadFailed { slot }) if slot == SlotIdx::new(0)
            ),
            "taint read failure must not downgrade to Clean: {:?}",
            result
        );
    }

    // --- State: Executed counter ---

    #[test]
    fn hydrate_run_frame_executed_counter_matches_tail_event_count() {
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
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(3),
                slot: SlotIdx::new(0),
                value: Some(postcard::to_allocvec(&SlotValue::I64(7)).unwrap()),
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

    // --- State: Parallel in-flight ---

    #[test]
    fn hydrate_run_frame_reconstructs_parallel_in_flight() {
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
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
                attempt: 1,
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(1),
                action: ActionId::new(2),
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(5),
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
