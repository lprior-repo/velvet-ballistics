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
        replay::core::{
            extract_terminal, is_terminal_event, recover_snapshot_plus_tail, replay_events,
        },
    };
    use crate::{DIGEST_BYTES, EventSeq, JournalEvent};
    use proptest::prelude::*;
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

    fn assert_sequence_divergence(
        result: Result<Vec<JournalEvent>, RecoveryError>,
        expected_detail: &str,
    ) {
        match result {
            Err(RecoveryError::ReplayDivergence { step, detail }) => {
                assert_eq!(step, StepIdx::ZERO);
                assert_eq!(detail, expected_detail);
            }
            other => panic!("expected ReplayDivergence, got {other:?}"),
        }
    }

    fn assert_sequence_pair_diverges(first: u64, second: u64, expected_detail: &str) {
        let run = RunId::new(54);
        let events = [
            make_run_accepted(run, first),
            make_run_accepted(run, second),
        ];
        let result = replay_events(&events, &mut ActionReplayTracker::new(), &[]);
        assert_sequence_divergence(result, expected_detail);
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
    fn replay_events_keeps_singleton_and_max_adjacent_sequences_valid() {
        let run = RunId::new(54);
        let singleton = [make_run_accepted(run, u64::MAX)];
        let adjacent = [
            make_run_accepted(run, u64::MAX - 1),
            make_run_accepted(run, u64::MAX),
        ];

        assert_eq!(
            replay_events(&singleton, &mut ActionReplayTracker::new(), &[]).ok(),
            Some(singleton.to_vec())
        );
        assert_eq!(
            replay_events(&adjacent, &mut ActionReplayTracker::new(), &[]).ok(),
            Some(adjacent.to_vec())
        );
    }

    #[test]
    fn replay_events_rejects_max_successor_before_tracker_mutation() {
        let run = RunId::new(54);
        let action = ActionId::new(8);
        let step = StepIdx::new(3);
        let event = |seq| JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            action,
            attempt: 1,
        };
        let events = [event(u64::MAX), event(u64::MAX)];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert_sequence_divergence(
            result,
            "journal sequence overflow after 18446744073709551615",
        );
        assert!(!tracker.is_resolved(action, step));
        assert_sequence_pair_diverges(
            u64::MAX,
            0,
            "journal sequence overflow after 18446744073709551615",
        );
    }

    #[test]
    fn replay_events_preserves_ordinary_noncontiguous_error_details() {
        assert_sequence_pair_diverges(0, 2, "journal sequence violation: expected 1, found 2");
        assert_sequence_pair_diverges(2, 1, "journal sequence violation: expected 3, found 1");
        assert_sequence_pair_diverges(1, 1, "journal sequence violation: expected 2, found 1");
    }

    #[test]
    fn snapshot_tail_replay_uses_checked_sequence_validation() {
        let run = RunId::new(54);
        let snapshot = crate::recovery::types::RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            slots: Vec::new(),
            taint: Vec::new(),
        };
        let tail = [
            make_run_accepted(run, u64::MAX - 1),
            make_run_accepted(run, u64::MAX),
            make_run_accepted(run, u64::MAX),
        ];
        let result =
            recover_snapshot_plus_tail(&snapshot, &tail, &mut ActionReplayTracker::new(), &[]);

        assert_sequence_divergence(
            result,
            "journal sequence overflow after 18446744073709551615",
        );
    }

    proptest! {
        #[test]
        fn prop_replay_accepts_adjacent_sequence_pairs(pair in any::<u64>().prop_filter_map(
            "sequence has a representable successor",
            |start| start.checked_add(1).map(|next| (start, next)),
        )) {
            let (start, next) = pair;
            let run = RunId::new(54);
            let events = [make_run_accepted(run, start), make_run_accepted(run, next)];

            prop_assert_eq!(
                replay_events(&events, &mut ActionReplayTracker::new(), &[]).ok(),
                Some(events.to_vec())
            );
        }
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
