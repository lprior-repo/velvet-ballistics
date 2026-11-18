#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod replay_core_tests {
    use crate::recovery::{
        ActionReplayTracker, RecoveryError,
        replay::core::{extract_terminal, is_terminal_event, replay_events},
    };
    use crate::{DIGEST_BYTES, EventSeq, JournalEvent};
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};

    fn make_step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    fn make_run_accepted(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        }
    }

    fn make_run_finished(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(seq),
            result: SlotIdx::new(0),
            attempt: 1,
        }
    }

    fn make_run_cancelled(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
            reason: None,
        }
    }

    fn make_run_failed(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
        }
    }

    #[test]
    fn is_terminal_event_identifies_run_finished() {
        let event = make_run_finished(RunId::new(1), 0);
        assert!(is_terminal_event(&event), "RunFinished should be terminal");
    }

    #[test]
    fn is_terminal_event_identifies_run_cancelled() {
        let event = make_run_cancelled(RunId::new(1), 0);
        assert!(is_terminal_event(&event), "RunCancelled should be terminal");
    }

    #[test]
    fn is_terminal_event_identifies_run_failed_event() {
        let event = make_run_failed(RunId::new(1), 0);
        assert!(
            is_terminal_event(&event),
            "RunFailedEvent should be terminal"
        );
    }

    #[test]
    fn is_terminal_event_rejects_non_terminal_events() {
        let event = make_run_accepted(RunId::new(1), 0);
        assert!(
            !is_terminal_event(&event),
            "RunAccepted should not be terminal"
        );

        let event = make_step_started(RunId::new(1), 0, 0);
        assert!(
            !is_terminal_event(&event),
            "StepStarted should not be terminal"
        );
    }

    #[test]
    fn extract_terminal_returns_terminal_when_present() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 0),
            make_step_started(run, 1, 0),
            make_run_finished(run, 2),
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some(), "should find terminal event");
        assert!(is_terminal_event(terminal.unwrap()));
    }

    #[test]
    fn extract_terminal_returns_none_when_no_terminal() {
        let events = vec![make_step_started(RunId::new(1), 0, 0)];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_none(), "should return None when no terminal");
    }

    #[test]
    fn extract_terminal_returns_last_terminal_with_highest_attempt() {
        let run = RunId::new(2);
        let events = vec![
            make_run_accepted(run, 0),
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
                attempt: 1,
                reason: None,
            },
            JournalEvent::RunRetried {
                run,
                seq: EventSeq::new(2),
                timestamp: chrono::Utc::now(),
            },
            make_step_started(run, 2, 0),
            make_run_finished(run, 3),
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some(), "should find terminal in latest attempt");
        let found = terminal.unwrap();
        assert!(
            matches!(found, JournalEvent::RunFinished { .. }),
            "should return terminal from latest attempt"
        );
    }

    #[test]
    fn replay_events_detects_non_contiguous_sequences() {
        let run = RunId::new(3);
        let events = vec![
            make_run_accepted(run, 0),
            // Missing seq 1
            make_step_started(run, 2, 0),
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "should reject non-contiguous sequences, got {:?}",
            result
        );
    }

    #[test]
    fn replay_events_detects_step_ordering_violation() {
        let run = RunId::new(4);
        let events = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(5),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(3),
                attempt: 1,
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "should reject step going backward, got {:?}",
            result
        );
    }

    #[test]
    fn replay_events_blocks_duplicate_action_completion() {
        let run = RunId::new(5);
        let action = ActionId::new(10);
        let step = StepIdx::new(0);
        let events = vec![
            JournalEvent::ActionCompletedEvent {
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
        assert!(
            matches!(result, Err(RecoveryError::NonIdempotentActionBlocked { action: a, step: s })
                if a == action && s == step),
            "should block duplicate action completion, got {:?}",
            result
        );
    }

    #[test]
    fn replay_events_blocks_action_completed_after_failed() {
        let run = RunId::new(6);
        let action = ActionId::new(11);
        let step = StepIdx::new(0);
        let events = vec![
            JournalEvent::ActionFailedEvent {
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
        assert!(
            matches!(
                result,
                Err(RecoveryError::NonIdempotentActionBlocked { .. })
            ),
            "should block action after failure, got {:?}",
            result
        );
    }
}
