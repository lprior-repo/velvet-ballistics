#![forbid(unsafe_code)]
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
        ActionReplayTracker, DigestCheck, RecoveredStepState, RecoveryError, RecoveryHydration,
        RecoveryTerminalState, RunSnapshot, UnsupportedRecoveryState, check_compiled_ir_digest,
        check_workflow_source_digest, extract_terminal, is_terminal_event,
        recover_all_incomplete_runs, recover_full_journal, recover_runtime_frame_seed,
        recover_runtime_frame_seed_from_events,
        recover_runtime_frame_seed_from_events_with_workflow, recover_runtime_summary,
        recover_snapshot_plus_tail, replay_events, summarize_recovery_events, verify_digests,
    };
    use crate::{EventSeq, FjallJournal, JournalEvent, RunHeaderRecord};
    use vb_core::value::{ConstValue, SlotValue, Taint};
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

    fn sample_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    fn deterministic_plan() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
        CompiledWorkflow::try_from_parts(deterministic_parts())
            .map_err(Box::<dyn std::error::Error>::from)
    }

    fn deterministic_parts() -> WorkflowParts {
        WorkflowParts {
            name: "recovery_replay".into(),
            digest: sample_digest(44),
            nodes: deterministic_nodes().into(),
            expressions: Vec::new().into(),
            accessors: Vec::new().into(),
            constants: vec![ConstValue::I64(42)].into(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    fn deterministic_nodes() -> Vec<CompiledNode> {
        vec![set_const_zero(), copy_zero_to_one(), finish_one()]
    }

    fn set_const_zero() -> CompiledNode {
        CompiledNode {
            id: StepIdx::ZERO,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: vb_core::ConstIdx::new(0),
            },
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
        }
    }

    fn copy_zero_to_one() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(1),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
        }
    }

    fn finish_one() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(2),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
            output: None,
            next: None,
        }
    }

    fn deterministic_replay_events(run: RunId, workflow: WorkflowDigest) -> Vec<JournalEvent> {
        vec![
            accepted_event(run, EventSeq::new(0), workflow),
            started_event(run, EventSeq::new(1), StepIdx::ZERO),
            succeeded_event(run, EventSeq::new(2), StepIdx::ZERO, SlotIdx::new(0)),
            started_event(run, EventSeq::new(3), StepIdx::new(1)),
            succeeded_event(run, EventSeq::new(4), StepIdx::new(1), SlotIdx::new(1)),
        ]
    }

    fn step_succeeded_events(
        run: RunId,
        workflow: WorkflowDigest,
        step: StepIdx,
    ) -> Vec<JournalEvent> {
        vec![
            accepted_event(run, EventSeq::new(0), workflow),
            succeeded_event(run, EventSeq::new(1), step, SlotIdx::new(0)),
        ]
    }

    fn accepted_event(run: RunId, seq: EventSeq, workflow: WorkflowDigest) -> JournalEvent {
        JournalEvent::RunAccepted { run, seq, workflow }
    }

    fn started_event(run: RunId, seq: EventSeq, step: StepIdx) -> JournalEvent {
        JournalEvent::StepStarted { run, seq, step }
    }

    fn succeeded_event(run: RunId, seq: EventSeq, step: StepIdx, output: SlotIdx) -> JournalEvent {
        JournalEvent::StepSucceeded {
            run,
            seq,
            step,
            output,
        }
    }

    fn assert_recovered_i64_slot(seed: &crate::recovery::RecoveryFrameSeed, slot: SlotIdx) {
        assert!(seed.slots.iter().any(|entry| {
            entry.slot == slot && entry.value == SlotValue::I64(42) && entry.taint == Taint::Clean
        }));
    }

    fn assert_compiled_digest_mismatch(
        result: Result<crate::recovery::RecoveryFrameSeed, RecoveryError>,
        expected: WorkflowDigest,
        found: WorkflowDigest,
    ) {
        assert!(matches!(
            result,
            Err(RecoveryError::CompiledIrDigestMismatch { expected: e, found: f })
                if e == expected && f == found
        ));
    }

    fn assert_replay_divergence_step(
        result: Result<crate::recovery::RecoveryFrameSeed, RecoveryError>,
        expected_step: StepIdx,
        expected_detail: &str,
    ) {
        assert!(matches!(
            result,
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == expected_step && detail == expected_detail
        ));
    }

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
    fn recover_runtime_frame_seed_from_events_rebuilds_dimensions_and_step_states()
    -> Result<(), String> {
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

        let seed = recover_runtime_frame_seed_from_events(&events)
            .map_err(|error| format!("seed recovery failed: {error:?}"))?;

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
    fn frame_seed_with_workflow_maps_replay_step_not_found()
    -> Result<(), Box<dyn std::error::Error>> {
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
        let workflow = sample_digest(12);

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
        events
            .iter()
            .fold(ReplaySummary::default(), |mut summary, event| {
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
                    JournalEvent::RunAdmission { .. }
                    | JournalEvent::SlotWrittenEvent { .. }
                    | JournalEvent::RetryScheduledEvent { .. } => {}
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
        let full_replay = recover_full_journal(&journal, run, &mut full_tracker)?;

        let snapshot = RunSnapshot {
            run,
            seq: snapshot_seq,
            workflow: sample_digest(1),
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
                workflow: sample_digest(1),
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
                workflow: sample_digest(1),
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
                workflow: sample_digest(1),
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
                workflow: sample_digest(1),
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
                workflow: sample_digest(1),
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
            workflow: sample_digest(1),
        }];

        let terminal = extract_terminal(&events);
        assert!(terminal.is_none());
    }

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
            workflow: sample_digest(1),
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
                workflow: sample_digest(1),
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
                extra: None,
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
            workflow: sample_digest(1),
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
        let journal =
            crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

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

        check_workflow_source_digest(&journal, run, digest)
            .expect("matching digest should succeed");
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
        let journal =
            crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

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
            digest,
            sample_digest(8),
            sample_digest(8),
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
            digest,
            sample_digest(8),
            sample_digest(9),
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
                workflow: sample_digest(1),
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
                workflow: sample_digest(1),
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
            },
            JournalEvent::StepStarted {
                run: run_b,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
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
            },
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(2),
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
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: SlotIdx::new(7),
                value: None,
                extra: None,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(3),
                slot: SlotIdx::new(2),
                value: None,
                extra: None,
            },
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(4),
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
}
