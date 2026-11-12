#[test]
fn summarize_recovery_events_returns_summary_hydration() {
    let run = RunId::new(77);
    let workflow = sample_digest(9);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action: ActionId::new(5),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(2),
            action: ActionId::new(5),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(4),
            result: SlotIdx::new(3),
            attempt: 1,
        },
    ];

    let hydration = summarize_recovery_events(&events).expect("summary recovery succeeds");
    let RecoveryHydration::Summary(summary) = hydration else {
        panic!("expected summary hydration");
    };

    assert_eq!(summary.run, run);
    assert_eq!(summary.first_seq, EventSeq::new(0));
    assert_eq!(summary.last_seq, EventSeq::new(4));
    assert_eq!(summary.workflow, Some(workflow));
    assert_eq!(summary.steps_started, 1);
    assert_eq!(summary.actions_scheduled, 1);
    assert_eq!(summary.actions_resolved, 1);
    assert_eq!(
        summary.terminal,
        Some(RecoveryTerminalState::Finished {
            result: SlotIdx::new(3),
        })
    );
}

#[test]
fn summarize_recovery_events_counts_duplicate_action_completed_envelope_once() {
    let run = RunId::new(78);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(0),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_completed_envelope_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(42),
            Taint::Clean,
        ),
        recovery_action_completed_envelope_event(
            run,
            EventSeq::new(2),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(42),
            Taint::Clean,
        ),
    ];

    let hydration = summarize_recovery_events(&events).expect("summary recovery succeeds");
    let RecoveryHydration::Summary(summary) = hydration else {
        panic!("expected summary hydration");
    };

    assert_eq!(summary.actions_scheduled, 1);
    assert_eq!(summary.actions_resolved, 1);
    assert_eq!(summary.steps_succeeded, 1);
    assert_eq!(summary.slots_written, 1);
}

#[test]
fn summarize_recovery_events_counts_duplicate_action_scheduled_ticket_once() {
    let run = RunId::new(780);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(0),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_completed_envelope_event(
            run,
            EventSeq::new(2),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(42),
            Taint::Clean,
        ),
    ];

    let hydration = summarize_recovery_events(&events).expect("summary recovery succeeds");
    let RecoveryHydration::Summary(summary) = hydration else {
        panic!("expected summary hydration");
    };

    assert_eq!(summary.actions_scheduled, 1);
    assert_eq!(summary.actions_resolved, 1);
}

#[test]
fn summarize_recovery_events_rejects_divergent_action_scheduled_ticket() {
    let run = RunId::new(781);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(0),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(2),
        ),
    ];

    let result = summarize_recovery_events(&events);

    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO && detail == "divergent action schedule ticket"
    ));
}

#[test]
fn summarize_recovery_events_rejects_completion_output_mismatch_with_schedule() {
    let run = RunId::new(782);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(0),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_completed_envelope_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(2),
            SlotValue::I64(42),
            Taint::Clean,
        ),
    ];

    let result = summarize_recovery_events(&events);

    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO
                && detail == "action completion envelope does not match schedule ticket"
    ));
}

#[test]
fn summarize_recovery_events_rejects_action_completed_envelope_without_schedule() {
    let run = RunId::new(786);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![recovery_action_completed_envelope_event(
        run,
        EventSeq::new(0),
        ticket,
        SlotIdx::new(1),
        SlotValue::I64(42),
        Taint::Clean,
    )];

    let result = summarize_recovery_events(&events);

    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO
                && detail == "action completion envelope missing schedule ticket"
    ));
}

#[test]
fn replay_events_accepts_identical_duplicate_action_scheduled_ticket() {
    let run = RunId::new(783);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(0),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_completed_envelope_event(
            run,
            EventSeq::new(2),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(42),
            Taint::Clean,
        ),
    ];
    let mut tracker = ActionReplayTracker::new();
    let expected = [(ticket.action, sample_digest(0xA1))];

    let result = replay_events(&events, &mut tracker, &expected);

    assert!(result.is_ok(), "expected Ok, got {:?}", result);
}

#[test]
fn replay_events_rejects_divergent_action_scheduled_ticket() {
    let run = RunId::new(784);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(0),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(2),
        ),
    ];
    let mut tracker = ActionReplayTracker::new();
    let expected = [(ticket.action, sample_digest(0xA1))];

    let result = replay_events(&events, &mut tracker, &expected);

    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO && detail == "divergent action schedule ticket"
    ));
}

#[test]
fn replay_events_rejects_completion_output_mismatch_with_schedule() {
    let run = RunId::new(785);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(0),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_completed_envelope_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(2),
            SlotValue::I64(42),
            Taint::Clean,
        ),
    ];
    let mut tracker = ActionReplayTracker::new();
    let expected = [(ticket.action, sample_digest(0xA1))];

    let result = replay_events(&events, &mut tracker, &expected);

    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO
                && detail == "action completion envelope does not match schedule ticket"
    ));
}

#[test]
fn replay_events_rejects_action_completed_envelope_without_schedule() {
    let run = RunId::new(787);
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, ActionId::new(1));
    let events = vec![recovery_action_completed_envelope_event(
        run,
        EventSeq::new(0),
        ticket,
        SlotIdx::new(1),
        SlotValue::I64(42),
        Taint::Clean,
    )];
    let mut tracker = ActionReplayTracker::new();

    let result = replay_events(&events, &mut tracker, &[]);

    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO
                && detail == "action completion envelope missing schedule ticket"
    ));
}

#[test]
fn recover_runtime_summary_reads_summary_from_journal() {
    let dir = tempfile::tempdir().expect("temp dir");
    let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
    let run = RunId::new(79);
    let workflow = sample_digest(10);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("accepted append succeeds");
    journal
        .append_journaled(&JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        })
        .expect("cancelled append succeeds");

    let summary = recover_runtime_summary(&journal, run)
        .expect("summary recovers")
        .summary();

    assert_eq!(summary.run, run);
    assert_eq!(summary.workflow, Some(workflow));
    assert_eq!(summary.terminal, Some(RecoveryTerminalState::Cancelled));
}

#[test]
fn recover_runtime_frame_seed_from_events_rebuilds_dimensions_and_step_states() -> Result<(), String>
{
    let run = RunId::new(91);
    let workflow = sample_digest(13);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(3),
            output: SlotIdx::new(4),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(4),
            result: SlotIdx::new(5),
            attempt: 1,
        },
    ];

    let seed = recover_runtime_frame_seed_from_events(&events)
        .map_err(|error| format!("seed recovery failed: {error:?}"))?;

    assert_eq!(seed.summary.run, run);
    assert_eq!(seed.summary.workflow, Some(workflow));
    assert_eq!(seed.step_count, 4);
    assert_eq!(seed.slot_count, 6);
    assert_eq!(seed.pc, StepIdx::new(3));
    assert!(
        seed.steps.iter().any(
            |entry| entry.step == StepIdx::new(1) && entry.state == RecoveredStepState::Waiting
        )
    );
    assert!(
        seed.steps
            .iter()
            .any(|entry| entry.step == StepIdx::new(3)
                && entry.state == RecoveredStepState::Succeeded)
    );
    assert_eq!(
        seed.unsupported,
        UnsupportedRecoveryState {
            slot_values: true,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
        }
    );
    Ok(())
}

#[test]
fn frame_seed_with_workflow_replays_deterministic_slot_values()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94);
    let plan = deterministic_plan()?;
    let events = deterministic_replay_events(run, sample_digest(44));

    let seed = recover_runtime_frame_seed_from_events_with_workflow(&events, &plan)?;

    assert!(!seed.unsupported.slot_values);
    assert!(!seed.unsupported.slot_taint);
    assert_recovered_i64_slot(&seed, SlotIdx::new(0));
    assert_recovered_i64_slot(&seed, SlotIdx::new(1));
    Ok(())
}

#[test]
fn frame_seed_with_workflow_preserves_action_completed_envelope_output_slot_value()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(943);
    let digest = sample_digest(55);
    let action = ActionId::new(4);
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: "action_recovery".into(),
        input_slots: Box::new([]),
        digest,
        nodes: vec![node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action,
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            None,
        )]
        .into(),
        expressions: Vec::new().into(),
        accessors: Vec::new().into(),
        constants: Vec::new().into(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let plan = CompiledWorkflow::try_from_parts(parts)?;
    let ticket = recovery_action_ticket(run, StepIdx::ZERO, action);
    let events = vec![
        accepted_event(run, EventSeq::new(0), digest),
        recovery_action_scheduled_ticket_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
        ),
        recovery_action_completed_envelope_event(
            run,
            EventSeq::new(2),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(1234),
            Taint::Secret,
        ),
    ];

    let seed = recover_runtime_frame_seed_from_events_with_workflow(&events, &plan)?;

    assert!(seed.slots.iter().any(|entry| {
        entry.slot == SlotIdx::new(1)
            && entry.value == SlotValue::I64(1234)
            && entry.taint == Taint::Secret
    }));
    assert!(seed.steps.iter().any(|entry| {
        entry.step == StepIdx::ZERO && entry.state == RecoveredStepState::Succeeded
    }));
    assert!(!seed.unsupported.slot_values);
    assert!(!seed.unsupported.action_payloads);
    Ok(())
}

#[test]
fn frame_seed_builder_delegates_to_workflow_replay() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(941);
    let plan = deterministic_plan()?;
    let events = deterministic_replay_events(run, sample_digest(44));

    let seed = crate::recovery::RecoveryFrameSeedBuilder::new()
        .with_workflow(&plan)
        .build(&events)?;

    assert_recovered_i64_slot(&seed, SlotIdx::new(1));
    assert_eq!(
        seed.unsupported,
        UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
        }
    );
    Ok(())
}

#[test]
fn frame_seed_with_workflow_rejects_digest_mismatch_before_replay()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(95);
    let plan = deterministic_plan()?;
    let mismatched = sample_digest(45);
    let events = step_succeeded_events(run, mismatched, StepIdx::new(99));

    let result = recover_runtime_frame_seed_from_events_with_workflow(&events, &plan);

    assert_compiled_digest_mismatch(result, sample_digest(44), mismatched);
    Ok(())
}

#[test]
fn frame_seed_with_workflow_maps_replay_step_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(96);
    let plan = deterministic_plan()?;
    let events = step_succeeded_events(run, sample_digest(44), StepIdx::new(99));

    let result = recover_runtime_frame_seed_from_events_with_workflow(&events, &plan);

    assert_replay_divergence_step(
        result,
        StepIdx::new(99),
        "replay step not found in compiled workflow",
    );
    Ok(())
}

#[test]
fn recover_runtime_frame_seed_rejects_dimension_overflow() {
    let run = RunId::new(92);
    let events = vec![JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::MAX,
        attempt: 1,
    }];

    let result = recover_runtime_frame_seed_from_events(&events);

    assert!(
        matches!(result, Err(RecoveryError::FrameDimensionOverflow { run: found }) if found == run)
    );
}

#[test]
fn recover_runtime_frame_seed_reads_events_from_journal() {
    let dir = tempfile::tempdir().expect("temp dir");
    let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
    let run = RunId::new(93);
    let workflow = sample_digest(14);

    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("accepted append succeeds");
    journal
        .append_journaled(&JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            attempt: 1,
        })
        .expect("ask append succeeds");

    let seed = recover_runtime_frame_seed(&journal, run).expect("seed recovers");

    assert_eq!(seed.step_count, 3);
    assert_eq!(seed.slot_count, 0);
    assert_eq!(seed.pc, StepIdx::new(2));
    assert!(seed.steps.iter().any(
        |entry| entry.step == StepIdx::new(2) && entry.state == RecoveredStepState::Asking
    ));
}
