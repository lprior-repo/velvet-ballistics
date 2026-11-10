#[test]
fn snapshot_plus_tail_rejects_event_before_snapshot() {
    let snapshot = RunSnapshot {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        workflow: sample_digest(1),
        slots: Vec::new(),
        taint: Vec::new(),
    };
    let tail = vec![JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(3),
        step: StepIdx::new(0),
        output: SlotIdx::new(0),
    }];
    let mut tracker = ActionReplayTracker::new();

    let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker, &[]);
    let Err(err) = result else {
        panic!("tail event before snapshot should be rejected");
    };
    assert!(matches!(err, RecoveryError::ReplayDivergence { .. }));
}

#[test]
fn full_journal_recovery_with_no_data_fails() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let mut tracker = ActionReplayTracker::new();

    let result = recover_full_journal(&journal, RunId::new(999), &mut tracker, &[], &[]);
    let Err(err) = result else {
        panic!("empty journal should produce NoRecoveryData");
    };
    assert!(matches!(err, RecoveryError::NoRecoveryData { .. }));
}

#[test]
fn full_journal_recovery_replays_events() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(42);

    let accepted = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: sample_digest(1),
    };
    let started = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let finished = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(2),
        result: SlotIdx::new(0),
        attempt: 1,
    };

    journal
        .append_journaled(&accepted)
        .expect("setup: append accepted");
    journal
        .append_journaled(&started)
        .expect("setup: append started");
    journal
        .append_journaled(&finished)
        .expect("setup: append finished");

    let mut tracker = ActionReplayTracker::new();
    let replayed = recover_full_journal(&journal, run, &mut tracker, &[], &[])
        .expect("full journal recovery should succeed");
    assert_eq!(replayed.len(), 3);
}

#[test]
fn replay_all_event_kinds() {
    let run = RunId::new(7);
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
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(8),
            step: StepIdx::new(3),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(9),
            step: StepIdx::new(3),
            output: SlotIdx::new(1),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(10),
            result: SlotIdx::new(1),
            attempt: 1,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let replayed = replay_events(&events, &mut tracker, &[])
        .expect("replay of all event kinds should succeed");
    assert_eq!(replayed.len(), 11);
    assert!(tracker.is_resolved(ActionId::new(1), StepIdx::new(0)));
}

#[test]
fn snapshot_plus_tail_accepts_valid_tail_events() {
    let snapshot = RunSnapshot {
        run: RunId::new(10),
        seq: EventSeq::new(5),
        workflow: sample_digest(1),
        slots: Vec::new(),
        taint: Vec::new(),
    };
    let tail = vec![
        JournalEvent::StepStarted {
            run: RunId::new(10),
            seq: EventSeq::new(6),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run: RunId::new(10),
            seq: EventSeq::new(7),
            step: StepIdx::new(0),
            output: SlotIdx::new(1),
        },
    ];
    let mut tracker = ActionReplayTracker::new();

    let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker, &[])
        .expect("valid tail events should replay successfully");
    assert_eq!(replayed.len(), 2);
}

#[test]
fn replay_detects_out_of_order_step() {
    let run = RunId::new(20);
    let events = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let result = replay_events(&events, &mut tracker, &[]);
    let Err(err) = result else {
        panic!("out-of-order steps should cause divergence");
    };
    assert!(matches!(err, RecoveryError::ReplayDivergence { .. }));
}

// --- New Recovery Tests ---

#[test]
fn check_workflow_source_digest_returns_mismatch_when_digests_differ() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(100);
    let stored_digest = sample_digest(1);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: stored_digest,
    };
    journal
        .append_journaled(&event)
        .expect("setup: append event");

    let wrong_digest = sample_digest(2);
    let result = check_workflow_source_digest(&journal, run, wrong_digest);
    let Err(RecoveryError::WorkflowSourceDigestMismatch { expected, found }) = result else {
        panic!("expected WorkflowSourceDigestMismatch, got {:?}", result);
    };
    assert_eq!(expected, wrong_digest);
    assert_eq!(found, stored_digest);
}

#[test]
fn check_workflow_source_digest_succeeds_when_digests_match() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(101);
    let digest = sample_digest(5);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal
        .append_journaled(&event)
        .expect("setup: append event");

    check_workflow_source_digest(&journal, run, digest).expect("matching digest should succeed");
}

#[test]
fn check_compiled_ir_digest_returns_mismatch_when_digests_differ() {
    let expected = sample_digest(10);
    let found = sample_digest(20);
    let result = check_compiled_ir_digest(expected, found);
    let Err(RecoveryError::CompiledIrDigestMismatch {
        expected: exp,
        found: fnd,
    }) = result
    else {
        panic!("expected CompiledIrDigestMismatch, got {:?}", result);
    };
    assert_eq!(exp, expected);
    assert_eq!(fnd, found);
}

#[test]
fn verify_digests_returns_ok_when_all_match() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(200);
    let digest = sample_digest(7);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal
        .append_journaled(&event)
        .expect("setup: append event");

    verify_digests(
        &journal,
        run,
        DigestVerificationRequest::full(
            digest,
            sample_digest(8),
            sample_digest(8),
            FullDigestEvidence::no_contracts(),
        ),
    )
    .expect("matching digests at Full level should succeed");
}

#[test]
fn verify_digests_returns_mismatch_when_ir_differs() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(201);
    let digest = sample_digest(7);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: digest,
    };
    journal
        .append_journaled(&event)
        .expect("setup: append event");

    let result = verify_digests(
        &journal,
        run,
        DigestVerificationRequest::workflow_and_ir(digest, sample_digest(8), sample_digest(9)),
    );
    assert!(matches!(
        result,
        Err(RecoveryError::CompiledIrDigestMismatch { .. })
    ));
}

#[test]
fn verify_digests_full_rejects_mismatched_action_abi_digest() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(300);
    let source = sample_digest(7);
    let ir = sample_digest(8);

    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: source,
    };
    journal
        .append_journaled(&event)
        .expect("setup: append event");

    let action = ActionId::new(11);
    let expected_action_abi = sample_digest(11);
    let mismatched_action_abi = sample_digest(12);

    let action_evidence = [ActionAbiDigestComparison::new(
        action,
        expected_action_abi,
        mismatched_action_abi,
    )];
    let result = verify_digests(
        &journal,
        run,
        DigestVerificationRequest::full(
            source,
            ir,
            ir,
            FullDigestEvidence::action_abi_only(&action_evidence),
        ),
    );

    assert!(
        matches!(
            result,
            Err(RecoveryError::ActionAbiMismatch { action_id: a }) if a == action
        ),
        "verify_digests::Full must reject mismatched action-ABI digest with ActionAbiMismatch, got {result:?}"
    );
}

#[test]
fn verify_digests_full_rejects_mismatched_policy_digest() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(301);
    let source = sample_digest(7);
    let ir = sample_digest(8);

    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: source,
    };
    journal
        .append_journaled(&event)
        .expect("setup: append event");

    let step = StepIdx::new(2);
    let expected_policy = sample_digest(21);
    let mismatched_policy = sample_digest(22);

    let policy_evidence = [PolicyDigestComparison::new(
        step,
        expected_policy,
        mismatched_policy,
    )];
    let result = verify_digests(
        &journal,
        run,
        DigestVerificationRequest::full(
            source,
            ir,
            ir,
            FullDigestEvidence::policy_only(&policy_evidence),
        ),
    );

    assert!(
        matches!(
            result,
            Err(RecoveryError::PolicyDigestMismatch { step: s }) if s == step
        ),
        "verify_digests::Full must reject mismatched policy digest with PolicyDigestMismatch, got {result:?}"
    );
}

#[test]
fn verify_digests_full_zero_digest_corruption_is_not_silently_equal() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(302);
    let source = sample_digest(7);
    let ir = sample_digest(8);

    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: source,
    };
    journal
        .append_journaled(&event)
        .expect("setup: append event");

    let zero = WorkflowDigest::from_bytes([0u8; 32]);
    let action = ActionId::new(13);
    let step = StepIdx::new(0);

    // Sanity: zero digests are byte-equal (this is why the typed-error
    // path is required). If a future regression re-introduces a
    // zero-digest fallback, two corrupted contracts would compare as
    // equal and silently pass.
    assert_eq!(zero, zero, "sanity: zero digests are byte-equal");

    // Two mismatched non-zero digests must surface as ActionAbiMismatch.
    let action_evidence = [ActionAbiDigestComparison::new(
        action,
        sample_digest(1),
        sample_digest(2),
    )];
    let result_action = verify_digests(
        &journal,
        run,
        DigestVerificationRequest::full(
            source,
            ir,
            ir,
            FullDigestEvidence::action_abi_only(&action_evidence),
        ),
    );
    assert!(
        matches!(result_action, Err(RecoveryError::ActionAbiMismatch { .. })),
        "asymmetric action-ABI digest must reject, got {result_action:?}"
    );

    // Two mismatched non-zero policy digests must surface as
    // PolicyDigestMismatch, not silently pass.
    let policy_evidence = [PolicyDigestComparison::new(
        step,
        sample_digest(1),
        sample_digest(2),
    )];
    let result_policy = verify_digests(
        &journal,
        run,
        DigestVerificationRequest::full(
            source,
            ir,
            ir,
            FullDigestEvidence::policy_only(&policy_evidence),
        ),
    );
    assert!(
        matches!(
            result_policy,
            Err(RecoveryError::PolicyDigestMismatch { .. })
        ),
        "asymmetric policy digest must reject, got {result_policy:?}"
    );
}

#[test]
fn recover_full_journal_returns_no_recovery_data_when_empty() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

    let run = RunId::new(999);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
    let Err(RecoveryError::NoRecoveryData { run: found_run }) = result else {
        panic!("expected NoRecoveryData, got {:?}", result);
    };
    assert_eq!(found_run, run);
}

#[test]
fn replay_events_produces_correct_final_state_from_empty() {
    let mut tracker = ActionReplayTracker::new();
    let replayed = replay_events(&[], &mut tracker, &[]).expect("empty replay should succeed");
    assert!(replayed.is_empty());
}

#[test]
fn replay_events_accumulates_state_from_multiple_events() {
    let run = RunId::new(30);
    let action = ActionId::new(1);
    let step = StepIdx::new(0);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(1),
            step,
            action,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(2),
            step,
            action,
            attempt: 1,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let replayed = replay_events(&events, &mut tracker, &[]).expect("replay should succeed");
    assert_eq!(replayed.len(), 3);
    assert!(tracker.is_resolved(action, step));
}

#[test]
fn is_terminal_event_returns_true_for_finished() {
    let event = JournalEvent::RunFinished {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        result: SlotIdx::new(0),
        attempt: 1,
    };
    assert!(is_terminal_event(&event));
}

#[test]
fn is_terminal_event_returns_true_for_failed() {
    let event = JournalEvent::RunFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        attempt: 1,
    };
    assert!(is_terminal_event(&event));
}

#[test]
fn is_terminal_event_returns_true_for_cancelled() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        attempt: 1,
        reason: None,
    };
    assert!(is_terminal_event(&event));
}

#[test]
fn is_terminal_event_returns_false_for_submitted() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: sample_digest(1),
    };
    assert!(!is_terminal_event(&event));
}

#[test]
fn is_terminal_event_returns_false_for_step_started() {
    let event = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    assert!(!is_terminal_event(&event));
}

#[test]
fn extract_terminal_returns_some_for_finished_event() {
    let finished = JournalEvent::RunFinished {
        run: RunId::new(1),
        seq: EventSeq::new(3),
        result: SlotIdx::new(42),
        attempt: 1,
    };
    let events = vec![
        JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
        },
        JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        finished.clone(),
    ];

    let result = extract_terminal(&events);
    assert!(result.is_some());
    assert_eq!(result, Some(&finished));
}

#[test]
fn extract_terminal_returns_none_for_non_terminal_event() {
    let events = vec![JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: sample_digest(1),
    }];

    let terminal = extract_terminal(&events);
    assert!(terminal.is_none());
}

// --- Recovery frame seed divergence and edge case tests ---

/// Multi-run divergence: `summarize_recovery_events` with events for different RunIds
/// should return ReplayDivergence error.
#[test]
fn summarize_recovery_events_rejects_multi_run_divergence() {
    let run_a = RunId::new(500);
    let run_b = RunId::new(501);
    let events = vec![
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
        },
        JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: run_b,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];

    let result = summarize_recovery_events(&events);
    let Err(RecoveryError::ReplayDivergence { step, detail }) = result else {
        panic!(
            "expected ReplayDivergence for multi-run events, got {:?}",
            result
        );
    };
    assert_eq!(step, StepIdx::ZERO);
    assert!(detail.contains("multiple runs"));
}

/// Multi-run divergence: `recover_runtime_frame_seed_from_events` with mixed RunIds
/// should return ReplayDivergence error.
#[test]
fn frame_seed_rejects_multi_run_divergence() {
    let run_a = RunId::new(600);
    let run_b = RunId::new(601);
    let events = vec![
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: sample_digest(2),
        },
        JournalEvent::StepSucceeded {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
        JournalEvent::StepStarted {
            run: run_b,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    let result = recover_runtime_frame_seed_from_events(&events);
    let Err(RecoveryError::ReplayDivergence { step, detail }) = result else {
        panic!(
            "expected ReplayDivergence for mixed-run frame seed, got {:?}",
            result
        );
    };
    assert_eq!(step, StepIdx::ZERO);
    assert!(detail.contains("multiple runs"));
}

/// Empty events: both `summarize_recovery_events` and
/// `recover_runtime_frame_seed_from_events` should return NoRecoveryData.
#[test]
fn empty_events_returns_no_recovery_data() {
    let events: Vec<JournalEvent> = vec![];

    let summary_result = summarize_recovery_events(&events);
    let Err(RecoveryError::NoRecoveryData { .. }) = summary_result else {
        panic!(
            "summarize_recovery_events: expected NoRecoveryData, got {:?}",
            summary_result
        );
    };

    let seed_result = recover_runtime_frame_seed_from_events(&events);
    let Err(RecoveryError::NoRecoveryData { .. }) = seed_result else {
        panic!(
            "recover_runtime_frame_seed_from_events: expected NoRecoveryData, got {:?}",
            seed_result
        );
    };
}

/// When no steps have started, `first_step` should default to `StepIdx::ZERO`.
/// A run with only SlotWritten events (no StepStarted/StepSucceeded) exercises this path.
#[test]
fn frame_seed_first_step_defaults_to_zero_when_no_steps_started() -> Result<(), String> {
    let run = RunId::new(700);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(3),
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(1),
            slot: SlotIdx::new(5),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
        },
    ];

    let seed = recover_runtime_frame_seed_from_events(&events)
        .map_err(|error| format!("seed recovery failed: {error:?}"))?;
    assert_eq!(seed.first_step, StepIdx::ZERO);
    assert_eq!(seed.step_count, 0);
    assert!(seed.steps.is_empty());
    assert_eq!(seed.pc, StepIdx::ZERO);
    Ok(())
}

/// SlotWrittenEvent slot-dimension tracking without StepSucceeded:
/// `max_slot` should update from SlotWritten events alone.
#[test]
fn slot_written_events_track_max_slot_without_step_succeeded() {
    let run = RunId::new(800);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(4),
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(1),
            slot: SlotIdx::new(3),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(7),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(3),
            slot: SlotIdx::new(2),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(4),
            attempt: 1,
        },
    ];

    let seed = recover_runtime_frame_seed_from_events(&events)
        .expect("seed should recover from SlotWritten-only events");
    // max_slot is 7, so slot_count should be 7 + 1 = 8
    assert_eq!(seed.slot_count, 8);
    assert_eq!(seed.summary.slots_written, 3);
}

// --- RecoveryError variant exact tests ---

#[test]
fn recovery_error_action_abi_mismatch_constructs_correctly() {
    let action_id = ActionId::new(42);
    let err = RecoveryError::ActionAbiMismatch { action_id };
    assert!(matches!(err, RecoveryError::ActionAbiMismatch { action_id: a } if a == action_id));
}

#[test]
fn recovery_error_policy_digest_mismatch_constructs_correctly() {
    let step = StepIdx::new(7);
    let err = RecoveryError::PolicyDigestMismatch { step };
    assert!(matches!(err, RecoveryError::PolicyDigestMismatch { step: s } if s == step));
}

#[test]
fn recovery_error_corrupt_snapshot_constructs_correctly() {
    let run = RunId::new(99);
    let seq = EventSeq::new(5);
    let err = RecoveryError::CorruptSnapshot { run, seq };
    assert!(
        matches!(err, RecoveryError::CorruptSnapshot { run: r, seq: s } if r == run && s == seq)
    );
}

#[test]
fn recovery_error_terminal_state_mismatch_constructs_correctly() {
    let expected = "Finished".to_string();
    let found = "Failed".to_string();
    let err = RecoveryError::TerminalStateMismatch {
        expected: expected.clone(),
        found: found.clone(),
    };
    assert!(
        matches!(err, RecoveryError::TerminalStateMismatch { expected: e, found: f } if e == expected && f == found)
    );
}
