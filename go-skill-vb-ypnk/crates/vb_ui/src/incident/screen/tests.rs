#[cfg(test)]
mod tests {
    use super::*;
    use crate::incident::repair::{RepairAction, RepairKind};

    #[test]
    fn test_process_failure_action_failed_event() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::ActionFailedEvent {
            run: RunId::new(42),
            seq: EventSeq::new(5),
            step: StepIdx::new(3),
            action: ActionId::new(10),
        };
        let inc = IncidentScreen::process_failure(&event).unwrap();
        assert_eq!(inc.run_id, 42);
        assert_eq!(inc.step_id, Some(3));
        assert_eq!(inc.incident_type, IncidentType::ActionFailure);
        assert_eq!(inc.severity, IncidentSeverity::Major);
        assert!(!inc.replay_safe);
        assert_eq!(inc.side_effect_certainty, SideEffectCertainty::Unknown);
        assert_eq!(inc.timeline.len(), 1);
        assert_eq!(
            inc.timeline.first().map(|t| t.event_kind),
            Some(TimelineEventKind::FailureObserved)
        );
        assert_eq!(inc.timeline.first().map(|t| t.seq), Some(0));
        assert!(inc.error_message.contains("Action 10"));
    }

    #[test]
    fn test_process_failure_run_failed_event() {
        use vb_core::RunId;
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::RunFailedEvent {
            run: RunId::new(99),
            seq: EventSeq::new(7),
        };
        let inc = IncidentScreen::process_failure(&event).unwrap();
        assert_eq!(inc.run_id, 99);
        assert!(inc.step_id.is_none());
        assert_eq!(inc.incident_type, IncidentType::BlockedReconciliation);
        assert_eq!(inc.severity, IncidentSeverity::Critical);
        assert!(!inc.replay_safe);
        assert_eq!(inc.timeline.len(), 1);
        assert!(inc.error_message.contains("99"));
    }

    #[test]
    fn test_process_failure_non_failure_event_returns_none() {
        use vb_core::{RunId, WorkflowDigest};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        assert!(IncidentScreen::process_failure(&event).is_none());
    }

    #[test]
    fn test_process_failure_step_started_returns_none() {
        use vb_core::{RunId, StepIdx};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        assert!(IncidentScreen::process_failure(&event).is_none());
    }

    #[test]
    fn test_process_failure_run_finished_returns_none() {
        use vb_core::{RunId, SlotIdx};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            result: SlotIdx::new(0),
        };
        assert!(IncidentScreen::process_failure(&event).is_none());
    }

    #[test]
    fn test_process_failure_action_completed_returns_none() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::ActionCompletedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            action: ActionId::new(5),
        };
        assert!(IncidentScreen::process_failure(&event).is_none());
    }

    #[test]
    fn test_process_run_failure_creates_incident() {
        let mut screen = IncidentScreen::new();
        let idx = screen.process_run_failure(
            42,
            Some("step-fetch"),
            FailureCode::ActionTimeout,
            "timed out after 30s",
        );
        assert_eq!(idx, 0);
        assert_eq!(screen.active_count(), 1);
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.failure_code, FailureCode::ActionTimeout);
        assert_eq!(detail.step_name.as_deref(), Some("step-fetch"));
        assert!(detail.replay_safe);
        assert_eq!(detail.side_effect_certainty, SideEffectCertainty::None);
        assert_eq!(detail.timeline.len(), 1);
        assert_eq!(
            detail.timeline.first().map(|t| t.event_kind),
            Some(TimelineEventKind::FailureObserved)
        );
        assert_eq!(detail.timeline.first().map(|t| t.seq), Some(0));
    }

    #[test]
    fn test_process_run_failure_assigns_incrementing_ids() {
        let mut screen = IncidentScreen::new();
        let i1 = screen.process_run_failure(1, None, FailureCode::ActionTimeout, "a");
        let i2 = screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        assert_eq!(i1, 0);
        assert_eq!(i2, 1);
        let d1 = screen.get_failure_detail(0).unwrap();
        let d2 = screen.get_failure_detail(1).unwrap();
        assert_ne!(d1.failure_code, d2.failure_code);
    }

    #[test]
    fn test_process_run_failure_taint_leak_is_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::TaintLeak, "secret leaked");
        assert_eq!(screen.critical_count(), 1);
        assert_eq!(
            screen.incidents().first().unwrap().incident_type,
            IncidentType::SecretLeak
        );
    }

    #[test]
    fn test_process_run_failure_step_panicked_is_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(11, None, FailureCode::StepPanicked, "panic");
        assert_eq!(screen.critical_count(), 1);
    }

    #[test]
    fn test_process_run_failure_action_failed_is_unknown_certainty() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(12, None, FailureCode::ActionFailed("db".into()), "db error");
        assert_eq!(
            screen.get_failure_detail(0).unwrap().side_effect_certainty,
            SideEffectCertainty::Unknown
        );
    }

    #[test]
    fn test_process_run_failure_validation_error_is_minor() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(
            13,
            None,
            FailureCode::ValidationError("bad input".into()),
            "bad input",
        );
        assert_eq!(screen.critical_count(), 0);
        assert_eq!(
            screen.get_failure_detail(0).unwrap().side_effect_certainty,
            SideEffectCertainty::None
        );
    }

    #[test]
    fn test_process_run_failure_no_step() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(99, None, FailureCode::ActionTimeout, "timeout");
        assert!(screen.get_failure_detail(0).unwrap().step_name.is_none());
    }

    #[test]
    fn test_process_replay_divergence_creates_critical_incident() {
        let mut screen = IncidentScreen::new();
        let idx = screen.process_replay_divergence(100, "value-A", "value-B");
        assert_eq!(idx, 0);
        assert_eq!(screen.active_count(), 1);
        assert_eq!(screen.critical_count(), 1);
        assert_eq!(
            screen.incidents().first().unwrap().incident_type,
            IncidentType::ReplayDivergence
        );
    }

    #[test]
    fn test_process_replay_divergence_has_two_timeline_entries() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(100, "value-A", "value-B");
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.timeline.len(), 2);
        assert_eq!(
            detail.timeline.first().map(|t| t.event_kind),
            Some(TimelineEventKind::FailureObserved)
        );
        assert_eq!(
            detail.timeline.get(1).map(|t| t.event_kind),
            Some(TimelineEventKind::ReplayDivergence)
        );
        assert_eq!(detail.timeline.first().map(|t| t.seq), Some(0));
        assert_eq!(detail.timeline.get(1).map(|t| t.seq), Some(1));
    }

    #[test]
    fn test_process_replay_divergence_not_replay_safe() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(100, "a", "b");
        let detail = screen.get_failure_detail(0).unwrap();
        assert!(!detail.replay_safe);
        assert_eq!(detail.side_effect_certainty, SideEffectCertainty::Unknown);
    }

    #[test]
    fn test_process_replay_divergence_error_message() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(50, "expected-val", "actual-val");
        screen.select(0);
        let selected = screen.selected().unwrap();
        assert!(selected.error_message.contains("expected-val"));
        assert!(selected.error_message.contains("actual-val"));
    }

    #[test]
    fn test_get_failure_detail_returns_none_for_missing() {
        let screen = IncidentScreen::new();
        assert!(screen.get_failure_detail(0).is_none());
    }

    #[test]
    fn test_get_failure_detail_returns_context() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, Some("step-1"), FailureCode::BudgetExceeded, "over");
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.failure_code, FailureCode::BudgetExceeded);
        assert_eq!(detail.step_name.as_deref(), Some("step-1"));
        assert!(detail.replay_safe);
        assert_eq!(detail.error_code, String::from("BudgetExceeded"));
        assert_eq!(detail.run_id, 1);
    }

    #[test]
    fn test_get_failure_detail_step_id_populated() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(5, Some("step-2"), FailureCode::ActionTimeout, "timeout");
        let detail = screen.get_failure_detail(0).unwrap();
        assert!(detail.step_id.is_none());
        assert_eq!(detail.step_name.as_deref(), Some("step-2"));
    }

    #[test]
    fn test_repair_suggestions_for_timeout() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        let suggestions = screen.repair_suggestions(0);
        assert!(!suggestions.is_empty());
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::IncreaseTimeout)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::IncreaseTimeout)
        );
    }

    #[test]
    fn test_repair_suggestions_for_missing_incident() {
        let screen = IncidentScreen::new();
        assert!(screen.repair_suggestions(999).is_empty());
    }

    #[test]
    fn test_repair_suggestions_for_taint_leak() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        let suggestions = screen.repair_suggestions(0);
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::FixSecretLeak)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::FixSecretLeak)
        );
    }

    #[test]
    fn test_incidents_slice_empty() {
        let screen = IncidentScreen::new();
        assert!(screen.incidents().is_empty());
    }

    #[test]
    fn test_active_count_empty() {
        let screen = IncidentScreen::new();
        assert_eq!(screen.active_count(), 0);
    }

    #[test]
    fn test_incidents_after_multiple_failures() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t1");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "t2");
        screen.process_run_failure(3, None, FailureCode::BudgetExceeded, "t3");
        assert_eq!(screen.active_count(), 3);
        let ids: Vec<u64> = screen.incidents().iter().map(|i| i.run_id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_dismiss_reduces_count() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.dismiss(0);
        assert_eq!(screen.active_count(), 1);
        assert_eq!(screen.incidents().first().unwrap().run_id, 2);
    }

    #[test]
    fn test_dismiss_out_of_bounds_is_noop() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.dismiss(5);
        assert_eq!(screen.active_count(), 1);
    }

    #[test]
    fn test_dismiss_all_incidents() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.dismiss(1);
        screen.dismiss(0);
        assert_eq!(screen.active_count(), 0);
    }

    #[test]
    fn test_select_and_selected() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        assert!(screen.selected().is_none());
        screen.select(1);
        assert_eq!(screen.selected().unwrap().run_id, 2);
        screen.select(99);
        assert_eq!(screen.selected().unwrap().run_id, 2);
    }

    #[test]
    fn test_selected_suggestions() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        assert!(screen.selected_suggestions().is_empty());
        screen.select(0);
        assert!(!screen.selected_suggestions().is_empty());
    }

    #[test]
    fn test_timeline_entry_has_timestamp_micros() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        let detail = screen.get_failure_detail(0).unwrap();
        let _micros = detail.timeline.first().unwrap().timestamp_micros;
    }

    #[test]
    fn test_timeline_replay_divergence_seq_numbers() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(10, "x", "y");
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.timeline.len(), 2);
        assert_eq!(detail.timeline.first().map(|t| t.seq), Some(0));
        assert_eq!(detail.timeline.get(1).map(|t| t.seq), Some(1));
    }

    #[test]
    fn test_format_failure_code_variants() {
        assert_eq!(
            format_failure_code(&FailureCode::ActionTimeout),
            "ActionTimeout"
        );
        assert_eq!(
            format_failure_code(&FailureCode::ActionFailed(String::from("db"))),
            "ActionFailed(db)"
        );
        assert_eq!(
            format_failure_code(&FailureCode::BudgetExceeded),
            "BudgetExceeded"
        );
        assert_eq!(
            format_failure_code(&FailureCode::StepPanicked),
            "StepPanicked"
        );
        assert_eq!(
            format_failure_code(&FailureCode::ValidationError(String::from("bad"))),
            "ValidationError(bad)"
        );
        assert_eq!(format_failure_code(&FailureCode::TaintLeak), "TaintLeak");
        assert_eq!(
            format_failure_code(&FailureCode::ReplayDivergence),
            "ReplayDivergence"
        );
        assert_eq!(
            format_failure_code(&FailureCode::Unknown(String::from("x"))),
            "Unknown(x)"
        );
    }

    #[test]
    fn test_incident_type_action_failure_for_timeout() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        assert_eq!(
            screen.incidents().first().unwrap().incident_type,
            IncidentType::ActionFailure
        );
    }

    #[test]
    fn test_incident_type_secret_leak_for_taint() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        assert_eq!(
            screen.incidents().first().unwrap().incident_type,
            IncidentType::SecretLeak
        );
    }

    #[test]
    fn test_incident_type_blocked_reconciliation_for_unknown() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::Unknown("err".into()), "err");
        assert_eq!(
            screen.incidents().first().unwrap().incident_type,
            IncidentType::BlockedReconciliation
        );
    }

    #[test]
    fn test_empty_screen_no_critical() {
        let screen = IncidentScreen::new();
        assert_eq!(screen.critical_count(), 0);
    }

    #[test]
    fn test_empty_screen_get_failure_detail_none() {
        let screen = IncidentScreen::new();
        assert!(screen.get_failure_detail(0).is_none());
        assert!(screen.get_failure_detail(100).is_none());
    }

    #[test]
    fn test_repair_kind_add_retry_backoff_for_action_failed() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionFailed("x".into()), "fail");
        assert!(
            screen
                .repair_suggestions(0)
                .iter()
                .any(|s| s.kind == RepairKind::AddRetryBackoff)
        );
    }

    #[test]
    fn test_repair_kind_reduce_payload_for_validation_error() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ValidationError("v".into()), "v");
        assert!(
            screen
                .repair_suggestions(0)
                .iter()
                .any(|s| s.kind == RepairKind::ReducePayload)
        );
    }

    #[test]
    fn test_repair_kind_pin_idempotency_for_replay_divergence() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(1, "a", "b");
        assert!(
            screen
                .repair_suggestions(0)
                .iter()
                .any(|s| s.kind == RepairKind::PinIdempotency)
        );
    }

    #[test]
    fn test_register_incident_assigns_id() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let mut screen = IncidentScreen::new();
        let event = vb_storage::JournalEvent::ActionFailedEvent {
            run: RunId::new(42),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(1),
        };
        let incident = IncidentScreen::process_failure(&event).unwrap();
        assert_eq!(incident.id, 0);
        let idx = screen.register_incident(incident);
        assert_eq!(idx, 0);
        assert_eq!(screen.active_count(), 1);
        assert_eq!(screen.incidents().first().unwrap().id, 1);
    }

    #[test]
    fn test_register_multiple_increments_ids() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let mut screen = IncidentScreen::new();
        let e1 = vb_storage::JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(1),
        };
        let e2 = vb_storage::JournalEvent::RunFailedEvent {
            run: RunId::new(2),
            seq: EventSeq::new(2),
        };
        let inc1 = IncidentScreen::process_failure(&e1).unwrap();
        let inc2 = IncidentScreen::process_failure(&e2).unwrap();
        screen.register_incident(inc1);
        screen.register_incident(inc2);
        assert_eq!(screen.active_count(), 2);
        assert_eq!(screen.incidents().first().unwrap().id, 1);
        assert_eq!(screen.incidents().get(1).unwrap().id, 2);
    }

    // ---------------------------------------------------------------------------
    // summary_text tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_summary_text_empty() {
        let screen = IncidentScreen::new();
        assert_eq!(screen.summary_text(), "0 incidents");
    }

    #[test]
    fn test_summary_text_single_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        assert_eq!(screen.summary_text(), "1 incidents: 1 Critical");
    }

    #[test]
    fn test_summary_text_single_error() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        assert_eq!(screen.summary_text(), "1 incidents: 1 Error");
    }

    #[test]
    fn test_summary_text_single_minor() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ValidationError("v".into()), "v");
        assert_eq!(screen.summary_text(), "1 incidents: 1 Minor");
    }

    #[test]
    fn test_summary_text_mixed_severities() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        screen.process_run_failure(2, None, FailureCode::ActionTimeout, "timeout");
        screen.process_run_failure(3, None, FailureCode::ValidationError("v".into()), "v");
        let text = screen.summary_text();
        assert!(text.starts_with("3 incidents:"), "actual: {text}");
        assert!(text.contains("1 Critical"), "actual: {text}");
        assert!(text.contains("1 Error"), "actual: {text}");
        assert!(text.contains("1 Minor"), "actual: {text}");
    }

    // ---------------------------------------------------------------------------
    // has_critical tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_has_critical_empty() {
        let screen = IncidentScreen::new();
        assert!(!screen.has_critical());
    }

    #[test]
    fn test_has_critical_true() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        assert!(screen.has_critical());
    }

    #[test]
    fn test_has_critical_false_when_only_major() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        assert!(!screen.has_critical());
    }

    #[test]
    fn test_has_critical_mixed_with_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "leak");
        screen.process_run_failure(3, None, FailureCode::BudgetExceeded, "budget");
        assert!(screen.has_critical());
    }

    #[test]
    fn test_has_critical_after_dismiss() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        assert!(screen.has_critical());
        screen.dismiss(0);
        assert!(!screen.has_critical());
    }

    // ---------------------------------------------------------------------------
    // filter_by_severity tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_filter_by_severity_empty() {
        let screen = IncidentScreen::new();
        let result = screen.filter_by_severity(IncidentSeverity::Critical);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_by_severity_single_match() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        let result = screen.filter_by_severity(IncidentSeverity::Critical);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].run_id, 1);
    }

    #[test]
    fn test_filter_by_severity_no_match() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        let result = screen.filter_by_severity(IncidentSeverity::Critical);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_by_severity_multiple_of_same() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak1");
        screen.process_run_failure(2, None, FailureCode::StepPanicked, "panic");
        screen.process_run_failure(3, None, FailureCode::ActionTimeout, "timeout");
        let result = screen.filter_by_severity(IncidentSeverity::Critical);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_by_severity_major() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "budget");
        screen.process_run_failure(3, None, FailureCode::TaintLeak, "leak");
        let result = screen.filter_by_severity(IncidentSeverity::Major);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_by_severity_minor() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ValidationError("v".into()), "v");
        let result = screen.filter_by_severity(IncidentSeverity::Minor);
        assert_eq!(result.len(), 1);
    }

    // ---------------------------------------------------------------------------
    // Additional tests: defaults, empty list, ordering, severity
    // ---------------------------------------------------------------------------

    #[test]
    fn test_new_defaults_empty_and_no_selection() {
        let screen = IncidentScreen::new();
        assert!(
            screen.incidents().is_empty(),
            "new screen should have no incidents"
        );
        assert_eq!(screen.active_count(), 0, "active_count should be 0");
        assert_eq!(screen.critical_count(), 0, "critical_count should be 0");
        assert!(
            screen.selected().is_none(),
            "no incident should be selected"
        );
        assert!(
            screen.selected_suggestions().is_empty(),
            "no suggestions without selection"
        );
    }

    #[test]
    fn test_default_trait_matches_new() {
        let from_new = IncidentScreen::new();
        let from_default = IncidentScreen::default();
        assert_eq!(from_new.active_count(), from_default.active_count());
        assert_eq!(from_new.critical_count(), from_default.critical_count());
        assert!(from_default.incidents().is_empty());
        assert!(from_default.selected().is_none());
    }

    #[test]
    fn test_empty_list_repair_suggestions_returns_empty() {
        let screen = IncidentScreen::new();
        assert!(screen.repair_suggestions(0).is_empty());
        assert!(screen.repair_suggestions(1).is_empty());
    }

    #[test]
    fn test_empty_list_dismiss_is_noop() {
        let mut screen = IncidentScreen::new();
        screen.dismiss(0);
        screen.dismiss(100);
        assert_eq!(screen.active_count(), 0);
    }

    #[test]
    fn test_empty_list_select_does_not_panic() {
        let mut screen = IncidentScreen::new();
        screen.select(0);
        screen.select(999);
        assert!(screen.selected().is_none());
    }

    #[test]
    fn test_severity_ordering_via_color_dominance() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "critical");
        screen.process_run_failure(2, None, FailureCode::ActionTimeout, "major");
        screen.process_run_failure(3, None, FailureCode::ValidationError("v".into()), "minor");

        let incidents = screen.incidents();
        let [crit_r, ..] = incidents[0].severity.severity_color();
        let [major_r, ..] = incidents[1].severity.severity_color();
        let [minor_r, ..] = incidents[2].severity.severity_color();
        assert!(crit_r >= major_r, "Critical red should dominate Major red");
        assert!(major_r > minor_r, "Major red should dominate Minor red");
    }

    #[test]
    fn test_incidents_preserve_insertion_order() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::ActionTimeout, "a");
        screen.process_run_failure(20, None, FailureCode::TaintLeak, "b");
        screen.process_run_failure(30, None, FailureCode::BudgetExceeded, "c");

        let run_ids: Vec<u64> = screen.incidents().iter().map(|i| i.run_id).collect();
        assert_eq!(
            run_ids,
            vec![10, 20, 30],
            "incidents should maintain insertion order"
        );
    }

    // =========================================================================
    // select_incident, selected_incident, dismiss_selected, detail_sections
    // =========================================================================

    // -- select_incident tests --

    #[test]
    fn test_select_incident_valid_index() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        let result = screen.select_incident(1);
        assert!(result.is_some());
        assert_eq!(result.map(|i| i.run_id), Some(2));
    }

    #[test]
    fn test_select_incident_first_index() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(20, None, FailureCode::TaintLeak, "l");
        let result = screen.select_incident(0);
        assert!(result.is_some());
        assert_eq!(result.map(|i| i.run_id), Some(10));
    }

    #[test]
    fn test_select_incident_out_of_bounds_returns_none() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        let result = screen.select_incident(5);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_incident_empty_screen_returns_none() {
        let mut screen = IncidentScreen::new();
        let result = screen.select_incident(0);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_incident_changes_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.select_incident(0);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(1));
        screen.select_incident(1);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(2));
    }

    #[test]
    fn test_select_incident_reselect_same_index() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(42, None, FailureCode::ActionTimeout, "t");
        let first_run_id = screen.select_incident(0).map(|i| i.run_id);
        let _ = first_run_id;
        let second_run_id = screen.select_incident(0).map(|i| i.run_id);
        assert!(second_run_id.is_some());
    }

    #[test]
    fn test_select_incident_returns_reference_to_selected() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(99, Some("step-x"), FailureCode::ActionTimeout, "timeout");
        let incident = screen.select_incident(0);
        assert!(incident.is_some());
        let inc = incident.map_or(false, |i| {
            i.run_id == 99 && i.step_name.as_deref() == Some("step-x")
        });
        assert!(inc);
    }

    // -- selected_incident tests --

    #[test]
    fn test_selected_incident_none_when_no_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        assert!(screen.selected_incident().is_none());
    }

    #[test]
    fn test_selected_incident_none_on_empty_screen() {
        let screen = IncidentScreen::new();
        assert!(screen.selected_incident().is_none());
    }

    #[test]
    fn test_selected_incident_after_select() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(7, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let selected = screen.selected_incident();
        assert!(selected.is_some());
        assert_eq!(selected.map(|i| i.run_id), Some(7));
        assert_eq!(
            selected.map(|i| i.failure_code.clone()),
            Some(FailureCode::TaintLeak)
        );
    }

    #[test]
    fn test_selected_incident_after_legacy_select() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(3, None, FailureCode::BudgetExceeded, "b");
        screen.select(0);
        let selected = screen.selected_incident();
        assert!(selected.is_some());
        assert_eq!(selected.map(|i| i.run_id), Some(3));
    }

    #[test]
    fn test_selected_incident_after_dismiss_becomes_none() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        assert!(screen.selected_incident().is_some());
        screen.dismiss(0);
        assert!(screen.selected_incident().is_none());
    }

    // -- dismiss_selected tests --

    #[test]
    fn test_dismiss_selected_with_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.select_incident(0);
        let dismissed = screen.dismiss_selected();
        assert!(dismissed);
        assert_eq!(screen.active_count(), 1);
        assert!(screen.selected_incident().is_none());
    }

    #[test]
    fn test_dismiss_selected_no_selection_returns_false() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        let dismissed = screen.dismiss_selected();
        assert!(!dismissed);
        assert_eq!(screen.active_count(), 1);
    }

    #[test]
    fn test_dismiss_selected_empty_screen_returns_false() {
        let mut screen = IncidentScreen::new();
        let dismissed = screen.dismiss_selected();
        assert!(!dismissed);
    }

    #[test]
    fn test_dismiss_selected_clears_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(1);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(2));
        let dismissed = screen.dismiss_selected();
        assert!(dismissed);
        assert!(screen.selected_incident().is_none());
    }

    #[test]
    fn test_dismiss_selected_reduces_count() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(20, None, FailureCode::BudgetExceeded, "b");
        screen.process_run_failure(30, None, FailureCode::TaintLeak, "l");
        screen.select_incident(1);
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 2);
    }

    #[test]
    fn test_dismiss_selected_twice_second_fails() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        assert!(
            !screen.dismiss_selected(),
            "second dismiss with no selection should return false"
        );
    }

    #[test]
    fn test_dismiss_selected_then_select_another() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "l");
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 1);
        screen.select_incident(0);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(2));
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 0);
    }

    #[test]
    fn test_dismiss_selected_all_incidents_one_by_one() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "a");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.process_run_failure(3, None, FailureCode::TaintLeak, "c");
        screen.select_incident(2);
        assert!(screen.dismiss_selected());
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 0);
        assert!(screen.selected_incident().is_none());
    }

    // -- detail_sections tests --

    #[test]
    fn test_detail_sections_no_selection_returns_empty() {
        let screen = IncidentScreen::new();
        let sections = screen.detail_sections();
        assert!(sections.cause.is_none());
        assert!(sections.timeline.is_empty());
        assert!(sections.state_diff.is_empty());
        assert!(sections.repair_suggestions.is_empty());
        assert!(!sections.replay_safe);
        assert_eq!(sections.side_effect_certainty, SideEffectCertainty::None);
    }

    #[test]
    fn test_detail_sections_empty_screen_returns_empty() {
        let screen = IncidentScreen::new();
        let sections = screen.detail_sections();
        assert!(sections.cause.is_none());
    }

    #[test]
    fn test_detail_sections_with_selected_incident_has_cause() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(
            42,
            Some("step-fetch"),
            FailureCode::ActionTimeout,
            "timed out",
        );
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        let c = cause.map_or(false, |v| {
            v.run_id == 42
                && v.error_message.contains("timed out")
                && v.severity == IncidentSeverity::Major
                && v.step_name.as_deref() == Some("step-fetch")
                && v.category == "action"
                && v.failure_code == FailureCode::ActionTimeout
        });
        assert!(c);
    }

    #[test]
    fn test_detail_sections_timeline_entries() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(!sections.timeline.is_empty());
        assert_eq!(sections.timeline.first().map(|e| e.seq), Some(0));
        assert_eq!(
            sections.timeline.first().map(|e| e.event_kind),
            Some(TimelineEventKind::FailureObserved)
        );
        assert!(
            sections
                .timeline
                .first()
                .map_or(false, |e| !e.description.is_empty())
        );
    }

    #[test]
    fn test_detail_sections_replay_divergence_timeline() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(100, "expected-val", "actual-val");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert_eq!(sections.timeline.len(), 2);
        assert_eq!(
            sections.timeline.first().map(|e| e.event_kind),
            Some(TimelineEventKind::FailureObserved)
        );
        assert_eq!(
            sections.timeline.get(1).map(|e| e.event_kind),
            Some(TimelineEventKind::ReplayDivergence)
        );
    }

    #[test]
    fn test_detail_sections_replay_safe_flag() {
        let mut screen = IncidentScreen::new();
        // ActionTimeout is replay_safe
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(sections.replay_safe);

        // TaintLeak is not replay_safe
        screen.dismiss(0);
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let sections2 = screen.detail_sections();
        assert!(!sections2.replay_safe);
    }

    #[test]
    fn test_detail_sections_side_effect_certainty() {
        let mut screen = IncidentScreen::new();
        // ActionTimeout => SideEffectCertainty::None
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert_eq!(sections.side_effect_certainty, SideEffectCertainty::None);

        // ActionFailed => SideEffectCertainty::Unknown
        screen.process_run_failure(2, None, FailureCode::ActionFailed("db".into()), "db error");
        screen.select_incident(1);
        let sections2 = screen.detail_sections();
        assert_eq!(
            sections2.side_effect_certainty,
            SideEffectCertainty::Unknown
        );

        // TaintLeak => SideEffectCertainty::Certain
        screen.process_run_failure(3, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(2);
        let sections3 = screen.detail_sections();
        assert_eq!(
            sections3.side_effect_certainty,
            SideEffectCertainty::Certain
        );
    }

    #[test]
    fn test_detail_sections_repair_suggestions() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(!sections.repair_suggestions.is_empty());
        assert!(
            sections
                .repair_suggestions
                .iter()
                .any(|s| s.kind == RepairKind::IncreaseTimeout)
        );
    }

    #[test]
    fn test_detail_sections_repair_suggestions_taint_leak() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(
            sections
                .repair_suggestions
                .iter()
                .any(|s| s.kind == RepairKind::FixSecretLeak)
        );
    }

    #[test]
    fn test_detail_sections_state_diff_empty_by_default() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(sections.state_diff.is_empty());
    }

    #[test]
    fn test_detail_sections_switching_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let sections1 = screen.detail_sections();
        assert_eq!(sections1.cause.as_ref().map(|c| c.run_id), Some(1));
        assert!(sections1.replay_safe);

        screen.select_incident(1);
        let sections2 = screen.detail_sections();
        assert_eq!(sections2.cause.as_ref().map(|c| c.run_id), Some(2));
        assert!(!sections2.replay_safe);
    }

    #[test]
    fn test_detail_sections_cause_category_matches_failure_code() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        assert_eq!(cause.map(|c| c.category.as_str()), Some("security"));
    }

    #[test]
    fn test_detail_sections_cause_severity_matches() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ValidationError("bad".into()), "v");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        assert_eq!(cause.map(|c| c.severity), Some(IncidentSeverity::Minor));
    }

    #[test]
    fn test_detail_sections_after_dismiss_selected_returns_empty() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let _ = screen.detail_sections();
        assert!(screen.dismiss_selected());
        let sections = screen.detail_sections();
        assert!(sections.cause.is_none());
        assert!(sections.timeline.is_empty());
    }

    #[test]
    fn test_detail_sections_timeline_entry_has_timestamp_micros() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let first = sections.timeline.first();
        assert!(first.is_some());
        // timestamp_micros should be a valid u64 (we just verify it exists)
        let _micros = first.map(|e| e.timestamp_micros);
    }

    #[test]
    fn test_detail_sections_all_failure_codes_produce_cause() {
        let codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed("err".into()),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError("bad".into()),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown("x".into()),
        ];
        for code in &codes {
            let mut screen = IncidentScreen::new();
            screen.process_run_failure(1, Some("step"), code.clone(), "error");
            screen.select_incident(0);
            let sections = screen.detail_sections();
            assert!(
                sections.cause.is_some(),
                "cause should be present for {:?}",
                code
            );
            assert!(
                !sections.repair_suggestions.is_empty(),
                "repair suggestions should exist for {:?}",
                code
            );
        }
    }

    #[test]
    fn test_detail_sections_incident_without_step_name() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        assert!(cause.map_or(false, |c| c.step_name.is_none()));
    }

    #[test]
    fn test_detail_sections_incident_with_step_name() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(
            1,
            Some("deploy-step"),
            FailureCode::BudgetExceeded,
            "budget",
        );
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        assert_eq!(
            cause.map(|c| c.step_name.as_deref()),
            Some(Some("deploy-step"))
        );
    }

    // =========================================================================
    // IncidentCauseView, IncidentTimelineEntry, IncidentSlotDiff construction
    // =========================================================================

    #[test]
    fn test_incident_cause_view_fields() {
        let cause = super::super::types::IncidentCauseView {
            category: String::from("action"),
            failure_code: FailureCode::ActionTimeout,
            error_message: String::from("timed out"),
            severity: IncidentSeverity::Major,
            step_name: Some(String::from("fetch")),
            run_id: 42,
        };
        assert_eq!(cause.category, "action");
        assert_eq!(cause.failure_code, FailureCode::ActionTimeout);
        assert_eq!(cause.error_message, "timed out");
        assert_eq!(cause.severity, IncidentSeverity::Major);
        assert_eq!(cause.step_name.as_deref(), Some("fetch"));
        assert_eq!(cause.run_id, 42);
    }

    #[test]
    fn test_incident_cause_view_clone() {
        let cause = super::super::types::IncidentCauseView {
            category: String::from("security"),
            failure_code: FailureCode::TaintLeak,
            error_message: String::from("leak detected"),
            severity: IncidentSeverity::Critical,
            step_name: None,
            run_id: 1,
        };
        let cloned = cause.clone();
        assert_eq!(cloned.category, cause.category);
        assert_eq!(cloned.failure_code, cause.failure_code);
        assert_eq!(cloned.error_message, cause.error_message);
        assert_eq!(cloned.severity, cause.severity);
        assert_eq!(cloned.step_name, cause.step_name);
        assert_eq!(cloned.run_id, cause.run_id);
    }

    #[test]
    fn test_incident_timeline_entry_fields() {
        let entry = super::super::types::IncidentTimelineEntry {
            seq: 5,
            description: String::from("retry attempted"),
            timestamp_micros: 1_000_000,
            event_kind: TimelineEventKind::RetryAttempted,
        };
        assert_eq!(entry.seq, 5);
        assert_eq!(entry.description, "retry attempted");
        assert_eq!(entry.timestamp_micros, 1_000_000);
        assert_eq!(entry.event_kind, TimelineEventKind::RetryAttempted);
    }

    #[test]
    fn test_incident_timeline_entry_clone() {
        let entry = super::super::types::IncidentTimelineEntry {
            seq: 3,
            description: String::from("failure observed"),
            timestamp_micros: 500_000,
            event_kind: TimelineEventKind::FailureObserved,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.seq, entry.seq);
        assert_eq!(cloned.description, entry.description);
        assert_eq!(cloned.timestamp_micros, entry.timestamp_micros);
        assert_eq!(cloned.event_kind, entry.event_kind);
    }

    #[test]
    fn test_incident_slot_diff_fields() {
        let diff = super::super::types::IncidentSlotDiff {
            slot_index: 7,
            value_before: String::from("old"),
            value_after: String::from("new"),
            change_label: String::from("modified"),
        };
        assert_eq!(diff.slot_index, 7);
        assert_eq!(diff.value_before, "old");
        assert_eq!(diff.value_after, "new");
        assert_eq!(diff.change_label, "modified");
    }

    #[test]
    fn test_incident_slot_diff_unchanged() {
        let diff = super::super::types::IncidentSlotDiff {
            slot_index: 1,
            value_before: String::from("same"),
            value_after: String::from("same"),
            change_label: String::from("unchanged"),
        };
        assert_eq!(diff.value_before, diff.value_after);
        assert_eq!(diff.change_label, "unchanged");
    }

    #[test]
    fn test_incident_slot_diff_clone() {
        let diff = super::super::types::IncidentSlotDiff {
            slot_index: 2,
            value_before: String::from("before"),
            value_after: String::from("after"),
            change_label: String::from("modified"),
        };
        let cloned = diff.clone();
        assert_eq!(cloned.slot_index, diff.slot_index);
        assert_eq!(cloned.value_before, diff.value_before);
        assert_eq!(cloned.value_after, diff.value_after);
        assert_eq!(cloned.change_label, diff.change_label);
    }

    #[test]
    fn test_incident_detail_sections_default_fields() {
        let sections = super::super::types::IncidentDetailSections {
            cause: None,
            timeline: Vec::new(),
            state_diff: Vec::new(),
            repair_suggestions: Vec::new(),
            replay_safe: false,
            side_effect_certainty: SideEffectCertainty::None,
        };
        assert!(sections.cause.is_none());
        assert!(sections.timeline.is_empty());
        assert!(sections.state_diff.is_empty());
        assert!(sections.repair_suggestions.is_empty());
        assert!(!sections.replay_safe);
        assert_eq!(sections.side_effect_certainty, SideEffectCertainty::None);
    }

    #[test]
    fn test_incident_detail_sections_clone() {
        let sections = super::super::types::IncidentDetailSections {
            cause: Some(super::super::types::IncidentCauseView {
                category: String::from("action"),
                failure_code: FailureCode::ActionTimeout,
                error_message: String::from("err"),
                severity: IncidentSeverity::Major,
                step_name: None,
                run_id: 1,
            }),
            timeline: vec![super::super::types::IncidentTimelineEntry {
                seq: 0,
                description: String::from("event"),
                timestamp_micros: 100,
                event_kind: TimelineEventKind::FailureObserved,
            }],
            state_diff: Vec::new(),
            repair_suggestions: Vec::new(),
            replay_safe: true,
            side_effect_certainty: SideEffectCertainty::None,
        };
        let cloned = sections.clone();
        assert!(cloned.cause.is_some());
        assert_eq!(cloned.timeline.len(), 1);
        assert_eq!(cloned.replay_safe, sections.replay_safe);
        assert_eq!(cloned.side_effect_certainty, sections.side_effect_certainty);
    }

    // =========================================================================
    // Interaction tests: select + dismiss + detail round-trips
    // =========================================================================

    #[test]
    fn test_select_then_dismiss_selected_then_select_remaining() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "a");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.process_run_failure(3, None, FailureCode::TaintLeak, "c");

        // Select middle one
        screen.select_incident(1);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(2));

        // Dismiss it
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 2);

        // Select the new first one (was index 0, still index 0)
        screen.select_incident(0);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(1));
    }

    #[test]
    fn test_detail_sections_after_multiple_dismissals() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "a");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "b");

        // Dismiss first via dismiss_selected
        screen.select_incident(0);
        assert!(screen.dismiss_selected());

        // Now only one left, select it
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert_eq!(sections.cause.as_ref().map(|c| c.run_id), Some(2));
        assert!(!sections.replay_safe);
    }

    #[test]
    fn test_select_incident_after_all_dismissed() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 0);

        let result = screen.select_incident(0);
        assert!(result.is_none());
        let sections = screen.detail_sections();
        assert!(sections.cause.is_none());
    }

    #[test]
    fn test_detail_sections_with_mixed_incidents() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(
            1,
            Some("deploy"),
            FailureCode::ActionFailed("net".into()),
            "network error",
        );
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "budget");
        screen.process_replay_divergence(3, "expected", "actual");

        // Select the replay divergence (2 timeline entries)
        screen.select_incident(2);
        let sections = screen.detail_sections();
        assert_eq!(sections.timeline.len(), 2);
        assert!(!sections.replay_safe);

        // Select the budget exceeded
        screen.select_incident(1);
        let sections2 = screen.detail_sections();
        assert!(sections2.replay_safe);
        assert_eq!(sections2.side_effect_certainty, SideEffectCertainty::None);
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// FINDING: summary_text() says "1 incidents" (plural) for a single incident.
    /// This is a grammatical inconsistency - should be "1 incident" (singular).
    /// Not a security issue but a logic error in display formatting.
    #[test]
    fn blackhat_summary_text_singular_vs_plural_grammar_bug() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        let text = screen.summary_text();
        // BUG: says "1 incidents" instead of "1 incident"
        assert!(
            text.contains("1 incidents"),
            "known grammar bug: should be '1 incident' but got '{}'",
            text
        );
    }

    /// FINDING: process_failure() always sets incident.id = 0.
    /// If the returned Incident is used directly without calling register_incident(),
    /// multiple incidents will share id=0, violating the uniqueness invariant.
    #[test]
    fn blackhat_process_failure_returns_zero_id_violating_uniqueness() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let e1 = vb_storage::JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(1),
        };
        let e2 = vb_storage::JournalEvent::RunFailedEvent {
            run: RunId::new(2),
            seq: EventSeq::new(2),
        };
        let inc1 = IncidentScreen::process_failure(&e1).unwrap();
        let inc2 = IncidentScreen::process_failure(&e2).unwrap();
        // Both have id=0, violating the uniqueness contract
        assert_eq!(inc1.id, 0, "process_failure always assigns id=0");
        assert_eq!(
            inc2.id, 0,
            "second incident also gets id=0 - uniqueness violated"
        );
    }

    /// FINDING: register_incident() properly assigns IDs via allocate_id().
    /// This compensates for process_failure's id=0 issue.
    /// Verify the ID allocation starts at 1 (not 0) and increments.
    #[test]
    fn blackhat_register_incident_compensates_for_process_failure_zero_id() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let mut screen = IncidentScreen::new();
        let event = vb_storage::JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(1),
        };
        let incident = IncidentScreen::process_failure(&event).unwrap();
        assert_eq!(incident.id, 0, "process_failure gives id=0");
        let _idx = screen.register_incident(incident);
        // register_incident overwrites id to 1
        assert_eq!(
            screen.incidents().first().map(|i| i.id),
            Some(1),
            "register_incident reassigns id starting from 1"
        );
    }

    /// FINDING: instant_to_micros uses instant.elapsed() which measures time
    /// since the Instant was created. Since `now = Instant::now()` is called
    /// just before conversion, the elapsed time is near-zero microseconds.
    /// This means timestamps are effectively 0 or very small, which may not
    /// be the intended behavior for persistent storage.
    #[test]
    fn blackhat_instant_to_micros_produces_near_zero_values() {
        let now = std::time::Instant::now();
        let micros = IncidentScreen::instant_to_micros(now);
        // elapsed() from a just-created Instant is ~0 microseconds
        assert!(
            micros < 1_000_000,
            "instant_to_micros returns near-zero for just-created Instant: {micros}"
        );
    }

    /// FINDING: allocate_id uses saturating_add which means after u64::MAX
    /// IDs, all subsequent incidents share the same ID. This violates the
    /// uniqueness invariant for extremely long-running sessions.
    #[test]
    fn blackhat_allocate_id_saturates_at_max() {
        let mut screen = IncidentScreen::new();
        // Simulate near-overflow by setting next_incident_id to u64::MAX
        screen.next_incident_id = u64::MAX;
        let id1 = {
            let id = screen.next_incident_id;
            screen.next_incident_id = screen.next_incident_id.saturating_add(1);
            id
        };
        // After saturation, next_incident_id stays at u64::MAX
        assert_eq!(id1, u64::MAX);
        assert_eq!(
            screen.next_incident_id,
            u64::MAX,
            "saturating_add causes ID reuse at u64::MAX"
        );
    }

    /// FINDING: severity_for_code maps BudgetExceeded to Major but the actual
    /// budget exhaustion could be Critical if it causes a cascading failure.
    /// This is a design decision worth documenting.
    #[test]
    fn blackhat_severity_for_budget_exceeded_is_major_not_critical() {
        let severity = IncidentScreen::severity_for_code(&FailureCode::BudgetExceeded);
        assert_eq!(
            severity,
            IncidentSeverity::Major,
            "BudgetExceeded is Major, not Critical"
        );
    }

    /// FINDING: replay_safe_for_code maps only ActionTimeout, ValidationError,
    /// and BudgetExceeded as replay-safe. StepPanicked is NOT marked replay-safe,
    /// which is correct since a panic may have left partial state.
    #[test]
    fn blackhat_step_panicked_is_not_replay_safe() {
        let replay_safe = IncidentScreen::replay_safe_for_code(&FailureCode::StepPanicked);
        assert!(!replay_safe, "StepPanicked is correctly NOT replay-safe");
    }

    /// FINDING: detail_sections() state_diff only matches taint_changes to
    /// slot_values_before by slot_index. If taint_changes has entries for
    /// slots not in slot_values_before, those taint changes are silently
    /// dropped from the diff view.
    #[test]
    fn blackhat_detail_sections_drops_taint_changes_without_matching_slot() {
        let mut screen = IncidentScreen::new();
        // Create an incident with a taint change for a slot not in slot_values_before
        let now = std::time::Instant::now();
        let incident = Incident {
            id: 1,
            incident_type: IncidentType::ActionFailure,
            severity: IncidentSeverity::Major,
            failure_code: FailureCode::ActionTimeout,
            run_id: 1,
            workflow_name: String::new(),
            step_id: None,
            step_name: None,
            error_message: String::from("test"),
            replay_safe: true,
            side_effect_certainty: SideEffectCertainty::None,
            timestamp: now,
            context: IncidentContext {
                slot_values_before: vec![(1_u16, String::from("slot1"))],
                taint_changes: vec![
                    (1_u16, String::from("tainted_slot1")),
                    (2_u16, String::from("orphan_taint")), // no matching slot_values_before
                ],
                action_attempts: 0,
                last_action_idempotency_key: None,
            },
            timeline: vec![],
        };
        screen.register_incident(incident);
        screen.select_incident(0);
        let sections = screen.detail_sections();
        // Only slot 1 should appear in state_diff; slot 2's taint is silently dropped
        assert_eq!(
            sections.state_diff.len(),
            1,
            "only slot_values_before slots appear in diff"
        );
        assert_eq!(sections.state_diff[0].slot_index, 1);
        // The orphan taint for slot 2 is not visible in the diff
    }

    /// FINDING: process_run_failure always sets step_id to None. The step
    /// parameter becomes step_name (Option<&str>), but the Incident's step_id
    /// field (Option<u16>) is always None. Callers expecting step_id to be
    /// populated from process_run_failure will be disappointed.
    #[test]
    fn blackhat_process_run_failure_always_sets_step_id_to_none() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, Some("step-name"), FailureCode::ActionTimeout, "t");
        let detail = screen.get_failure_detail(0).unwrap();
        assert!(
            detail.step_id.is_none(),
            "step_id is always None from process_run_failure"
        );
        assert_eq!(
            detail.step_name.as_deref(),
            Some("step-name"),
            "step_name is populated from the step param"
        );
    }

    /// FINDING: process_replay_divergence sets side_effect_certainty to Unknown
    /// and replay_safe to false. This is correct and conservative.
    #[test]
    fn blackhat_replay_divergence_is_unsafe_and_unknown_certainty() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(1, "a", "b");
        let detail = screen.get_failure_detail(0).unwrap();
        assert!(!detail.replay_safe);
        assert_eq!(detail.side_effect_certainty, SideEffectCertainty::Unknown);
    }

    /// FINDING: format_failure_code uses different labels than FailureCode::as_str().
    /// For example, BudgetExceeded is "BudgetExceeded" in format_failure_code
    /// but "StepBudgetExhausted" in FailureCode::as_str(). This inconsistency
    /// could confuse consumers of the error information.
    #[test]
    fn blackhat_format_failure_code_disagrees_with_as_str() {
        assert_eq!(
            format_failure_code(&FailureCode::BudgetExceeded),
            "BudgetExceeded"
        );
        assert_eq!(FailureCode::BudgetExceeded.as_str(), "StepBudgetExhausted");
        // These disagree! format_failure_code and as_str() return different strings.

        assert_eq!(format_failure_code(&FailureCode::TaintLeak), "TaintLeak");
        assert_eq!(FailureCode::TaintLeak.as_str(), "TaintViolation");
        // Another disagreement: "TaintLeak" vs "TaintViolation"
    }

    /// FINDING: IncidentScreen wraps IncidentConsole but does not expose
    /// the Phase 5A record API (push_incident, active_incidents, etc.).
    /// This means Phase 5A record features are only available through
    /// IncidentConsole directly, creating a split API surface.
    #[test]
    fn blackhat_screen_does_not_expose_phase5a_record_api() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        // IncidentScreen has no push_incident, active_incidents, clear_resolved, etc.
        // These are only on IncidentConsole.
        assert_eq!(screen.active_count(), 1);
        // active_count() delegates to legacy active_count, not records
    }

    /// FINDING: dismiss_selected() has a TOCTOU-like pattern where it gets
    /// selected_index, then checks had_incident, then dismisses. In single-
    /// threaded code this is safe, but the defensive check is good practice.
    #[test]
    fn blackhat_dismiss_selected_defensive_check_when_incident_already_gone() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        // Dismiss via the non-selected path first
        screen.dismiss(0);
        // Now dismiss_selected tries to use the stale selected_index
        let result = screen.dismiss_selected();
        assert!(
            !result,
            "dismiss_selected returns false when incident already gone"
        );
    }

    // =========================================================================
    // Phase 5A layout data model tests
    // =========================================================================

    // -- FailureKind::label tests (6 tests) --

    #[test]
    fn phase5a_failure_kind_label_action_timeout() {
        assert_eq!(FailureKind::ActionTimeout.label(), "ACTION_TIMEOUT");
    }

    #[test]
    fn phase5a_failure_kind_label_action_failed() {
        assert_eq!(FailureKind::ActionFailed.label(), "ACTION_FAILED");
    }

    #[test]
    fn phase5a_failure_kind_label_run_failed() {
        assert_eq!(FailureKind::RunFailed.label(), "RUN_FAILED");
    }

    #[test]
    fn phase5a_failure_kind_label_retry_exhausted() {
        assert_eq!(FailureKind::RetryExhausted.label(), "RETRY_EXHAUSTED");
    }

    #[test]
    fn phase5a_failure_kind_label_taint_violation() {
        assert_eq!(FailureKind::TaintViolation.label(), "TAINT_VIOLATION");
    }

    #[test]
    fn phase5a_failure_kind_label_internal_error() {
        assert_eq!(FailureKind::InternalError.label(), "INTERNAL_ERROR");
    }

    // -- FailureKind::is_replay_safe_default tests (3 tests) --

    #[test]
    fn phase5a_failure_kind_replay_safe_timeout() {
        assert!(FailureKind::ActionTimeout.is_replay_safe_default());
    }

    #[test]
    fn phase5a_failure_kind_replay_safe_retry_exhausted() {
        assert!(FailureKind::RetryExhausted.is_replay_safe_default());
    }

    #[test]
    fn phase5a_failure_kind_not_replay_safe_others() {
        assert!(!FailureKind::ActionFailed.is_replay_safe_default());
        assert!(!FailureKind::RunFailed.is_replay_safe_default());
        assert!(!FailureKind::TaintViolation.is_replay_safe_default());
        assert!(!FailureKind::InternalError.is_replay_safe_default());
    }

    // -- FailureKind distinctness (1 test) --

    #[test]
    fn phase5a_failure_kind_variants_are_distinct() {
        let variants = [
            FailureKind::ActionTimeout,
            FailureKind::ActionFailed,
            FailureKind::RunFailed,
            FailureKind::RetryExhausted,
            FailureKind::TaintViolation,
            FailureKind::InternalError,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    // -- IncidentCard tests (6 tests) --

    #[test]
    fn phase5a_incident_card_placeholder_values() {
        let card = IncidentCard::placeholder();
        assert_eq!(card.run_id, 8172);
        assert_eq!(card.workflow_name, "issue-triage");
        assert_eq!(card.step_idx, 3);
        assert_eq!(card.failure_kind, FailureKind::ActionTimeout);
        assert!(card.replay_safe);
    }

    #[test]
    fn phase5a_incident_card_badge_color_timeout() {
        let card = IncidentCard::placeholder();
        assert_eq!(card.badge_color(), NEON_ORANGE);
    }

    #[test]
    fn phase5a_incident_card_run_id_text() {
        let card = IncidentCard::placeholder();
        assert_eq!(card.run_id_text(), "8172");
    }

    #[test]
    fn phase5a_incident_card_step_text() {
        let card = IncidentCard::placeholder();
        assert_eq!(card.step_text(), "StepIdx 3");
    }

    #[test]
    fn phase5a_incident_card_custom_construction() {
        let card = IncidentCard {
            run_id: 99,
            workflow_name: String::from("ci-pipeline"),
            step_idx: 7,
            failure_kind: FailureKind::TaintViolation,
            timestamp: 9999,
            replay_safe: false,
        };
        assert_eq!(card.run_id, 99);
        assert_eq!(card.workflow_name, "ci-pipeline");
        assert_eq!(card.failure_kind, FailureKind::TaintViolation);
        assert!(!card.replay_safe);
        assert_eq!(card.badge_color(), NEON_MAGENTA);
    }

    #[test]
    fn phase5a_incident_card_clone() {
        let card = IncidentCard::placeholder();
        let cloned = card.clone();
        assert_eq!(cloned.run_id, card.run_id);
        assert_eq!(cloned.workflow_name, card.workflow_name);
        assert_eq!(cloned.step_idx, card.step_idx);
        assert_eq!(cloned.failure_kind, card.failure_kind);
        assert_eq!(cloned.timestamp, card.timestamp);
        assert_eq!(cloned.replay_safe, card.replay_safe);
    }

    // -- CausePanel tests (3 tests) --

    #[test]
    fn phase5a_cause_panel_placeholder_values() {
        let panel = CausePanel::placeholder();
        assert_eq!(panel.error_message, "TIMEOUT");
        assert_eq!(panel.context, "5s exceeded");
        assert_eq!(panel.recommended_action, "retry with same ticket");
    }

    #[test]
    fn phase5a_cause_panel_custom_construction() {
        let panel = CausePanel {
            error_message: String::from("SEGFAULT"),
            context: String::from("null pointer dereference"),
            recommended_action: String::from("investigate step logic"),
        };
        assert_eq!(panel.error_message, "SEGFAULT");
        assert_eq!(panel.context, "null pointer dereference");
        assert_eq!(panel.recommended_action, "investigate step logic");
    }

    #[test]
    fn phase5a_cause_panel_clone() {
        let panel = CausePanel::placeholder();
        let cloned = panel.clone();
        assert_eq!(cloned.error_message, panel.error_message);
        assert_eq!(cloned.context, panel.context);
        assert_eq!(cloned.recommended_action, panel.recommended_action);
    }

    // -- TimelineChip tests (3 tests) --

    #[test]
    fn phase5a_timeline_chip_label_format() {
        let chip = TimelineChip {
            seq: 14,
            kind: String::from("ActionFailed"),
            step_idx: 3,
        };
        assert_eq!(chip.label(), "[14] ActionFailed");
    }

    #[test]
    fn phase5a_timeline_chip_custom_construction() {
        let chip = TimelineChip {
            seq: 1,
            kind: String::from("StepStarted"),
            step_idx: 0,
        };
        assert_eq!(chip.seq, 1);
        assert_eq!(chip.kind, "StepStarted");
        assert_eq!(chip.step_idx, 0);
    }

    #[test]
    fn phase5a_timeline_chip_clone() {
        let chip = TimelineChip {
            seq: 42,
            kind: String::from("RunFailed"),
            step_idx: 10,
        };
        let cloned = chip.clone();
        assert_eq!(cloned.seq, chip.seq);
        assert_eq!(cloned.kind, chip.kind);
        assert_eq!(cloned.step_idx, chip.step_idx);
    }

    // -- TimelinePanel tests (5 tests) --

    #[test]
    fn phase5a_timeline_panel_placeholder_has_four_chips() {
        let panel = TimelinePanel::placeholder();
        assert_eq!(panel.chip_count(), 4);
        assert!(!panel.is_empty());
    }

    #[test]
    fn phase5a_timeline_panel_placeholder_first_chip() {
        let panel = TimelinePanel::placeholder();
        let first = panel.events.first();
        assert!(first.is_some());
        assert_eq!(first.map(|c| c.seq), Some(12));
        assert_eq!(first.map(|c| c.kind.as_str()), Some("StepStarted"));
        assert_eq!(first.map(|c| c.step_idx), Some(3));
    }

    #[test]
    fn phase5a_timeline_panel_placeholder_last_chip() {
        let panel = TimelinePanel::placeholder();
        let last = panel.events.last();
        assert!(last.is_some());
        assert_eq!(last.map(|c| c.kind.as_str()), Some("RunFailed"));
    }

    #[test]
    fn phase5a_timeline_panel_empty() {
        let panel = TimelinePanel { events: Vec::new() };
        assert_eq!(panel.chip_count(), 0);
        assert!(panel.is_empty());
    }

    #[test]
    fn phase5a_timeline_panel_clone() {
        let panel = TimelinePanel::placeholder();
        let cloned = panel.clone();
        assert_eq!(cloned.chip_count(), panel.chip_count());
        assert_eq!(cloned.events.len(), panel.events.len());
    }

    // -- SlotDiff tests (8 tests) --

    #[test]
    fn phase5a_slot_diff_display_label_value_change() {
        let diff = SlotDiff {
            slot_idx: 12,
            before: String::from("null"),
            after: String::from("ObjectId(\"issue-8472\")"),
            taint_changed: false,
        };
        assert_eq!(
            diff.display_label(),
            "SlotIdx(12): null -> ObjectId(\"issue-8472\")"
        );
    }

    #[test]
    fn phase5a_slot_diff_display_label_taint_change() {
        let diff = SlotDiff {
            slot_idx: 5,
            before: String::from("Clean"),
            after: String::from("DerivedFromSecret"),
            taint_changed: true,
        };
        assert_eq!(
            diff.display_label(),
            "Taint(SlotIdx(5)): Clean -> DerivedFromSecret"
        );
    }

    #[test]
    fn phase5a_slot_diff_value_changed_true() {
        let diff = SlotDiff {
            slot_idx: 1,
            before: String::from("old"),
            after: String::from("new"),
            taint_changed: false,
        };
        assert!(diff.value_changed());
    }

    #[test]
    fn phase5a_slot_diff_value_changed_false() {
        let diff = SlotDiff {
            slot_idx: 1,
            before: String::from("same"),
            after: String::from("same"),
            taint_changed: false,
        };
        assert!(!diff.value_changed());
    }

    #[test]
    fn phase5a_slot_diff_diff_color_taint() {
        let diff = SlotDiff {
            slot_idx: 5,
            before: String::from("a"),
            after: String::from("b"),
            taint_changed: true,
        };
        assert_eq!(diff.diff_color(), NEON_MAGENTA);
    }

    #[test]
    fn phase5a_slot_diff_diff_color_value_change() {
        let diff = SlotDiff {
            slot_idx: 1,
            before: String::from("old"),
            after: String::from("new"),
            taint_changed: false,
        };
        assert_eq!(diff.diff_color(), NEON_CYAN);
    }

    #[test]
    fn phase5a_slot_diff_diff_color_no_change() {
        let diff = SlotDiff {
            slot_idx: 1,
            before: String::from("same"),
            after: String::from("same"),
            taint_changed: false,
        };
        assert_eq!(diff.diff_color(), TEXT_SECONDARY);
    }

    #[test]
    fn phase5a_slot_diff_clone() {
        let diff = SlotDiff {
            slot_idx: 3,
            before: String::from("a"),
            after: String::from("b"),
            taint_changed: true,
        };
        let cloned = diff.clone();
        assert_eq!(cloned.slot_idx, diff.slot_idx);
        assert_eq!(cloned.before, diff.before);
        assert_eq!(cloned.after, diff.after);
        assert_eq!(cloned.taint_changed, diff.taint_changed);
    }

    // -- StateDiffPanel tests (7 tests) --

    #[test]
    fn phase5a_state_diff_panel_placeholder_has_two_diffs() {
        let panel = StateDiffPanel::placeholder();
        assert_eq!(panel.diff_count(), 2);
        assert!(!panel.is_empty());
    }

    #[test]
    fn phase5a_state_diff_panel_placeholder_first_diff() {
        let panel = StateDiffPanel::placeholder();
        let first = panel.diffs.first();
        assert!(first.is_some());
        assert_eq!(first.map(|d| d.slot_idx), Some(12));
        assert_eq!(first.map(|d| d.before.as_str()), Some("null"));
        assert_eq!(
            first.map(|d| d.after.as_str()),
            Some("ObjectId(\"issue-8472\")")
        );
        assert_eq!(first.map(|d| d.taint_changed), Some(false));
    }

    #[test]
    fn phase5a_state_diff_panel_placeholder_taint_change() {
        let panel = StateDiffPanel::placeholder();
        let second = panel.diffs.get(1);
        assert!(second.is_some());
        assert_eq!(second.map(|d| d.taint_changed), Some(true));
        assert_eq!(second.map(|d| d.slot_idx), Some(5));
    }

    #[test]
    fn phase5a_state_diff_panel_has_taint_and_value_changes() {
        let panel = StateDiffPanel::placeholder();
        assert!(panel.has_taint_changes());
        assert!(panel.has_value_changes());
    }

    #[test]
    fn phase5a_state_diff_panel_empty() {
        let panel = StateDiffPanel { diffs: Vec::new() };
        assert_eq!(panel.diff_count(), 0);
        assert!(panel.is_empty());
        assert!(!panel.has_taint_changes());
        assert!(!panel.has_value_changes());
    }

    #[test]
    fn phase5a_state_diff_panel_no_taint_changes() {
        let panel = StateDiffPanel {
            diffs: vec![SlotDiff {
                slot_idx: 1,
                before: String::from("a"),
                after: String::from("b"),
                taint_changed: false,
            }],
        };
        assert!(!panel.has_taint_changes());
        assert!(panel.has_value_changes());
    }

    #[test]
    fn phase5a_state_diff_panel_clone() {
        let panel = StateDiffPanel::placeholder();
        let cloned = panel.clone();
        assert_eq!(cloned.diff_count(), panel.diff_count());
        assert_eq!(cloned.diffs.len(), panel.diffs.len());
    }

    // -- Color constants tests (4 tests) --

    #[test]
    fn phase5a_color_constants_background_layer() {
        assert_eq!(CANVAS_BG, "#0a0a12");
        assert_eq!(PANEL_BG, "#12121f");
        assert_eq!(PANEL_BG_ALT, "#1a1a2e");
        assert_eq!(CARD_BG, "#16162a");
        assert_eq!(BORDER, "#2a2a4a");
        assert_eq!(GRID_LINE, "#1e1e3a");
    }

    #[test]
    fn phase5a_color_constants_neon_accents() {
        assert_eq!(NEON_CYAN, "#00f5ff");
        assert_eq!(NEON_MAGENTA, "#ff00ff");
        assert_eq!(NEON_YELLOW, "#ffe600");
        assert_eq!(NEON_GREEN, "#39ff14");
        assert_eq!(NEON_RED, "#ff073a");
        assert_eq!(NEON_PURPLE, "#b14dff");
        assert_eq!(NEON_ORANGE, "#ff6b00");
        assert_eq!(NEON_TEAL, "#00e5c7");
        assert_eq!(NEON_PINK, "#ff2d7b");
        assert_eq!(NEON_BLUE, "#2d6bff");
    }

    #[test]
    fn phase5a_color_constants_text_and_state() {
        assert_eq!(TEXT_PRIMARY, "#e8e8ff");
        assert_eq!(TEXT_SECONDARY, "#8888aa");
        assert_eq!(TEXT_DIM, "#555577");
        assert_eq!(TEXT_ACCENT, "#00f5ff");
        assert_eq!(STATE_SUCCEEDED, "#39ff14");
        assert_eq!(STATE_RUNNING, "#00f5ff");
        assert_eq!(STATE_FAILED, "#ff073a");
        assert_eq!(STATE_WAITING, "#2d6bff");
        assert_eq!(STATE_RETRYING, "#ff6b00");
        assert_eq!(STATE_CANCELLED, "#555577");
        assert_eq!(STATE_SECRET_TAINTED, "#ff00ff");
    }

    #[test]
    fn phase5a_failure_kind_color_all_mappings() {
        assert_eq!(failure_kind_color(&FailureKind::ActionTimeout), NEON_ORANGE);
        assert_eq!(failure_kind_color(&FailureKind::ActionFailed), NEON_RED);
        assert_eq!(failure_kind_color(&FailureKind::RunFailed), NEON_RED);
        assert_eq!(
            failure_kind_color(&FailureKind::RetryExhausted),
            NEON_YELLOW
        );
        assert_eq!(
            failure_kind_color(&FailureKind::TaintViolation),
            NEON_MAGENTA
        );
        assert_eq!(failure_kind_color(&FailureKind::InternalError), TEXT_DIM);
    }

    // -- severity_color_hex (1 test) --

    #[test]
    fn phase5a_severity_color_hex_all_mappings() {
        assert_eq!(severity_color_hex(IncidentSeverity::Critical), NEON_RED);
        assert_eq!(severity_color_hex(IncidentSeverity::Major), NEON_ORANGE);
        assert_eq!(severity_color_hex(IncidentSeverity::Minor), NEON_YELLOW);
        assert_eq!(severity_color_hex(IncidentSeverity::Warning), NEON_YELLOW);
        assert_eq!(severity_color_hex(IncidentSeverity::Info), NEON_CYAN);
    }

    // -- FailureKind Copy trait (1 test) --

    #[test]
    fn phase5a_failure_kind_copy_trait() {
        let kind = FailureKind::ActionTimeout;
        let copied = kind;
        assert_eq!(kind, copied);
    }

    // -- IncidentCard badge_color all kinds (1 test) --

    #[test]
    fn phase5a_incident_card_badge_color_all_kinds() {
        let kinds = [
            (FailureKind::ActionTimeout, NEON_ORANGE),
            (FailureKind::ActionFailed, NEON_RED),
            (FailureKind::RunFailed, NEON_RED),
            (FailureKind::RetryExhausted, NEON_YELLOW),
            (FailureKind::TaintViolation, NEON_MAGENTA),
            (FailureKind::InternalError, TEXT_DIM),
        ];
        for (kind, expected_color) in &kinds {
            let card = IncidentCard {
                run_id: 1,
                workflow_name: String::from("test"),
                step_idx: 0,
                failure_kind: *kind,
                timestamp: 0,
                replay_safe: true,
            };
            assert_eq!(
                card.badge_color(),
                *expected_color,
                "badge_color mismatch for {:?}",
                kind
            );
        }
    }

    // -- SlotDiff display_label edge case: empty strings (1 test) --

    #[test]
    fn phase5a_slot_diff_display_label_empty_strings() {
        let diff = SlotDiff {
            slot_idx: 0,
            before: String::new(),
            after: String::new(),
            taint_changed: false,
        };
        assert_eq!(diff.display_label(), "SlotIdx(0):  -> ");
        assert!(!diff.value_changed());
    }

    // -- StateDiffPanel: taint change with no value change (1 test) --

    #[test]
    fn phase5a_state_diff_panel_taint_only_change() {
        let panel = StateDiffPanel {
            diffs: vec![SlotDiff {
                slot_idx: 3,
                before: String::from("same"),
                after: String::from("same"),
                taint_changed: true,
            }],
        };
        assert!(panel.has_taint_changes());
        assert!(
            !panel.has_value_changes(),
            "values are same but taint changed"
        );
    }

    // =========================================================================
    // suggest_repairs_for_failure_kind tests
    // =========================================================================

    #[test]
    fn test_suggest_repairs_for_failure_kind_action_timeout() {
        assert_eq!(
            suggest_repairs_for_failure_kind(&FailureKind::ActionTimeout),
            "Increase timeout or check downstream service",
        );
    }

    #[test]
    fn test_suggest_repairs_for_failure_kind_retry_exhausted() {
        assert_eq!(
            suggest_repairs_for_failure_kind(&FailureKind::RetryExhausted),
            "Check external service availability",
        );
    }

    #[test]
    fn test_suggest_repairs_for_failure_kind_taint_violation() {
        assert_eq!(
            suggest_repairs_for_failure_kind(&FailureKind::TaintViolation),
            "Add sanitization step before Finish",
        );
    }

    #[test]
    fn test_suggest_repairs_for_failure_kind_all_variants_non_empty() {
        let variants = [
            FailureKind::ActionTimeout,
            FailureKind::ActionFailed,
            FailureKind::RunFailed,
            FailureKind::RetryExhausted,
            FailureKind::TaintViolation,
            FailureKind::InternalError,
        ];
        for kind in &variants {
            let suggestion = suggest_repairs_for_failure_kind(kind);
            assert!(
                !suggestion.is_empty(),
                "suggestion must be non-empty for {:?}",
                kind
            );
        }
    }

    // =========================================================================
    // SuggestionItem tests
    // =========================================================================

    #[test]
    fn test_suggestion_item_kind_label() {
        let item = SuggestionItem {
            suggestion: super::super::repair::RepairSuggestion {
                kind: super::super::repair::RepairKind::IncreaseTimeout,
                description: String::from("test"),
                action: super::super::repair::RepairAction::IncreaseTimeout,
                confidence: 0.9,
                confidence_level: super::super::repair::RepairConfidence::High,
                rationale: String::from("test"),
            },
            action_label: String::from("ACTION_TIMEOUT"),
            badge_color: NEON_ORANGE,
            summary: String::from("Increase timeout or check downstream service"),
        };
        assert_eq!(item.kind_label(), "IncreaseTimeout");
    }

    #[test]
    fn test_suggestion_item_confidence_display() {
        let item = SuggestionItem {
            suggestion: super::super::repair::RepairSuggestion {
                kind: super::super::repair::RepairKind::FixSecretLeak,
                description: String::from("test"),
                action: super::super::repair::RepairAction::FixSecretLeak,
                confidence: 0.85,
                confidence_level: super::super::repair::RepairConfidence::High,
                rationale: String::from("test"),
            },
            action_label: String::from("TAINT_VIOLATION"),
            badge_color: NEON_MAGENTA,
            summary: String::from("Add sanitization step before Finish"),
        };
        assert_eq!(item.confidence_display(), "85%");
    }

    #[test]
    fn test_suggestion_item_is_high_confidence() {
        let item = SuggestionItem {
            suggestion: super::super::repair::RepairSuggestion {
                kind: super::super::repair::RepairKind::FixSecretLeak,
                description: String::from("test"),
                action: super::super::repair::RepairAction::FixSecretLeak,
                confidence: 0.9,
                confidence_level: super::super::repair::RepairConfidence::High,
                rationale: String::from("test"),
            },
            action_label: String::from("TAINT_VIOLATION"),
            badge_color: NEON_MAGENTA,
            summary: String::from("test"),
        };
        assert!(item.is_high_confidence());
    }

    #[test]
    fn test_suggestion_item_is_not_high_confidence_for_low() {
        let item = SuggestionItem {
            suggestion: super::super::repair::RepairSuggestion {
                kind: super::super::repair::RepairKind::PinIdempotency,
                description: String::from("test"),
                action: super::super::repair::RepairAction::ManualIntervention,
                confidence: 0.3,
                confidence_level: super::super::repair::RepairConfidence::Low,
                rationale: String::from("test"),
            },
            action_label: String::from("INTERNAL_ERROR"),
            badge_color: TEXT_DIM,
            summary: String::from("test"),
        };
        assert!(!item.is_high_confidence());
    }

    // =========================================================================
    // build_suggestion_items tests
    // =========================================================================

    #[test]
    fn test_build_suggestion_items_empty_suggestions() {
        let items = build_suggestion_items(&FailureKind::ActionTimeout, Vec::new());
        assert!(items.is_empty());
    }

    #[test]
    fn test_build_suggestion_items_primary_uses_kind_summary() {
        let suggestions = vec![super::super::repair::RepairSuggestion {
            kind: super::super::repair::RepairKind::IncreaseTimeout,
            description: String::from("test"),
            action: super::super::repair::RepairAction::IncreaseTimeout,
            confidence: 0.9,
            confidence_level: super::super::repair::RepairConfidence::High,
            rationale: String::from("test"),
        }];
        let items = build_suggestion_items(&FailureKind::ActionTimeout, suggestions);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].summary,
            "Increase timeout or check downstream service"
        );
        assert_eq!(items[0].badge_color, NEON_ORANGE);
    }

    #[test]
    fn test_build_suggestion_items_secondary_uses_teal() {
        let suggestions = vec![
            super::super::repair::RepairSuggestion {
                kind: super::super::repair::RepairKind::IncreaseTimeout,
                description: String::from("primary"),
                action: super::super::repair::RepairAction::IncreaseTimeout,
                confidence: 0.9,
                confidence_level: super::super::repair::RepairConfidence::High,
                rationale: String::from("test"),
            },
            super::super::repair::RepairSuggestion {
                kind: super::super::repair::RepairKind::AddRetryBackoff,
                description: String::from("secondary desc"),
                action: super::super::repair::RepairAction::AddRetryBackoff,
                confidence: 0.7,
                confidence_level: super::super::repair::RepairConfidence::Medium,
                rationale: String::from("test"),
            },
        ];
        let items = build_suggestion_items(&FailureKind::ActionTimeout, suggestions);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].badge_color, NEON_TEAL);
        assert_eq!(items[1].summary, "secondary desc");
    }

    // =========================================================================
    // suggestion_items_for_failure_code tests
    // =========================================================================

    #[test]
    fn test_suggestion_items_for_action_timeout() {
        let items = suggestion_items_for_failure_code(&FailureCode::ActionTimeout);
        assert!(!items.is_empty());
        assert!(items[0].summary.contains("Increase timeout"));
    }

    #[test]
    fn test_suggestion_items_for_taint_leak() {
        let items = suggestion_items_for_failure_code(&FailureCode::TaintLeak);
        assert!(!items.is_empty());
        assert!(items[0].summary.contains("sanitization"));
    }

    #[test]
    fn test_suggestion_items_for_budget_exceeded() {
        let items = suggestion_items_for_failure_code(&FailureCode::BudgetExceeded);
        assert!(!items.is_empty());
    }

    #[test]
    fn test_suggestion_items_for_all_failure_codes() {
        let codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed("err".into()),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError("v".into()),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown("x".into()),
        ];
        for code in &codes {
            let items = suggestion_items_for_failure_code(code);
            assert!(!items.is_empty(), "must produce items for {:?}", code);
        }
    }

    // =========================================================================
    // IncidentScreen suggestion_items / selected_suggestion_items tests
    // =========================================================================

    #[test]
    fn test_selected_suggestion_items_empty_when_no_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        let items = screen.selected_suggestion_items();
        assert!(items.is_empty());
    }

    #[test]
    fn test_selected_suggestion_items_returns_items_after_select() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let items = screen.selected_suggestion_items();
        assert!(!items.is_empty());
        assert!(items[0].summary.contains("Increase timeout"));
    }

    #[test]
    fn test_suggestion_items_by_index() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        let items = screen.suggestion_items(0);
        assert!(!items.is_empty());
        assert!(items[0].summary.contains("sanitization"));
    }

    #[test]
    fn test_suggestion_items_by_index_out_of_bounds() {
        let screen = IncidentScreen::new();
        let items = screen.suggestion_items(0);
        assert!(items.is_empty());
    }
}
