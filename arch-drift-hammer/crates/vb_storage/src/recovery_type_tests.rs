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
mod recovery_type_tests {
    use crate::recovery::{
        ActionReplayTracker, DigestCheck, RecoveredPendingAction, RecoveredStepEntry,
        RecoveredStepState, RecoveryRuntimeSummary, RecoveryTerminalState,
        UnsupportedRecoveryState, RecoveryFrameSeed, RecoveredSlotEntry, RecoveryHydration,
    };
    use crate::EventSeq;
    use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx, Taint};

    #[test]
    fn action_replay_tracker_new_is_empty() {
        let tracker = ActionReplayTracker::new();
        assert!(!tracker.is_resolved(ActionId::new(0), StepIdx::new(0)));
    }

    #[test]
    fn action_replay_tracker_default_is_empty() {
        let tracker = ActionReplayTracker::default();
        assert!(!tracker.is_resolved(ActionId::new(0), StepIdx::new(0)));
    }

    #[test]
    fn action_replay_tracker_marks_and_checks_completed() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(1);
        let step = StepIdx::new(2);
        assert!(!tracker.is_resolved(action, step));
        tracker.mark_completed(action, step);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn action_replay_tracker_marks_and_checks_failed() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(3);
        let step = StepIdx::new(4);
        assert!(!tracker.is_resolved(action, step));
        tracker.mark_failed(action, step);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn action_replay_tracker_different_step_not_resolved() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(5);
        tracker.mark_completed(action, StepIdx::new(0));
        assert!(tracker.is_resolved(action, StepIdx::new(0)));
        assert!(!tracker.is_resolved(action, StepIdx::new(1)));
    }

    #[test]
    fn action_replay_tracker_different_action_not_resolved() {
        let mut tracker = ActionReplayTracker::new();
        tracker.mark_completed(ActionId::new(7), StepIdx::new(0));
        assert!(!tracker.is_resolved(ActionId::new(8), StepIdx::new(0)));
    }

    #[test]
    fn digest_check_variants_exist() {
        let _source = DigestCheck::WorkflowSourceOnly;
        let _ir = DigestCheck::WorkflowAndIr;
        let _full = DigestCheck::Full;
    }

    #[test]
    fn terminal_state_finished_carries_result_slot() {
        let state = RecoveryTerminalState::Finished {
            result: SlotIdx::new(5),
        };
        match state {
            RecoveryTerminalState::Finished { result } => assert_eq!(result, SlotIdx::new(5)),
            _ => panic!("expected Finished"),
        }
    }

    #[test]
    fn recovered_step_state_variants_exist() {
        let _running = RecoveredStepState::Running;
        let _succeeded = RecoveredStepState::Succeeded;
        let _failed = RecoveredStepState::Failed;
        let _waiting = RecoveredStepState::Waiting;
        let _asking = RecoveredStepState::Asking;
    }

    #[test]
    fn recovered_step_entry_carries_step_and_state() {
        let entry = RecoveredStepEntry {
            step: StepIdx::new(3),
            state: RecoveredStepState::Succeeded,
        };
        assert_eq!(entry.step, StepIdx::new(3));
        assert_eq!(entry.state, RecoveredStepState::Succeeded);
    }

    #[test]
    fn recovered_slot_entry_carries_slot_value_taint() {
        let entry = RecoveredSlotEntry {
            slot: SlotIdx::new(1),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
        };
        assert_eq!(entry.slot, SlotIdx::new(1));
        assert_eq!(entry.value, SlotValue::I64(42));
        assert_eq!(entry.taint, Taint::Clean);
    }

    #[test]
    fn recovered_pending_action_carries_step_and_action() {
        let pending = RecoveredPendingAction {
            step: StepIdx::new(7),
            action: ActionId::new(99),
        };
        assert_eq!(pending.step, StepIdx::new(7));
        assert_eq!(pending.action, ActionId::new(99));
    }

    #[test]
    fn unsupported_recovery_state_supported_is_fully_false() {
        let s = UnsupportedRecoveryState::SUPPORTED;
        assert!(!s.slot_values);
        assert!(!s.slot_taint);
        assert!(!s.action_payloads);
        assert!(!s.pending_actions);
    }

    #[test]
    fn unsupported_recovery_state_event_slot_taint_unsupported() {
        let s = UnsupportedRecoveryState::event_slot_taint_unsupported();
        assert!(!s.slot_values);
        assert!(s.slot_taint);
        assert!(!s.action_payloads);
        assert!(!s.pending_actions);
    }

    #[test]
    fn unsupported_recovery_state_slot_values_unsupported() {
        let s = UnsupportedRecoveryState::slot_values_unsupported();
        assert!(s.slot_values);
        assert!(!s.slot_taint);
        assert!(!s.action_payloads);
        assert!(!s.pending_actions);
    }

    #[test]
    fn unsupported_recovery_state_pending_actions_unsupported() {
        let s = UnsupportedRecoveryState::pending_actions_unsupported();
        assert!(!s.slot_values);
        assert!(!s.slot_taint);
        assert!(!s.action_payloads);
        assert!(s.pending_actions);
    }

    #[test]
    fn unsupported_recovery_state_union_combines_correctly() {
        let a = UnsupportedRecoveryState::slot_values_unsupported();
        let b = UnsupportedRecoveryState::event_slot_taint_unsupported();
        let c = a.union(b);
        assert!(c.slot_values);
        assert!(c.slot_taint);
        assert!(!c.action_payloads);
        assert!(!c.pending_actions);
    }

    #[test]
    fn recovery_hydration_summary_variant() {
        let summary = RecoveryRuntimeSummary {
            run: RunId::new(1),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(10),
            workflow: None,
            steps_started: 5,
            steps_succeeded: 4,
            actions_scheduled: 3,
            actions_resolved: 2,
            suspensions: 1,
            slots_written: 0,
            terminal: None,
        };
        let hydration = RecoveryHydration::Summary(summary);
        let s = hydration.summary();
        assert_eq!(s.run, RunId::new(1));
        assert_eq!(s.steps_started, 5);
    }

    #[test]
    fn recovery_frame_seed_has_expected_structure() {
        let seed = RecoveryFrameSeed {
            summary: RecoveryRuntimeSummary {
                run: RunId::new(2),
                first_seq: EventSeq::new(0),
                last_seq: EventSeq::new(3),
                workflow: None,
                steps_started: 2,
                steps_succeeded: 1,
                actions_scheduled: 1,
                actions_resolved: 0,
                suspensions: 0,
                slots_written: 0,
                terminal: None,
            },
            first_step: StepIdx::new(0),
            step_count: 2,
            slot_count: 1,
            pc: StepIdx::new(1),
            steps: vec![],
            slots: vec![],
            pending_actions: vec![],
            unsupported: UnsupportedRecoveryState::SUPPORTED,
        };
        assert_eq!(seed.step_count, 2);
        assert_eq!(seed.slot_count, 1);
        assert_eq!(seed.pc, StepIdx::new(1));
    }
}
