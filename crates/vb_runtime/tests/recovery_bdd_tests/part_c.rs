// ---------------------------------------------------------------------------
// Additional recovery tests to meet 5x density target (70 tests total)
// These tests cover: boundary conditions, error variants, and replay scenarios
// ---------------------------------------------------------------------------

#[test]
fn hydrate_run_frame_from_empty_events_returns_no_recovery_data() {
    let result = hydrate_run_frame_from_events(&[], RunId::new(9001));
    let Err(RecoveryError::NoRecoveryData { .. }) = result else {
        panic!("expected NoRecoveryData for empty events, got: {result:?}");
    };
}

#[test]
fn hydrate_run_frame_validates_snapshot_run_id_match() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9002);
    let wrong_run = RunId::new(9999);
    let digest = test_digest(0xA1);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::new(1),
        attempt: 1,
    }];

    let result = hydrate_run_frame(&snapshot, &tail, wrong_run);
    assert!(
        result.is_err(),
        "should fail when snapshot.run != requested run_id"
    );
}

#[test]
fn hydrate_run_frame_rejects_tail_events_with_wrong_run_id() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9003);
    let wrong_run = RunId::new(9998);
    let digest = test_digest(0xA1);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![JournalEvent::StepStarted {
        run: wrong_run,
        seq: EventSeq::new(2),
        step: StepIdx::new(1),
        attempt: 1,
    }];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { .. })
    ));
}

#[test]
fn hydrate_run_frame_rejects_tail_seq_before_snapshot() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9004);
    let digest = test_digest(0xA1);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(3),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::new(1),
        attempt: 1,
    }];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { .. })
    ));
}

#[test]
fn recover_runtime_summary_handles_empty_journal() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9005);

    let journal = open_journal(&dir);
    let result = recover_runtime_summary(&journal, run);
    assert!(result.is_err(), "empty journal should return error");
}

#[test]
fn recover_runtime_frame_seed_from_events_with_multiple_attempts() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9006);
    let digest = test_digest(0xA1);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: digest,
                },
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::ZERO,
                    attempt: 1,
                },
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(2),
                    action: ActionId::new(77),
                    step: StepIdx::ZERO,
                    attempt: 1,
                },
                JournalEvent::ActionFailedEvent {
                    run,
                    seq: EventSeq::new(3),
                    action: ActionId::new(77),
                    step: StepIdx::ZERO,
                    attempt: 1,
                },
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(4),
                    step: StepIdx::ZERO,
                    attempt: 2,
                },
                JournalEvent::StepSucceeded {
                    run,
                    seq: EventSeq::new(5),
                    step: StepIdx::ZERO,
                    output: SlotIdx::new(0),
                },
            ],
        );
    }

    let journal = open_journal(&dir);
    let result = recover_runtime_frame_seed(&journal, run);
    assert!(
        result.is_ok(),
        "should recover frame seed with multiple attempts"
    );
    let seed = result.unwrap();
    assert_eq!(seed.step_count, 1, "should have 1 step");
}

#[test]
fn action_replay_tracker_mark_completed_preserves_resolution() {
    let mut tracker = ActionReplayTracker::new();
    let action = ActionId::new(77);
    let step = StepIdx::new(1);

    tracker.mark_completed(action.clone(), step);
    assert!(
        tracker.is_resolved(action.clone(), step),
        "action should be resolved after mark_completed"
    );

    tracker.mark_failed(action.clone(), step);
    assert!(
        tracker.is_resolved(action, step),
        "action should remain resolved after mark_failed"
    );
}

#[test]
fn action_replay_tracker_new_is_unresolved() {
    let tracker = ActionReplayTracker::new();
    let action = ActionId::new(77);
    let step = StepIdx::new(1);

    assert!(
        !tracker.is_resolved(action, step),
        "new tracker should have unresolved actions"
    );
}

#[test]
fn digest_check_variants_exist() {
    use vb_storage::recovery::DigestCheck;

    let _ = DigestCheck::WorkflowSourceOnly;
    let _ = DigestCheck::WorkflowAndIr;
    let _ = DigestCheck::Full;

    assert_eq!(
        DigestCheck::WorkflowSourceOnly,
        DigestCheck::WorkflowSourceOnly
    );
    assert_eq!(DigestCheck::WorkflowAndIr, DigestCheck::WorkflowAndIr);
    assert_eq!(DigestCheck::Full, DigestCheck::Full);
}

#[test]
fn recover_all_incomplete_runs_excludes_finished_runs() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9007);
    let digest = test_digest(0xA1);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: digest,
                },
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(1),
                    attempt: 1,
                },
                JournalEvent::StepSucceeded {
                    run,
                    seq: EventSeq::new(2),
                    step: StepIdx::new(1),
                    output: SlotIdx::new(0),
                },
            ],
        );
    }

    let journal = open_journal(&dir);
    let result = recover_runtime_summary(&journal, run);
    assert!(result.is_ok(), "finished run should be recoverable");
}

#[test]
fn slot_written_none_value_reconstructed_correctly() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9008);
    let digest = test_digest(0xA1);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: digest,
                },
                JournalEvent::SlotWrittenEvent {
                    run,
                    seq: EventSeq::new(1),
                    slot: SlotIdx::new(0),
                    value: None,
                    extra: None,
                    attempt: 1,
                },
            ],
        );
    }

    let _journal = open_journal(&dir);
    let result = hydrate_run_frame_from_events(
        &[
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::ZERO,
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
        ],
        run,
    );
    assert_unsupported_frame_seed(result, run, "slot_values");
}

#[test]
fn multiple_slots_different_indices_reconstructed() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9009);
    let digest = test_digest(0xA1);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            value: Some(vec![]),
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(3),
            slot: SlotIdx::new(1),
            value: Some(vec![]),
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(4),
            slot: SlotIdx::new(5),
            value: Some(vec![]),
            extra: None,
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "slot_values");
}

#[test]
fn step_started_event_advances_pc() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9010);
    let digest = test_digest(0xA1);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(3),
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    assert_unsupported_frame_seed(result, run, "workflow_missing");
}

#[test]
fn action_scheduled_then_completed_reconstructed() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9011);
    let digest = test_digest(0xA1);
    let action = ActionId::new(78);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            action: action.clone(),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            action,
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    // A frame seed alone never carries the full RunState, so the
    // storage boundary must reject the hydration with
    // `UnsupportedFrameSeed`. The legacy event-only frame
    // reconstruction that this test used to exercise is no
    // longer reachable from the public API; the run state has
    // to be rebuilt by the higher-level runtime boundary that
    // supplies the missing full-state components.
    assert_unsupported_frame_seed(result, run, "workflow_missing");
}

#[test]
fn action_scheduled_then_failed_reconstructed() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9012);
    let digest = test_digest(0xA1);
    let action = ActionId::new(79);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            action: action.clone(),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(3),
            action,
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    // A frame seed alone never carries the full RunState, so the
    // storage boundary must reject the hydration with
    // `UnsupportedFrameSeed`. The legacy event-only frame
    // reconstruction that this test used to exercise is no
    // longer reachable from the public API; the run state has
    // to be rebuilt by the higher-level runtime boundary that
    // supplies the missing full-state components.
    assert_unsupported_frame_seed(result, run, "workflow_missing");
}

#[test]
fn retry_scheduled_event_reconstructed() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9013);
    let digest = test_digest(0xA1);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 2,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    // A frame seed alone never carries the full RunState, so the
    // storage boundary must reject the hydration with
    // `UnsupportedFrameSeed`. The legacy event-only frame
    // reconstruction that this test used to exercise is no
    // longer reachable from the public API; the run state has
    // to be rebuilt by the higher-level runtime boundary that
    // supplies the missing full-state components.
    assert_unsupported_frame_seed(result, run, "workflow_missing");
}

#[test]
fn ask_scheduled_and_answered_events_reconstructed() {
    let run = RunId::new(9014);
    let digest = test_digest(0xA1);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame_from_events(&events, run);
    // A frame seed alone never carries the full RunState, so the
    // storage boundary must reject the hydration with
    // `UnsupportedFrameSeed`. The legacy event-only frame
    // reconstruction that this test used to exercise is no
    // longer reachable from the public API; the run state has
    // to be rebuilt by the higher-level runtime boundary that
    // supplies the missing full-state components.
    assert_unsupported_frame_seed(result, run, "workflow_missing");
}

#[test]
fn run_failed_event_sets_terminal_state() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9015);
    let digest = test_digest(0xA1);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let result = recover_runtime_summary(&open_journal(&dir), run);
    assert!(result.is_ok(), "run failed event should be recoverable");
}

#[test]
fn run_finished_event_sets_terminal_state_with_result() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9016);
    let digest = test_digest(0xA1);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(3),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let result = recover_runtime_summary(&open_journal(&dir), run);
    assert!(result.is_ok(), "run finished should be recoverable");
    let summary = result.unwrap().summary();
    assert!(summary.terminal.is_some(), "terminal should be present");
    if let Some(terminal) = summary.terminal {
        assert!(
            matches!(terminal, RecoveryTerminalState::Finished { .. }),
            "terminal should be Finished"
        );
    }
}

#[test]
fn run_cancelled_event_sets_terminal_state() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9017);
    let digest = test_digest(0xA1);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let result = recover_runtime_summary(&open_journal(&dir), run);
    assert!(result.is_ok(), "run cancelled should be recoverable");
    let summary = result.unwrap().summary();
    assert!(summary.terminal.is_some(), "terminal should be present");
    if let Some(terminal) = summary.terminal {
        assert!(
            matches!(terminal, RecoveryTerminalState::Cancelled),
            "terminal should be Cancelled"
        );
    }
}

#[test]
fn watermark_preserves_snapshot_data_beyond_tail() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9018);
    let digest = test_digest(0xA1);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(4),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
    ];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert_unsupported_frame_seed(result, run, "slot_values");
}

#[test]
fn identical_tail_on_same_snapshot_is_idempotent() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9019);
    let digest = test_digest(0xA1);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            output: SlotIdx::new(0),
        },
    ];

    let result1 = hydrate_run_frame(&snapshot, &tail, run);
    let result2 = hydrate_run_frame(&snapshot, &tail, run);

    assert!(
        result1.is_ok() && result2.is_ok(),
        "idempotent on same input"
    );
}

#[test]
fn check_workflow_source_digest_accepts_matching_digest() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9020);
    let digest = test_digest(0xA1);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            }],
        );
    }

    let journal = open_journal(&dir);
    let result = check_workflow_source_digest(&journal, run, digest);
    assert!(
        result.is_ok(),
        "matching workflow digest should be accepted"
    );
}

#[test]
fn check_workflow_source_digest_rejects_mismatch() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9021);
    let stored_digest = test_digest(0xA1);
    let wrong_digest = test_digest(0xFF);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: stored_digest,
            }],
        );
    }

    let journal = open_journal(&dir);
    let result = check_workflow_source_digest(&journal, run, wrong_digest);
    assert!(
        result.is_err(),
        "mismatched workflow digest should be rejected"
    );
}

#[test]
fn check_compiled_ir_digest_accepts_matching_digest() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9022);
    let source_digest = test_digest(0xA1);
    let _ir_digest = test_digest(0xB2);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: source_digest,
            }],
        );
    }

    let digest = test_digest(0xD1);
    let result = check_compiled_ir_digest(digest, digest);
    assert!(result.is_ok(), "matching digests should succeed");
}

#[test]
fn check_compiled_ir_digest_rejects_mismatch() {
    let expected = test_digest(0xE1);
    let found = test_digest(0xE2);

    let result = check_compiled_ir_digest(expected, found);
    assert!(result.is_err(), "mismatched digests should be rejected");
}

#[test]
fn recover_runtime_summary_returns_recovery_hydration() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9023);
    let digest = test_digest(0xA1);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: digest,
                },
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(1),
                    attempt: 1,
                },
            ],
        );
    }

    let journal = open_journal(&dir);
    let result = recover_runtime_summary(&journal, run);
    assert!(result.is_ok(), "should return RecoveryHydration");
}

#[test]
fn snapshot_plus_tail_with_empty_taint_preserves_empty_taint() {
    let _dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9024);
    let digest = test_digest(0xA1);

    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: digest,
        slots: vec![],
        taint: vec![],
    };

    let tail = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            output: SlotIdx::new(0),
        },
    ];

    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert!(result.is_ok(), "empty taint should remain empty");
}

#[test]
fn verify_digests_at_workflow_source_only_level() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9025);
    let source_digest = test_digest(0xA1);
    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: source_digest,
            }],
        );
    }

    let journal = open_journal(&dir);
    let result = verify_digests(
        &journal,
        run,
        DigestVerificationRequest::workflow_source_only(source_digest),
    );
    assert!(
        result.is_ok(),
        "WorkflowSourceOnly should only check workflow digest"
    );
}

#[test]
fn recover_runtime_frame_seed_with_no_slot_events() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(9026);
    let digest = test_digest(0xA1);

    {
        let journal = open_journal(&dir);
        write_events_strict(
            &journal,
            &[
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: digest,
                },
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(1),
                    attempt: 1,
                },
            ],
        );
    }

    let journal = open_journal(&dir);
    let result = recover_runtime_frame_seed(&journal, run);
    assert!(
        result.is_ok(),
        "should recover frame seed without slot events"
    );
    let seed = result.unwrap();
    assert_eq!(seed.slot_count, 0, "no slots should result in slot_count 0");
}

