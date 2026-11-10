#[test]
fn recover_runtime_frame_seed_reopens_fjall_pending_action_ticket()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let run = RunId::new(94);
    let step = StepIdx::new(2);
    let action = ActionId::new(7);
    let workflow = sample_digest(15);
    let input = SlotIdx::new(1);
    let output = SlotIdx::new(3);
    let ticket = recovery_action_ticket(run, step, action);
    let events = [
        accepted_event(run, EventSeq::new(0), workflow),
        recovery_action_scheduled_ticket_event(run, EventSeq::new(1), ticket, input, output),
    ];

    {
        let journal = FjallJournal::open(temp_dir.path(), None)?;
        journal.append_strict_batch(&events)?;
    }

    let reopened = FjallJournal::open(temp_dir.path(), None)?;
    let seed = recover_runtime_frame_seed(&reopened, run)?;
    let cannot_resume = seed.cannot_resume_state();

    assert_eq!(seed.summary.workflow, Some(workflow));
    assert_eq!(seed.summary.actions_scheduled, 1);
    assert_eq!(seed.step_count, 3);
    assert_eq!(seed.slot_count, 4);
    assert_eq!(seed.pc, step);
    assert!(
        seed.pending_actions
            .iter()
            .any(|entry| entry.step == step && entry.action == action)
    );
    assert!(seed.unsupported.pending_actions);
    assert!(cannot_resume.pending_actions);
    assert_eq!(cannot_resume.unsupported_reason(), "pending_actions");
    Ok(())
}

#[test]
fn recover_runtime_frame_seed_reopens_fjall_wait_and_ask_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let run = RunId::new(95);
    let workflow = sample_digest(16);
    let wait_step = StepIdx::new(1);
    let ask_step = StepIdx::new(2);
    let events = [
        accepted_event(run, EventSeq::new(0), workflow),
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(1),
            step: wait_step,
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: ask_step,
            attempt: 1,
        },
    ];

    {
        let journal = FjallJournal::open(temp_dir.path(), None)?;
        journal.append_strict_batch(&events)?;
    }

    let reopened = FjallJournal::open(temp_dir.path(), None)?;
    let seed = recover_runtime_frame_seed(&reopened, run)?;
    let cannot_resume = seed.cannot_resume_state();

    assert_eq!(seed.summary.workflow, Some(workflow));
    assert_eq!(seed.summary.suspensions, 2);
    assert_eq!(seed.step_count, 3);
    assert_eq!(seed.pc, ask_step);
    assert!(
        seed.steps
            .iter()
            .any(|entry| entry.step == wait_step && entry.state == RecoveredStepState::Waiting)
    );
    assert!(
        seed.steps
            .iter()
            .any(|entry| entry.step == ask_step && entry.state == RecoveredStepState::Asking)
    );
    assert_eq!(seed.unsupported, UnsupportedRecoveryState::SUPPORTED);
    assert!(cannot_resume.pending_timers);
    assert!(cannot_resume.pending_asks);
    assert_eq!(cannot_resume.unsupported_reason(), "pending_timers");
    Ok(())
}

#[test]
fn recover_all_incomplete_runs_returns_only_non_terminal_runs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
    let workflow = sample_digest(11);
    let incomplete = RunId::new(81);
    let finished = RunId::new(82);

    put_test_header(&journal, incomplete, workflow);
    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run: incomplete,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("incomplete accepted append succeeds");
    journal
        .append_journaled(&JournalEvent::StepStarted {
            run: incomplete,
            seq: EventSeq::new(1),
            step: StepIdx::new(4),
            attempt: 1,
        })
        .expect("incomplete step append succeeds");
    journal
        .append_journaled(&JournalEvent::RunAccepted {
            run: finished,
            seq: EventSeq::new(0),
            workflow,
        })
        .expect("finished accepted append succeeds");
    journal
        .append_journaled(&JournalEvent::RunFinished {
            run: finished,
            seq: EventSeq::new(1),
            result: SlotIdx::new(2),
            attempt: 1,
        })
        .expect("finished append succeeds");

    let recovered = recover_all_incomplete_runs(&journal).expect("incomplete recovery succeeds");

    assert_eq!(recovered.len(), 1);
    assert_eq!(
        recovered.first().expect("one recovery").summary().run,
        incomplete
    );
}

#[test]
fn recover_all_incomplete_runs_rejects_header_without_journal() {
    let dir = tempfile::tempdir().expect("temp dir");
    let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
    let run = RunId::new(83);
    let workflow = sample_digest(12);

    put_test_header(&journal, run, workflow);

    let result = recover_all_incomplete_runs(&journal);

    assert!(matches!(result, Err(RecoveryError::NoRecoveryData { run: found }) if found == run));
}

fn put_test_header(journal: &FjallJournal, run: RunId, digest: WorkflowDigest) {
    journal
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 123,
        })
        .expect("header write succeeds");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalSummary {
    Cancelled,
    Finished(SlotIdx),
    Failed,
    Killed,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ReplaySummary {
    accepted: usize,
    step_started: usize,
    step_succeeded: usize,
    action_scheduled: usize,
    action_completed: usize,
    action_failed: usize,
    wait_scheduled: usize,
    ask_scheduled: usize,
    ask_answered: usize,
    /// Boundary-suspension count. Mirrors production `RecoveryRuntimeSummary::suspensions`
    /// semantics: incremented by `RetryScheduledEvent` only. `WaitResolvedEvent`
    /// (kind 31, RE-009) is a resumption and must NOT inflate this count.
    suspensions: usize,
    terminal: Option<TerminalSummary>,
}

fn summarize_events(events: &[JournalEvent]) -> ReplaySummary {
    events
        .iter()
        .fold(ReplaySummary::default(), |mut summary, event| {
            match event {
                JournalEvent::RunContinuation { .. } => {}
                JournalEvent::RunAccepted { .. } => {
                    summary.accepted = summary.accepted.saturating_add(1);
                }
                JournalEvent::StepStarted { .. } => {
                    summary.step_started = summary.step_started.saturating_add(1);
                }
                JournalEvent::StepSucceeded { .. } => {
                    summary.step_succeeded = summary.step_succeeded.saturating_add(1);
                }
                JournalEvent::ActionScheduled { .. } => {
                    summary.action_scheduled = summary.action_scheduled.saturating_add(1);
                }
                JournalEvent::ActionScheduledTicket { .. } => {
                    summary.action_scheduled = summary.action_scheduled.saturating_add(1);
                }
                JournalEvent::ActionCompletedEvent { .. } => {
                    summary.action_completed = summary.action_completed.saturating_add(1);
                }
                JournalEvent::ActionCompletedEnvelope { .. } => {
                    summary.action_completed = summary.action_completed.saturating_add(1);
                }
                JournalEvent::ActionFailedEvent { .. } => {
                    summary.action_failed = summary.action_failed.saturating_add(1);
                }
                JournalEvent::WaitScheduledEvent { .. } => {
                    summary.wait_scheduled = summary.wait_scheduled.saturating_add(1);
                }
                JournalEvent::AskScheduledEvent { .. } => {
                    summary.ask_scheduled = summary.ask_scheduled.saturating_add(1);
                }
                JournalEvent::AskAnsweredEvent { .. } => {
                    summary.ask_answered = summary.ask_answered.saturating_add(1);
                }
                JournalEvent::RetryScheduledEvent { .. } => {
                    summary.suspensions = summary.suspensions.saturating_add(1);
                }
                JournalEvent::RunCancelled { .. } => {
                    summary.terminal = Some(TerminalSummary::Cancelled);
                }
                JournalEvent::RunKilled { .. } => {
                    summary.terminal = Some(TerminalSummary::Killed);
                }
                JournalEvent::RunFinished { result, .. } => {
                    summary.terminal = Some(TerminalSummary::Finished(*result));
                }
                JournalEvent::RunFailedEvent { .. } => {
                    summary.terminal = Some(TerminalSummary::Failed);
                }
                JournalEvent::RunAdmission { .. }
                | JournalEvent::RunAdmissionBudget { .. }
                | JournalEvent::RetryCheckpoint { .. }
                | JournalEvent::StepFailed { .. }
                | JournalEvent::SlotWrittenEvent { .. }
                | JournalEvent::WaitResolvedEvent { .. }
                | JournalEvent::AskTimedOutEvent { .. }
                | JournalEvent::RunResumed { .. }
                | JournalEvent::RunRetried { .. }
                | JournalEvent::RunAnswered { .. }
                | JournalEvent::ActionAbandoned { .. }
                | JournalEvent::PolicyDecisionEvent { .. }
                | JournalEvent::GateDecisionEvent { .. }
                | JournalEvent::GateOpenedEvent { .. }
                | JournalEvent::CompatibilityDecisionEvent { .. }
                | JournalEvent::TenantAdmissionEvent { .. } => {}
            }
            summary
        })
}

fn combine_summaries(base: ReplaySummary, tail: ReplaySummary) -> ReplaySummary {
    ReplaySummary {
        accepted: base.accepted.saturating_add(tail.accepted),
        step_started: base.step_started.saturating_add(tail.step_started),
        step_succeeded: base.step_succeeded.saturating_add(tail.step_succeeded),
        action_scheduled: base.action_scheduled.saturating_add(tail.action_scheduled),
        action_completed: base.action_completed.saturating_add(tail.action_completed),
        action_failed: base.action_failed.saturating_add(tail.action_failed),
        wait_scheduled: base.wait_scheduled.saturating_add(tail.wait_scheduled),
        ask_scheduled: base.ask_scheduled.saturating_add(tail.ask_scheduled),
        ask_answered: base.ask_answered.saturating_add(tail.ask_answered),
        suspensions: base.suspensions.saturating_add(tail.suspensions),
        terminal: tail.terminal.or(base.terminal),
    }
}

fn summary_through(events: &[JournalEvent], seq: EventSeq) -> ReplaySummary {
    let prefix = events
        .iter()
        .filter(|event| event.seq() <= seq)
        .cloned()
        .collect::<Vec<_>>();
    summarize_events(&prefix)
}

fn tail_after(events: &[JournalEvent], seq: EventSeq) -> Vec<JournalEvent> {
    events
        .iter()
        .filter(|event| event.seq() > seq)
        .cloned()
        .collect()
}

fn append_events(
    journal: &FjallJournal,
    events: &[JournalEvent],
) -> Result<(), crate::JournalError> {
    events
        .iter()
        .try_for_each(|event| journal.append_journaled(event))
}

fn assert_snapshot_tail_matches_full_summary(
    run: RunId,
    snapshot_seq: EventSeq,
    events: &[JournalEvent],
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(temp_dir.path(), None)?;
    append_events(&journal, events)?;

    let mut full_tracker = ActionReplayTracker::new();
    let full_replay = recover_full_journal(&journal, run, &mut full_tracker, &[], &[])?;

    let snapshot = RunSnapshot {
        run,
        seq: snapshot_seq,
        workflow: sample_digest(1),
        slots: Vec::new(),
        taint: Vec::new(),
    };
    let tail = tail_after(events, snapshot_seq);
    let mut tail_tracker = ActionReplayTracker::new();
    let tail_replay = recover_snapshot_plus_tail(&snapshot, &tail, &mut tail_tracker, &[])?;

    let full_summary = summarize_events(&full_replay);
    let snapshot_summary = summary_through(events, snapshot_seq);
    let tail_summary = summarize_events(&tail_replay);
    let combined_summary = combine_summaries(snapshot_summary, tail_summary);

    assert_eq!(full_summary, combined_summary);
    Ok(())
}

#[test]
fn ask_timed_out_recovery_summary_does_not_count_as_answered() -> Result<(), String> {
    let run = RunId::new(84);
    let events = [
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::AskTimedOutEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            attempt: 1,
        },
    ];

    let summary = summarize_events(&events);

    let timeout_event = events
        .get(1)
        .ok_or_else(|| String::from("missing timed-out event"))?;
    assert_eq!(timeout_event.record_kind(), RecordKind::AskTimedOut);
    assert_ne!(timeout_event.record_kind(), RecordKind::AskAnswered);
    assert_eq!(summary.ask_answered, 1);
    Ok(())
}

#[test]
fn wait_resolved_event_does_not_inflate_suspension_count() -> Result<(), String> {
    // PO: RE-009 — WaitResolvedEvent (kind 31) is the resumption of a suspended
    // run, not a new suspension. Only RetryScheduledEvent (kind 19) should
    // increment the suspension counter; WaitResolvedEvent must be invisible to it.
    let run = RunId::new(85);
    let events = [
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::WaitResolvedEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    let summary = summarize_events(&events);

    let resolved = events
        .get(1)
        .ok_or_else(|| String::from("missing WaitResolvedEvent"))?;
    assert_eq!(
        resolved.record_kind(),
        RecordKind::WaitResolved,
        "WaitResolvedEvent must map to RecordKind::WaitResolved"
    );
    assert_eq!(
        summary.suspensions, 0,
        "WaitResolvedEvent must NOT inflate the suspension counter (RE-009: resumption, not retry)"
    );
    Ok(())
}

#[test]
fn action_tracker_blocks_non_idempotent_replay() {
    let mut tracker = ActionReplayTracker::new();
    let action = ActionId::new(1);
    let step = StepIdx::new(5);

    tracker.mark_completed(action, step);
    assert!(tracker.is_resolved(action, step));

    let events = vec![JournalEvent::ActionScheduled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        step,
        action,
        attempt: 1,
    }];

    let result = replay_events(&events, &mut tracker, &[]);
    let Err(err) = result else {
        panic!("replay should fail for already-completed action");
    };
    assert!(matches!(
        err,
        RecoveryError::NonIdempotentActionBlocked { .. }
    ));
}

#[test]
fn action_tracker_allows_first_execution() {
    let mut tracker = ActionReplayTracker::new();
    let action = ActionId::new(1);
    let step = StepIdx::new(5);

    let events = vec![
        JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step,
            action,
            attempt: 1,
        },
    ];

    let replayed =
        replay_events(&events, &mut tracker, &[]).expect("first execution should succeed");
    assert_eq!(replayed.len(), 2);
    assert!(tracker.is_resolved(action, step));
}

#[test]
fn snapshot_tail_matches_full_journal_lifecycle_summary() -> Result<(), Box<dyn std::error::Error>>
{
    let run = RunId::new(900);
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
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            output: SlotIdx::new(3),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::new(3),
            attempt: 1,
        },
    ];

    assert_snapshot_tail_matches_full_summary(run, EventSeq::new(1), &events)
}

#[test]
fn snapshot_tail_matches_full_journal_action_summary() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(901);
    let action = ActionId::new(4);
    let step = StepIdx::new(2);
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
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];

    assert_snapshot_tail_matches_full_summary(run, EventSeq::new(1), &events)
}

#[test]
fn snapshot_tail_matches_full_journal_wait_summary() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(902);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(7),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
            reason: None,
        },
    ];

    assert_snapshot_tail_matches_full_summary(run, EventSeq::new(0), &events)
}

#[test]
fn snapshot_tail_matches_full_journal_ask_summary() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(903);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(8),
            attempt: 1,
        },
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(8),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::new(1),
            attempt: 1,
        },
    ];

    assert_snapshot_tail_matches_full_summary(run, EventSeq::new(1), &events)
}

#[test]
fn action_tracker_tracks_failed_actions() {
    let mut tracker = ActionReplayTracker::new();
    let action = ActionId::new(2);
    let step = StepIdx::new(3);

    tracker.mark_failed(action, step);
    assert!(tracker.is_resolved(action, step));

    let events = vec![JournalEvent::ActionScheduled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        step,
        action,
        attempt: 1,
    }];

    let result = replay_events(&events, &mut tracker, &[]);
    let Err(err) = result else {
        panic!("replay should fail for already-failed action");
    };
    assert!(matches!(
        err,
        RecoveryError::NonIdempotentActionBlocked { .. }
    ));
}

#[test]
fn compiled_ir_digest_match_succeeds() {
    let digest = sample_digest(42);
    check_compiled_ir_digest(digest, digest).expect("matching digests should succeed");
}

#[test]
fn compiled_ir_digest_mismatch_fails() {
    let expected = sample_digest(1);
    let found = sample_digest(2);
    let Err(err) = check_compiled_ir_digest(expected, found) else {
        panic!("mismatched digests should fail");
    };
    assert!(matches!(
        err,
        RecoveryError::CompiledIrDigestMismatch { .. }
    ));
}

#[test]
fn is_terminal_event_identifies_terminals() {
    assert!(is_terminal_event(&JournalEvent::RunFinished {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        result: SlotIdx::new(0),
        attempt: 1,
    }));
    assert!(is_terminal_event(&JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        attempt: 1,
        reason: None,
    }));
    assert!(is_terminal_event(&JournalEvent::RunFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        attempt: 1,
    }));
    assert!(!is_terminal_event(&JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        attempt: 1,
    }));
}

#[test]
fn extract_terminal_finds_last_terminal() {
    let events = vec![
        JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
        },
        JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];

    let terminal = extract_terminal(&events);
    assert!(terminal.is_some());
    assert!(matches!(terminal, Some(JournalEvent::RunFinished { .. })));
}

#[test]
fn extract_terminal_returns_none_without_terminal() {
    let events = vec![JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: sample_digest(1),
    }];

    let terminal = extract_terminal(&events);
    assert!(terminal.is_none());
}
