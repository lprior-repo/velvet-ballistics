#![forbid(unsafe_code)]
#![cfg(test)]

mod tests {
    use crate::recovery::replay::core::{extract_terminal, is_terminal_event, replay_events};
    use crate::recovery::replay::summary::{
        apply_summary_event, recover_run_admission_from_events,
        recover_runtime_frame_seed_from_events, summarize_recovery_events,
    };
    use crate::recovery::types::{
        ActionReplayTracker, RecoveryError, RecoveryRuntimeSummary, RecoveryTerminalState,
        UnsupportedRecoveryState,
    };
    use crate::{EventSeq, JournalEvent};
    use vb_core::value::ConstValue;
    use vb_core::{
        ActionId, CapabilitySet, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest,
    };

    fn sample_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    // =========================================================================
    // RecoveryError variant tests
    // =========================================================================

    #[test]
    fn recovery_error_journal_error_from_source() {
        let source = crate::JournalError::PostcardDecodeFailed;
        let err = RecoveryError::Journal(source);
        assert!(matches!(err, RecoveryError::Journal(_)));
    }

    #[test]
    fn recovery_error_workflow_source_digest_mismatch() {
        let expected = sample_digest(1);
        let found = sample_digest(2);
        let err = RecoveryError::WorkflowSourceDigestMismatch { expected, found };
        assert!(matches!(
            err,
            RecoveryError::WorkflowSourceDigestMismatch {
                expected: e,
                found: f
            } if e == expected && f == found
        ));
    }

    #[test]
    fn recovery_error_compiled_ir_digest_mismatch() {
        let expected = sample_digest(3);
        let found = sample_digest(4);
        let err = RecoveryError::CompiledIrDigestMismatch { expected, found };
        assert!(matches!(
            err,
            RecoveryError::CompiledIrDigestMismatch {
                expected: e,
                found: f
            } if e == expected && f == found
        ));
    }

    #[test]
    fn recovery_error_action_abi_mismatch() {
        let action_id = ActionId::new(99);
        let expected = WorkflowDigest::from_bytes([1u8; 32]);
        let found = WorkflowDigest::from_bytes([2u8; 32]);
        let err = RecoveryError::ActionAbiMismatch {
            action_id,
            expected,
            found,
        };
        assert!(matches!(
            err,
            RecoveryError::ActionAbiMismatch { action_id: a, .. } if a == action_id
        ));
    }

    #[test]
    fn recovery_error_policy_digest_mismatch() {
        let step = StepIdx::new(5);
        let expected = WorkflowDigest::from_bytes([1u8; 32]);
        let found = WorkflowDigest::from_bytes([2u8; 32]);
        let err = RecoveryError::PolicyDigestMismatch {
            step,
            expected,
            found,
        };
        assert!(matches!(
            err,
            RecoveryError::PolicyDigestMismatch { step: s, .. } if s == step
        ));
    }

    #[test]
    fn recovery_error_non_idempotent_action_blocked() {
        let action = ActionId::new(7);
        let step = StepIdx::new(3);
        let err = RecoveryError::NonIdempotentActionBlocked { action, step };
        assert!(matches!(
            err,
            RecoveryError::NonIdempotentActionBlocked { action: a, step: s }
                if a == action && s == step
        ));
    }

    #[test]
    fn recovery_error_replay_divergence() {
        let step = StepIdx::new(11);
        let detail = "step ordering violation".to_owned();
        let err = RecoveryError::ReplayDivergence {
            step,
            detail: detail.clone(),
        };
        assert!(matches!(
            err,
            RecoveryError::ReplayDivergence { step: s, detail: d }
                if s == StepIdx::new(11) && d == detail
        ));
    }

    #[test]
    fn recovery_error_no_recovery_data() {
        let run = RunId::new(42);
        let err = RecoveryError::NoRecoveryData { run };
        assert!(matches!(err, RecoveryError::NoRecoveryData { run: r } if r == run));
    }

    #[test]
    fn recovery_error_corrupt_snapshot() {
        let run = RunId::new(13);
        let seq = EventSeq::new(7);
        let err = RecoveryError::CorruptSnapshot { run, seq };
        assert!(matches!(
            err,
            RecoveryError::CorruptSnapshot { run: r, seq: s } if r == run && s == seq
        ));
    }

    #[test]
    fn recovery_error_terminal_state_mismatch() {
        let expected = "Finished".to_string();
        let found = "Cancelled".to_string();
        let err = RecoveryError::TerminalStateMismatch {
            expected: expected.clone(),
            found: found.clone(),
        };
        assert!(matches!(
            err,
            RecoveryError::TerminalStateMismatch {
                expected: e,
                found: f
            } if e == expected && f == found
        ));
    }

    #[test]
    fn recovery_error_frame_dimension_overflow() {
        let run = RunId::new(77);
        let err = RecoveryError::FrameDimensionOverflow { run };
        assert!(matches!(
            err,
            RecoveryError::FrameDimensionOverflow { run: r } if r == run
        ));
    }

    // =========================================================================
    // RecoveryTerminalState variant tests
    // =========================================================================

    #[test]
    fn recovery_terminal_state_cancelled() {
        let state = RecoveryTerminalState::Cancelled;
        assert!(matches!(state, RecoveryTerminalState::Cancelled));
    }

    #[test]
    fn recovery_terminal_state_finished() {
        let result = SlotIdx::new(5);
        let state = RecoveryTerminalState::Finished { result };
        assert!(matches!(state, RecoveryTerminalState::Finished { result: r } if r == result));
    }

    #[test]
    fn recovery_terminal_state_failed() {
        let state = RecoveryTerminalState::Failed;
        assert!(matches!(state, RecoveryTerminalState::Failed));
    }

    #[test]
    fn recovery_terminal_state_equality() {
        let a = RecoveryTerminalState::Cancelled;
        let b = RecoveryTerminalState::Cancelled;
        let c = RecoveryTerminalState::Failed;
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // =========================================================================
    // RecoveryRuntimeSummary construction tests
    // =========================================================================

    #[test]
    fn recovery_runtime_summary_construction() {
        let run = RunId::new(1);
        let first_seq = EventSeq::new(0);
        let last_seq = EventSeq::new(10);
        let workflow = Some(sample_digest(9));
        let summary = RecoveryRuntimeSummary {
            run,
            first_seq,
            last_seq,
            workflow,
            steps_started: 5,
            steps_succeeded: 4,
            actions_scheduled: 3,
            actions_resolved: 3,
            suspensions: 2,
            slots_written: 6,
            terminal: Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(1),
            }),
        };
        assert_eq!(summary.run, run);
        assert_eq!(summary.first_seq, first_seq);
        assert_eq!(summary.last_seq, last_seq);
        assert_eq!(summary.workflow, workflow);
        assert_eq!(summary.steps_started, 5);
        assert_eq!(summary.steps_succeeded, 4);
        assert_eq!(summary.actions_scheduled, 3);
        assert_eq!(summary.actions_resolved, 3);
        assert_eq!(summary.suspensions, 2);
        assert_eq!(summary.slots_written, 6);
        assert!(matches!(
            summary.terminal,
            Some(RecoveryTerminalState::Finished { result })
                if result == SlotIdx::new(1)
        ));
    }

    #[test]
    fn recovery_runtime_summary_with_no_terminal() {
        let summary = RecoveryRuntimeSummary {
            run: RunId::new(2),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(5),
            workflow: None,
            steps_started: 1,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        assert_eq!(summary.terminal, None);
        assert_eq!(summary.workflow, None);
    }

    // =========================================================================
    // UnsupportedRecoveryState tests
    // =========================================================================

    #[test]
    fn unsupported_recovery_state_supported_is_all_false() {
        let state = UnsupportedRecoveryState::SUPPORTED;
        assert!(!state.slot_values);
        assert!(!state.slot_taint);
        assert!(!state.action_payloads);
        assert!(!state.pending_actions);
    }

    #[test]
    fn unsupported_recovery_state_slot_values_unsupported() {
        let state = UnsupportedRecoveryState::slot_values_unsupported();
        assert!(state.slot_values);
        assert!(!state.slot_taint);
        assert!(!state.action_payloads);
        assert!(!state.pending_actions);
    }

    #[test]
    fn unsupported_recovery_state_event_slot_taint_unsupported() {
        let state = UnsupportedRecoveryState::event_slot_taint_unsupported();
        assert!(!state.slot_values);
        assert!(state.slot_taint);
        assert!(!state.action_payloads);
        assert!(!state.pending_actions);
    }

    #[test]
    fn unsupported_recovery_state_pending_actions_unsupported() {
        let state = UnsupportedRecoveryState::pending_actions_unsupported();
        assert!(!state.slot_values);
        assert!(!state.slot_taint);
        assert!(!state.action_payloads);
        assert!(state.pending_actions);
    }

    #[test]
    fn unsupported_recovery_state_union() {
        let a = UnsupportedRecoveryState::slot_values_unsupported();
        let b = UnsupportedRecoveryState::pending_actions_unsupported();
        let union = UnsupportedRecoveryState::union(a, b);
        assert!(union.slot_values);
        assert!(!union.slot_taint);
        assert!(!union.action_payloads);
        assert!(union.pending_actions);
    }

    // =========================================================================
    // summarize_recovery_events tests
    // =========================================================================

    #[test]
    fn summarize_recovery_events_with_run_finished() {
        let run = RunId::new(100);
        let workflow = sample_digest(7);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: SlotIdx::new(0),
                attempt: 1,
            },
        ];
        let result = summarize_recovery_events(&events);
        assert!(result.is_ok());
        let hydration = result.unwrap();
        let summary = hydration.summary();
        assert_eq!(summary.run, run);
        assert_eq!(summary.workflow, Some(workflow));
        assert_eq!(summary.steps_started, 1);
        assert_eq!(
            summary.terminal,
            Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(0)
            })
        );
    }

    #[test]
    fn summarize_recovery_events_with_run_cancelled() {
        let run = RunId::new(101);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(8),
            },
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
                attempt: 1,
                reason: None,
            },
        ];
        let result = summarize_recovery_events(&events);
        assert!(result.is_ok());
        let hydration = result.unwrap();
        let summary = hydration.summary();
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Cancelled));
    }

    #[test]
    fn summarize_recovery_events_with_run_failed() {
        let run = RunId::new(102);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(9),
            },
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(1),
                attempt: 1,
            },
        ];
        let result = summarize_recovery_events(&events);
        assert!(result.is_ok());
        let hydration = result.unwrap();
        let summary = hydration.summary();
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Failed));
    }

    #[test]
    fn summarize_recovery_events_counts_all_event_types() {
        let run = RunId::new(103);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(10),
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
                output: SlotIdx::new(0),
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
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(5),
                slot: SlotIdx::new(0),
                value: None,
                extra: None,
                attempt: 1,
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(1),
                attempt: 1,
            },
        ];
        let result = summarize_recovery_events(&events);
        assert!(result.is_ok());
        let hydration = result.unwrap();
        let summary = hydration.summary();
        assert_eq!(summary.steps_started, 1);
        assert_eq!(summary.steps_succeeded, 1);
        assert_eq!(summary.actions_scheduled, 1);
        assert_eq!(summary.actions_resolved, 1);
        assert_eq!(summary.slots_written, 1);
        assert_eq!(summary.slots_written, 1);
        assert_eq!(summary.suspensions, 1);
    }

    // =========================================================================
    // recover_run_admission_from_events tests
    // =========================================================================

    #[test]
    fn recover_run_admission_from_events_finds_latest() {
        let run = RunId::new(200);
        let first = sample_digest(1);
        let latest = sample_digest(2);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(9),
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(1),
                artifact_digest: first,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Relaxed,
            },
            JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(2),
                artifact_digest: latest,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
            },
        ];
        let admission = recover_run_admission_from_events(&events);
        assert!(admission.is_some());
        let admission = admission.unwrap();
        assert_eq!(admission.run_id, run);
        assert_eq!(admission.artifact_digest, latest);
        assert_eq!(admission.policy, RuntimePolicy::Strict);
    }

    #[test]
    fn recover_run_admission_from_events_returns_none_when_no_admission() {
        let run = RunId::new(201);
        let events = vec![JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(9),
        }];
        let admission = recover_run_admission_from_events(&events);
        assert!(admission.is_none());
    }

    #[test]
    fn recover_run_admission_from_events_returns_none_for_empty() {
        let events: Vec<JournalEvent> = vec![];
        let admission = recover_run_admission_from_events(&events);
        assert!(admission.is_none());
    }

    // =========================================================================
    // recover_runtime_frame_seed_from_events tests
    // =========================================================================

    #[test]
    fn recover_runtime_frame_seed_from_events_reconstructs_pc() {
        let run = RunId::new(300);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(11),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(3),
                attempt: 1,
            },
        ];
        let seed = recover_runtime_frame_seed_from_events(&events);
        assert!(seed.is_ok());
        let seed = seed.unwrap();
        assert_eq!(seed.pc, StepIdx::new(3));
        assert_eq!(seed.step_count, 4);
    }

    #[test]
    fn recover_runtime_frame_seed_from_events_no_steps() {
        let run = RunId::new(301);
        let events = vec![JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(12),
        }];
        let seed = recover_runtime_frame_seed_from_events(&events);
        assert!(seed.is_ok());
        let seed = seed.unwrap();
        assert_eq!(seed.step_count, 0);
        assert_eq!(seed.first_step, StepIdx::ZERO);
        assert_eq!(seed.pc, StepIdx::ZERO);
    }

    #[test]
    fn recover_runtime_frame_seed_from_events_with_asking_step() {
        let run = RunId::new(302);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(13),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
        ];
        let seed = recover_runtime_frame_seed_from_events(&events);
        assert!(seed.is_ok());
        let seed = seed.unwrap();
        assert!(seed.steps.iter().any(|e| {
            e.step == StepIdx::new(0)
                && e.state == crate::recovery::types::RecoveredStepState::Asking
        }));
    }

    #[test]
    fn recover_runtime_frame_seed_from_events_with_waiting_step() {
        let run = RunId::new(303);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: sample_digest(14),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
                attempt: 1,
            },
        ];
        let seed = recover_runtime_frame_seed_from_events(&events);
        assert!(seed.is_ok());
        let seed = seed.unwrap();
        assert!(seed.steps.iter().any(|e| {
            e.step == StepIdx::new(1)
                && e.state == crate::recovery::types::RecoveredStepState::Waiting
        }));
    }

    #[test]
    fn recover_runtime_frame_seed_from_events_empty_returns_error() {
        let events: Vec<JournalEvent> = vec![];
        let result = recover_runtime_frame_seed_from_events(&events);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RecoveryError::NoRecoveryData { .. }
        ));
    }

    // =========================================================================
    // apply_summary_event tests
    // =========================================================================

    #[test]
    fn apply_summary_event_run_accepted_sets_workflow() {
        let mut summary = RecoveryRuntimeSummary {
            run: RunId::new(400),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let workflow = sample_digest(15);
        let event = JournalEvent::RunAccepted {
            run: RunId::new(400),
            seq: EventSeq::new(0),
            workflow,
        };
        apply_summary_event(&mut summary, &event);
        assert_eq!(summary.workflow, Some(workflow));
    }

    #[test]
    fn apply_summary_event_run_admission_is_no_op() {
        let mut summary = RecoveryRuntimeSummary {
            run: RunId::new(401),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let event = JournalEvent::RunAdmission {
            run: RunId::new(401),
            seq: EventSeq::new(0),
            artifact_digest: sample_digest(16),
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        };
        apply_summary_event(&mut summary, &event);
        assert_eq!(summary.terminal, None);
    }

    #[test]
    fn apply_summary_event_retry_scheduled_increments_suspensions() {
        let mut summary = RecoveryRuntimeSummary {
            run: RunId::new(402),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let event = JournalEvent::RetryScheduledEvent {
            run: RunId::new(402),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        apply_summary_event(&mut summary, &event);
        assert_eq!(summary.suspensions, 1);
    }

    #[test]
    fn apply_summary_event_run_resumed_is_no_op() {
        use chrono::Utc;
        let mut summary = RecoveryRuntimeSummary {
            run: RunId::new(403),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let event = JournalEvent::RunResumed {
            run: RunId::new(403),
            seq: EventSeq::ZERO,
            timestamp: Utc::now(),
        };
        apply_summary_event(&mut summary, &event);
        assert_eq!(summary.suspensions, 0);
    }

    #[test]
    fn apply_summary_event_run_retried_is_no_op() {
        use chrono::Utc;
        let mut summary = RecoveryRuntimeSummary {
            run: RunId::new(404),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let event = JournalEvent::RunRetried {
            run: RunId::new(404),
            seq: EventSeq::ZERO,
            timestamp: Utc::now(),
        };
        apply_summary_event(&mut summary, &event);
        assert_eq!(summary.suspensions, 0);
    }

    #[test]
    fn apply_summary_event_run_answered_is_no_op() {
        use chrono::Utc;
        let mut summary = RecoveryRuntimeSummary {
            run: RunId::new(405),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let event = JournalEvent::RunAnswered {
            run: RunId::new(405),
            seq: EventSeq::ZERO,
            slot_idx: SlotIdx::new(0),
            answer: ConstValue::Null,
            timestamp: Utc::now(),
        };
        apply_summary_event(&mut summary, &event);
        assert_eq!(summary.suspensions, 0);
    }

    // =========================================================================
    // ActionReplayTracker tests
    // =========================================================================

    #[test]
    fn action_replay_tracker_new_is_empty() {
        let tracker = ActionReplayTracker::new();
        let action = ActionId::new(1);
        let step = StepIdx::new(1);
        assert!(!tracker.is_resolved(action, step));
    }

    #[test]
    fn action_replay_tracker_mark_completed() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(2);
        let step = StepIdx::new(3);
        tracker.mark_completed(action, step);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn action_replay_tracker_mark_failed() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(4);
        let step = StepIdx::new(5);
        tracker.mark_failed(action, step);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn action_replay_tracker_separate_tracks() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(6);
        let step = StepIdx::new(7);
        tracker.mark_completed(action, step);
        assert!(tracker.is_resolved(action, step));
        let other_action = ActionId::new(7);
        assert!(!tracker.is_resolved(other_action, step));
        assert!(!tracker.is_resolved(action, StepIdx::new(8)));
    }

    // =========================================================================
    // replay_events with PRE-001 (attempt filtering) tests
    // =========================================================================

    #[test]
    fn replay_events_skips_old_attempt_events() {
        let run = RunId::new(500);
        let events = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 2,
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert!(result.is_ok());
        let replayed = result.unwrap();
        assert_eq!(replayed.len(), 2);
    }

    #[test]
    fn replay_events_tracks_action_completion() {
        let run = RunId::new(501);
        let action = ActionId::new(1);
        let step = StepIdx::new(0);
        let events = vec![
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(1),
                step,
                action,
                attempt: 1,
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert!(result.is_ok());
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn replay_events_empty_input_succeeds() {
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&[], &mut tracker, &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // =========================================================================
    // is_terminal_event tests
    // =========================================================================

    #[test]
    fn is_terminal_event_run_finished_is_true() {
        let event = JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            result: SlotIdx::new(0),
            attempt: 1,
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_run_cancelled_is_true() {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            attempt: 1,
            reason: None,
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_run_failed_is_true() {
        let event = JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            attempt: 1,
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_run_accepted_is_false() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: sample_digest(1),
        };
        assert!(!is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_step_started_is_false() {
        let event = JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        assert!(!is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_action_scheduled_is_false() {
        let event = JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        };
        assert!(!is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_slot_written_is_false() {
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        };
        assert!(!is_terminal_event(&event));
    }

    // =========================================================================
    // extract_terminal tests
    // =========================================================================

    #[test]
    fn extract_terminal_empty_events() {
        let events: Vec<JournalEvent> = vec![];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_none());
    }

    #[test]
    fn extract_terminal_run_cancelled() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: sample_digest(1),
            },
            JournalEvent::RunCancelled {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                attempt: 1,
                reason: None,
            },
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some());
        assert!(matches!(terminal, Some(JournalEvent::RunCancelled { .. })));
    }

    #[test]
    fn extract_terminal_run_failed() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: sample_digest(1),
            },
            JournalEvent::RunFailedEvent {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                attempt: 1,
            },
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some());
        assert!(matches!(
            terminal,
            Some(JournalEvent::RunFailedEvent { .. })
        ));
    }

    #[test]
    fn extract_terminal_ignores_old_attempt_terminals() {
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
            JournalEvent::RunFinished {
                run: RunId::new(1),
                seq: EventSeq::new(2),
                result: SlotIdx::new(1),
                attempt: 2,
            },
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some());
        assert!(matches!(
            terminal,
            Some(JournalEvent::RunFinished { attempt: 2, .. })
        ));
    }

    #[test]
    fn extract_terminal_returns_last_terminal_in_sequence() {
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
            JournalEvent::RunCancelled {
                run: RunId::new(1),
                seq: EventSeq::new(2),
                attempt: 1,
                reason: None,
            },
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some());
        assert!(matches!(terminal, Some(JournalEvent::RunCancelled { .. })));
    }

    // ══ vb-hbav B15: RecoveryError exhaustiveness compile-time check ════════
    #[test]
    fn recovery_error_match_covers_all_variants() {
        fn _exhaustive_match(e: &RecoveryError) -> &'static str {
            match e {
                RecoveryError::Journal(_) => "journal",
                RecoveryError::WorkflowSourceDigestMismatch { .. } => {
                    "workflow_source_digest_mismatch"
                }
                RecoveryError::CompiledIrDigestMismatch { .. } => "compiled_ir_digest_mismatch",
                RecoveryError::ActionAbiMismatch { .. } => "action_abi_mismatch",
                RecoveryError::PolicyDigestMismatch { .. } => "policy_digest_mismatch",
                RecoveryError::NonIdempotentActionBlocked { .. } => "non_idempotent_action_blocked",
                RecoveryError::ReplayDivergence { .. } => "replay_divergence",
                RecoveryError::SlotTaintReadFailed { .. } => "slot_taint_read_failed",
                RecoveryError::CorruptSlotTaint { .. } => "corrupt_slot_taint",
                RecoveryError::NoRecoveryData { .. } => "no_recovery_data",
                RecoveryError::CorruptSnapshot { .. } => "corrupt_snapshot",
                RecoveryError::TerminalStateMismatch { .. } => "terminal_state_mismatch",
                RecoveryError::FrameDimensionOverflow { .. } => "frame_dimension_overflow",
            }
        }
        let _ = _exhaustive_match;
    }
}
