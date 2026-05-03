//! Recovery tests for velvet-ballastics journal.

#[cfg(test)]
#[allow(
    clippy::assertions_on_constants,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use crate::recovery::{
        ActionReplayTracker, DigestCheck, RecoveredStepEntry, RecoveredStepState, RecoveryError,
        RecoveryFrameSeed, RecoveryHydration, RecoveryResult, RecoveryRuntimeSummary,
        RecoveryTerminalState, RunSnapshot, UnsupportedRecoveryState, check_compiled_ir_digest,
        check_workflow_source_digest, extract_terminal, is_terminal_event,
        recover_all_incomplete_runs, recover_full_journal, recover_runtime_frame_seed,
        recover_runtime_frame_seed_from_events, recover_runtime_summary,
        recover_snapshot_plus_tail, replay_events, summarize_recovery_events, verify_digests,
    };
    use crate::{EventSeq, FjallJournal, JournalEvent, RunHeaderRecord};
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

    fn test_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    #[test]
    fn summarize_recovery_events_returns_summary_hydration() {
        let run = RunId::new(77);
        let workflow = test_digest(9);
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
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(2),
                action: ActionId::new(5),
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(2),
                action: ActionId::new(5),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(4),
                result: SlotIdx::new(3),
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
    fn recover_runtime_summary_reads_summary_from_journal() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
        let run = RunId::new(79);
        let workflow = test_digest(10);

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
    fn recover_runtime_frame_seed_from_events_rebuilds_dimensions_and_step_states() {
        let run = RunId::new(91);
        let workflow = test_digest(13);
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
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(1),
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
            },
        ];

        let seed = recover_runtime_frame_seed_from_events(&events).expect("seed recovers");

        assert_eq!(seed.summary.run, run);
        assert_eq!(seed.summary.workflow, Some(workflow));
        assert_eq!(seed.step_count, 4);
        assert_eq!(seed.slot_count, 6);
        assert_eq!(seed.pc, StepIdx::new(3));
        assert!(seed.steps.iter().any(
            |entry| entry.step == StepIdx::new(1) && entry.state == RecoveredStepState::Waiting
        ));
        assert!(
            seed.steps.iter().any(|entry| entry.step == StepIdx::new(3)
                && entry.state == RecoveredStepState::Succeeded)
        );
        assert_eq!(
            seed.unsupported,
            UnsupportedRecoveryState {
                slot_values: true,
                slot_taint: true,
                action_payloads: false,
            }
        );
    }

    #[test]
    fn recover_runtime_frame_seed_rejects_dimension_overflow() {
        let run = RunId::new(92);
        let events = vec![JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::MAX,
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
        let workflow = test_digest(14);

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

    #[test]
    fn recover_all_incomplete_runs_returns_only_non_terminal_runs() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
        let workflow = test_digest(11);
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
            })
            .expect("finished append succeeds");

        let recovered =
            recover_all_incomplete_runs(&journal).expect("incomplete recovery succeeds");

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
        let workflow = test_digest(12);

        put_test_header(&journal, run, workflow);

        let result = recover_all_incomplete_runs(&journal);

        assert!(
            matches!(result, Err(RecoveryError::NoRecoveryData { run: found }) if found == run)
        );
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
        terminal: Option<TerminalSummary>,
    }

    fn summarize_events(events: &[JournalEvent]) -> ReplaySummary {
        let mut summary = ReplaySummary::default();
        for event in events {
            match event {
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
                JournalEvent::ActionCompletedEvent { .. } => {
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
                JournalEvent::RunCancelled { .. } => {
                    summary.terminal = Some(TerminalSummary::Cancelled);
                }
                JournalEvent::RunFinished { result, .. } => {
                    summary.terminal = Some(TerminalSummary::Finished(*result));
                }
                JournalEvent::RunFailedEvent { .. } => {
                    summary.terminal = Some(TerminalSummary::Failed);
                }
                JournalEvent::SlotWrittenEvent { .. }
                | JournalEvent::RetryScheduledEvent { .. } => {}
            }
        }
        summary
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
            terminal: tail.terminal.or(base.terminal),
        }
    }

    fn summary_through(events: &[JournalEvent], seq: EventSeq) -> ReplaySummary {
        let mut prefix = Vec::new();
        for event in events {
            if event.seq() <= seq {
                prefix.push(event.clone());
            }
        }
        summarize_events(&prefix)
    }

    fn tail_after(events: &[JournalEvent], seq: EventSeq) -> Vec<JournalEvent> {
        let mut tail = Vec::new();
        for event in events {
            if event.seq() > seq {
                tail.push(event.clone());
            }
        }
        tail
    }

    fn append_events(
        journal: &FjallJournal,
        events: &[JournalEvent],
    ) -> Result<(), crate::JournalError> {
        for event in events {
            journal.append_journaled(event)?;
        }
        Ok(())
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
        let full_replay = recover_full_journal(&journal, run, &mut full_tracker)?;

        let snapshot = RunSnapshot {
            run,
            seq: snapshot_seq,
            workflow: test_digest(1),
            slots: Vec::new(),
            taint: Vec::new(),
        };
        let tail = tail_after(events, snapshot_seq);
        let mut tail_tracker = ActionReplayTracker::new();
        let tail_replay = recover_snapshot_plus_tail(&snapshot, &tail, &mut tail_tracker)?;

        let full_summary = summarize_events(&full_replay);
        let snapshot_summary = summary_through(events, snapshot_seq);
        let tail_summary = summarize_events(&tail_replay);
        let combined_summary = combine_summaries(snapshot_summary, tail_summary);

        assert_eq!(full_summary, combined_summary);
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
        }];

        let result = replay_events(&events, &mut tracker);
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
            },
            JournalEvent::ActionCompletedEvent {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                step,
                action,
            },
        ];

        let replayed =
            replay_events(&events, &mut tracker).expect("first execution should succeed");
        assert_eq!(replayed.len(), 2);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn snapshot_tail_matches_full_journal_lifecycle_summary()
    -> Result<(), Box<dyn std::error::Error>> {
        let run = RunId::new(900);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
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
            },
        ];

        assert_snapshot_tail_matches_full_summary(run, EventSeq::new(1), &events)
    }

    #[test]
    fn snapshot_tail_matches_full_journal_action_summary() -> Result<(), Box<dyn std::error::Error>>
    {
        let run = RunId::new(901);
        let action = ActionId::new(4);
        let step = StepIdx::new(2);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(1),
                step,
                action,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(2),
                step,
                action,
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(3),
                result: SlotIdx::new(0),
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
                workflow: test_digest(1),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(7),
            },
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(2),
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
                workflow: test_digest(1),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(8),
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(8),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(3),
                result: SlotIdx::new(1),
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
        }];

        let result = replay_events(&events, &mut tracker);
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
        let digest = test_digest(42);
        check_compiled_ir_digest(digest, digest).expect("matching digests should succeed");
    }

    #[test]
    fn compiled_ir_digest_mismatch_fails() {
        let expected = test_digest(1);
        let found = test_digest(2);
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
        }));
        assert!(is_terminal_event(&JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(5),
        }));
        assert!(is_terminal_event(&JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(5),
        }));
        assert!(!is_terminal_event(&JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
        }));
    }

    #[test]
    fn extract_terminal_finds_last_terminal() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::RunFinished {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                result: SlotIdx::new(0),
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
            workflow: test_digest(1),
        }];

        let terminal = extract_terminal(&events);
        assert!(terminal.is_none());
    }

    #[test]
    fn snapshot_plus_tail_rejects_event_before_snapshot() {
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            workflow: test_digest(1),
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

        let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);
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

        let result = recover_full_journal(&journal, RunId::new(999), &mut tracker);
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
            workflow: test_digest(1),
        };
        let started = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let finished = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
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
        let replayed = recover_full_journal(&journal, run, &mut tracker)
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
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: SlotIdx::new(0),
                value: None,
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(0),
                action: ActionId::new(1),
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(0),
                action: ActionId::new(1),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(1),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(2),
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(7),
                step: StepIdx::new(2),
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(8),
                step: StepIdx::new(3),
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
            },
        ];

        let mut tracker = ActionReplayTracker::new();
        let replayed =
            replay_events(&events, &mut tracker).expect("replay of all event kinds should succeed");
        assert_eq!(replayed.len(), 11);
        assert!(tracker.is_resolved(ActionId::new(1), StepIdx::new(0)));
    }

    #[test]
    fn snapshot_plus_tail_accepts_valid_tail_events() {
        let snapshot = RunSnapshot {
            run: RunId::new(10),
            seq: EventSeq::new(5),
            workflow: test_digest(1),
            slots: Vec::new(),
            taint: Vec::new(),
        };
        let tail = vec![
            JournalEvent::StepStarted {
                run: RunId::new(10),
                seq: EventSeq::new(6),
                step: StepIdx::new(0),
            },
            JournalEvent::StepSucceeded {
                run: RunId::new(10),
                seq: EventSeq::new(7),
                step: StepIdx::new(0),
                output: SlotIdx::new(1),
            },
        ];
        let mut tracker = ActionReplayTracker::new();

        let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
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
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
            },
        ];

        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker);
        let Err(err) = result else {
            panic!("out-of-order steps should cause divergence");
        };
        assert!(matches!(err, RecoveryError::ReplayDivergence { .. }));
    }

    // --- New Recovery Tests ---

    #[test]
    fn check_workflow_source_digest_returns_mismatch_when_digests_differ() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal =
            crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(100);
        let stored_digest = test_digest(1);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: stored_digest,
        };
        journal
            .append_journaled(&event)
            .expect("setup: append event");

        let wrong_digest = test_digest(2);
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
        let journal =
            crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(101);
        let digest = test_digest(5);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        journal
            .append_journaled(&event)
            .expect("setup: append event");

        check_workflow_source_digest(&journal, run, digest)
            .expect("matching digest should succeed");
    }

    #[test]
    fn check_compiled_ir_digest_returns_mismatch_when_digests_differ() {
        let expected = test_digest(10);
        let found = test_digest(20);
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
        let journal =
            crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(200);
        let digest = test_digest(7);
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
            digest,
            test_digest(8),
            test_digest(8),
            DigestCheck::Full,
        )
        .expect("matching digests at Full level should succeed");
    }

    #[test]
    fn verify_digests_returns_mismatch_when_ir_differs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal =
            crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(201);
        let digest = test_digest(7);
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
            digest,
            test_digest(8),
            test_digest(9),
            DigestCheck::WorkflowAndIr,
        );
        assert!(matches!(
            result,
            Err(RecoveryError::CompiledIrDigestMismatch { .. })
        ));
    }

    #[test]
    fn recover_full_journal_returns_no_recovery_data_when_empty() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal =
            crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(999);
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker);
        let Err(RecoveryError::NoRecoveryData { run: found_run }) = result else {
            panic!("expected NoRecoveryData, got {:?}", result);
        };
        assert_eq!(found_run, run);
    }

    #[test]
    fn replay_events_produces_correct_final_state_from_empty() {
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&[], &mut tracker).expect("empty replay should succeed");
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
                workflow: test_digest(1),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(1),
                step,
                action,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(2),
                step,
                action,
            },
        ];

        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 3);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn is_terminal_event_returns_true_for_finished() {
        let event = JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            result: SlotIdx::new(0),
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_returns_true_for_failed() {
        let event = JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(5),
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_returns_true_for_cancelled() {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(5),
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_returns_false_for_submitted() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        assert!(!is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_returns_false_for_step_started() {
        let event = JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        assert!(!is_terminal_event(&event));
    }

    #[test]
    fn extract_terminal_returns_some_for_finished_event() {
        let finished = JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(3),
            result: SlotIdx::new(42),
        };
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
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
            workflow: test_digest(1),
        }];

        let terminal = extract_terminal(&events);
        assert!(terminal.is_none());
    }
}
