use super::*;
use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx};
use vb_storage::EventSeq;

fn mk_seq(n: u64) -> EventSeq {
    EventSeq::new(n)
}

fn mk_run(id: u64) -> RunId {
    RunId::new(id)
}

fn mk_step(idx: u16) -> StepIdx {
    StepIdx::new(idx)
}

fn mk_action(id: u16) -> ActionId {
    ActionId::new(id)
}

fn mk_slot(idx: u16) -> SlotIdx {
    SlotIdx::new(idx)
}

fn mk_slot_value(val: i64) -> SlotValue {
    SlotValue::I64(val)
}

#[test]
fn diff_event_summary_returns_run_accepted_with_seq() {
    let event = JournalEvent::RunAccepted {
        run: mk_run(1),
        seq: mk_seq(5),
        workflow: vb_core::WorkflowDigest::from_bytes([0xAB; 32]),
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"].as_str().unwrap(), "RunAccepted");
    assert_eq!(summary["seq"].as_u64().unwrap(), 5);
}

#[test]
fn diff_event_summary_returns_step_started_with_index() {
    let event = JournalEvent::StepStarted {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(3),
        attempt: 1,
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"].as_str().unwrap(), "StepStarted");
    assert_eq!(summary["step"].as_u64().unwrap(), 3);
}

#[test]
fn diff_event_summary_returns_step_succeeded_with_output() {
    let event = JournalEvent::StepSucceeded {
        run: mk_run(1),
        seq: mk_seq(2),
        step: mk_step(1),
        output: mk_slot(0),
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"].as_str().unwrap(), "StepSucceeded");
}

#[test]
fn diff_event_summary_returns_action_scheduled_with_action_id() {
    let event = JournalEvent::ActionScheduled {
        run: mk_run(1),
        seq: mk_seq(4),
        step: mk_step(2),
        action: mk_action(42),
        attempt: 1,
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"].as_str().unwrap(), "ActionScheduled");
    assert_eq!(summary["action"].as_u64().unwrap(), 42);
}

#[test]
fn diff_event_summary_returns_slot_written_with_value() {
    let event = JournalEvent::SlotWrittenEvent {
        run: mk_run(1),
        seq: mk_seq(3),
        slot: mk_slot(5),
        value: Some(vec![1, 2, 3]),
        extra: None,
        attempt: 1,
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["has_value"].as_bool().unwrap(), true);
}

#[test]
fn diff_event_summary_returns_slot_written_without_value() {
    let event = JournalEvent::SlotWrittenEvent {
        run: mk_run(1),
        seq: mk_seq(3),
        slot: mk_slot(2),
        value: None,
        extra: None,
        attempt: 1,
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["has_value"].as_bool().unwrap(), false);
}

#[test]
fn diff_event_summary_returns_run_cancelled() {
    let event = JournalEvent::RunCancelled {
        run: mk_run(1),
        seq: mk_seq(11),
        attempt: 1,
        reason: None,
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"].as_str().unwrap(), "RunCancelled");
}

#[test]
fn diff_event_summary_returns_run_finished_with_result() {
    let event = JournalEvent::RunFinished {
        run: mk_run(1),
        seq: mk_seq(12),
        result: mk_slot(0),
        attempt: 1,
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"].as_str().unwrap(), "RunFinished");
}

#[test]
fn diff_event_summary_returns_run_resumed() {
    let event = JournalEvent::RunResumed {
        run: mk_run(1),
        seq: mk_seq(14),
        timestamp: chrono::Utc::now(),
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"].as_str().unwrap(), "RunResumed");
}

#[test]
fn diff_event_summary_returns_run_answered_with_slot_idx() {
    let event = JournalEvent::RunAnswered {
        run: mk_run(1),
        seq: mk_seq(16),
        slot_idx: mk_slot(3),
        answer: vb_core::ConstValue::Bool(true),
        timestamp: chrono::Utc::now(),
    };
    let summary = diff_event_summary(&event);
    assert_eq!(summary["type"].as_str().unwrap(), "RunAnswered");
    assert_eq!(summary["slot_idx"].as_u64().unwrap(), 3);
}

#[test]
fn event_name_returns_run_accepted() {
    let event = JournalEvent::RunAccepted {
        run: mk_run(1),
        seq: mk_seq(1),
        workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
    };
    assert_eq!(event_name(&event), "RunAccepted");
}

#[test]
fn event_name_returns_step_started() {
    let event = JournalEvent::StepStarted {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(1),
        attempt: 1,
    };
    assert_eq!(event_name(&event), "StepStarted");
}

#[test]
fn event_name_returns_all_variants() {
    assert_eq!(
        event_name(&JournalEvent::StepStarted {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            attempt: 1
        }),
        "StepStarted"
    );
    assert_eq!(
        event_name(&JournalEvent::ActionCompletedEvent {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            action: mk_action(1),
            attempt: 1
        }),
        "ActionCompleted"
    );
    assert_eq!(
        event_name(&JournalEvent::ActionFailedEvent {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            action: mk_action(1),
            attempt: 1
        }),
        "ActionFailed"
    );
    assert_eq!(
        event_name(&JournalEvent::AskScheduledEvent {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            attempt: 1
        }),
        "AskScheduled"
    );
    assert_eq!(
        event_name(&JournalEvent::AskAnsweredEvent {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            attempt: 1
        }),
        "AskAnswered"
    );
    assert_eq!(
        event_name(&JournalEvent::RetryScheduledEvent {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            attempt: 1
        }),
        "RetryScheduled"
    );
    assert_eq!(
        event_name(&JournalEvent::WaitScheduledEvent {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            attempt: 1
        }),
        "WaitScheduled"
    );
    assert_eq!(
        event_name(&JournalEvent::SlotWrittenEvent {
            run: mk_run(1),
            seq: mk_seq(1),
            slot: mk_slot(1),
            value: None,
            extra: None,
            attempt: 1
        }),
        "SlotWritten"
    );
}

#[test]
fn events_differ_returns_false_for_identical_step_succeeded() {
    let a = JournalEvent::StepSucceeded {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(1),
        output: mk_slot(0),
    };
    let b = JournalEvent::StepSucceeded {
        run: mk_run(2),
        seq: mk_seq(99),
        step: mk_step(1),
        output: mk_slot(0),
    };
    assert!(!events_differ(&a, &b));
}

#[test]
fn events_differ_returns_true_for_different_step_in_step_succeeded() {
    let a = JournalEvent::StepSucceeded {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(1),
        output: mk_slot(0),
    };
    let b = JournalEvent::StepSucceeded {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(2),
        output: mk_slot(0),
    };
    assert!(events_differ(&a, &b));
}

#[test]
fn events_differ_returns_true_for_different_event_types() {
    let a = JournalEvent::StepStarted {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(1),
        attempt: 1,
    };
    let b = JournalEvent::StepSucceeded {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(1),
        output: mk_slot(0),
    };
    assert!(events_differ(&a, &b));
}

#[test]
fn events_differ_compares_action_scheduled_by_step_and_action() {
    let a = JournalEvent::ActionScheduled {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(1),
        action: mk_action(1),
        attempt: 1,
    };
    let b = JournalEvent::ActionScheduled {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(1),
        action: mk_action(2),
        attempt: 1,
    };
    assert!(events_differ(&a, &b));
}

#[test]
fn events_differ_compares_slot_written_by_slot_and_value() {
    let a = JournalEvent::SlotWrittenEvent {
        run: mk_run(1),
        seq: mk_seq(1),
        slot: mk_slot(1),
        value: Some(vec![1]),
        extra: None,
        attempt: 1,
    };
    let b = JournalEvent::SlotWrittenEvent {
        run: mk_run(1),
        seq: mk_seq(1),
        slot: mk_slot(2),
        value: Some(vec![1]),
        extra: None,
        attempt: 1,
    };
    assert!(events_differ(&a, &b));
}

#[test]
fn events_differ_compares_run_finished_by_result() {
    let a = JournalEvent::RunFinished {
        run: mk_run(1),
        seq: mk_seq(1),
        result: mk_slot(0),
        attempt: 1,
    };
    let b = JournalEvent::RunFinished {
        run: mk_run(1),
        seq: mk_seq(1),
        result: mk_slot(1),
        attempt: 1,
    };
    assert!(events_differ(&a, &b));
}

#[test]
fn collect_step_outcomes_returns_empty_for_empty_events() {
    let outcomes = collect_step_outcomes(&[]);
    assert!(outcomes.is_empty());
}

#[test]
fn collect_step_outcomes_ignores_non_step_relevant_events() {
    let events = vec![JournalEvent::RunAccepted {
        run: mk_run(1),
        seq: mk_seq(1),
        workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
    }];
    assert!(collect_step_outcomes(&events).is_empty());
}

#[test]
fn collect_step_outcomes_records_step_succeeded_with_output() {
    let events = vec![JournalEvent::StepSucceeded {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(5),
        output: mk_slot(3),
    }];
    let outcomes = collect_step_outcomes(&events);
    assert_eq!(outcomes.get(&5).unwrap(), "succeeded(output=3)");
}

#[test]
fn collect_step_outcomes_records_action_failed_with_action() {
    let events = vec![JournalEvent::ActionFailedEvent {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(2),
        action: mk_action(7),
        attempt: 1,
    }];
    let outcomes = collect_step_outcomes(&events);
    assert_eq!(outcomes.get(&2).unwrap(), "failed(action=7)");
}

#[test]
fn collect_step_outcomes_last_occurrence_wins_for_same_step() {
    let events = vec![
        JournalEvent::StepSucceeded {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            output: mk_slot(0),
        },
        JournalEvent::ActionFailedEvent {
            run: mk_run(1),
            seq: mk_seq(2),
            step: mk_step(1),
            action: mk_action(4),
            attempt: 1,
        },
    ];
    let outcomes = collect_step_outcomes(&events);
    assert_eq!(outcomes.get(&1).unwrap(), "failed(action=4)");
}

#[test]
fn collect_slot_values_records_decodable_slot_value() {
    let val = mk_slot_value(42);
    let bytes = postcard::to_allocvec(&val).unwrap();
    let events = vec![JournalEvent::SlotWrittenEvent {
        run: mk_run(1),
        seq: mk_seq(1),
        slot: mk_slot(2),
        value: Some(bytes),
        extra: None,
        attempt: 1,
    }];
    let slots = collect_slot_values(&events);
    assert_eq!(slots.get(&2).unwrap(), "42");
}

#[test]
fn collect_slot_values_displays_none_for_absent_value() {
    let events = vec![JournalEvent::SlotWrittenEvent {
        run: mk_run(1),
        seq: mk_seq(1),
        slot: mk_slot(3),
        value: None,
        extra: None,
        attempt: 1,
    }];
    let slots = collect_slot_values(&events);
    assert_eq!(slots.get(&3).unwrap(), "none");
}

#[test]
fn collect_slot_values_shows_byte_count_for_undecodable_bytes() {
    let events = vec![JournalEvent::SlotWrittenEvent {
        run: mk_run(1),
        seq: mk_seq(1),
        slot: mk_slot(5),
        value: Some(vec![0xFF; 3]),
        extra: None,
        attempt: 1,
    }];
    let slots = collect_slot_values(&events);
    assert_eq!(slots.get(&5).unwrap(), "[3 bytes]");
}

#[test]
fn compute_diff_returns_empty_for_identical_empty_streams() {
    let result = compute_diff(&[], &[]);
    assert_eq!(result.events_a, 0);
    assert_eq!(result.events_b, 0);
    assert!(result.diffs.is_empty());
}

#[test]
fn compute_diff_reports_only_in_a_when_b_is_shorter() {
    let a = vec![JournalEvent::RunAccepted {
        run: mk_run(1),
        seq: mk_seq(1),
        workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
    }];
    let result = compute_diff(&a, &[]);
    assert_eq!(result.events_a, 1);
    assert_eq!(result.events_b, 0);
    assert_eq!(result.diffs[0]["kind"].as_str().unwrap(), "only_in_a");
}

#[test]
fn compute_diff_reports_only_in_b_when_a_is_shorter() {
    let b = vec![JournalEvent::RunAccepted {
        run: mk_run(1),
        seq: mk_seq(1),
        workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
    }];
    let result = compute_diff(&[], &b);
    assert_eq!(result.diffs[0]["kind"].as_str().unwrap(), "only_in_b");
}

#[test]
fn compute_diff_reports_no_diff_for_identical_events() {
    let a = vec![JournalEvent::StepSucceeded {
        run: mk_run(1),
        seq: mk_seq(1),
        step: mk_step(1),
        output: mk_slot(0),
    }];
    let b = vec![JournalEvent::StepSucceeded {
        run: mk_run(2),
        seq: mk_seq(99),
        step: mk_step(1),
        output: mk_slot(0),
    }];
    let result = compute_diff(&a, &b);
    assert_eq!(result.diffs.len(), 0);
}

#[test]
fn compute_diff_detects_step_missing_in_b() {
    let a = vec![
        JournalEvent::StepSucceeded {
            run: mk_run(1),
            seq: mk_seq(1),
            step: mk_step(1),
            output: mk_slot(0),
        },
        JournalEvent::StepSucceeded {
            run: mk_run(1),
            seq: mk_seq(2),
            step: mk_step(2),
            output: mk_slot(0),
        },
    ];
    let b = vec![JournalEvent::StepSucceeded {
        run: mk_run(2),
        seq: mk_seq(99),
        step: mk_step(1),
        output: mk_slot(0),
    }];
    let result = compute_diff(&a, &b);
    let has_missing = result
        .diffs
        .iter()
        .any(|d| d["kind"].as_str() == Some("step_missing_in_b"));
    assert!(has_missing);
}

#[test]
fn compute_diff_detects_slot_value_differs() {
    let val_a = postcard::to_allocvec(&mk_slot_value(10)).unwrap();
    let val_b = postcard::to_allocvec(&mk_slot_value(20)).unwrap();
    let a = vec![JournalEvent::SlotWrittenEvent {
        run: mk_run(1),
        seq: mk_seq(1),
        slot: mk_slot(2),
        value: Some(val_a),
        extra: None,
        attempt: 1,
    }];
    let b = vec![JournalEvent::SlotWrittenEvent {
        run: mk_run(2),
        seq: mk_seq(1),
        slot: mk_slot(2),
        value: Some(val_b),
        extra: None,
        attempt: 1,
    }];
    let result = compute_diff(&a, &b);
    let has_slot_diff = result
        .diffs
        .iter()
        .any(|d| d["kind"].as_str() == Some("slot_value_differs"));
    assert!(has_slot_diff);
}

#[test]
fn compute_diff_correctly_reports_event_counts() {
    let a = vec![
        JournalEvent::RunAccepted {
            run: mk_run(1),
            seq: mk_seq(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
        },
        JournalEvent::StepStarted {
            run: mk_run(1),
            seq: mk_seq(2),
            step: mk_step(1),
            attempt: 1,
        },
    ];
    let b = vec![JournalEvent::RunAccepted {
        run: mk_run(2),
        seq: mk_seq(1),
        workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
    }];
    let result = compute_diff(&a, &b);
    assert_eq!(result.events_a, 2);
    assert_eq!(result.events_b, 1);
}
