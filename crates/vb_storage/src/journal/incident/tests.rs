use super::*;
use crate::EventSeq;
use crate::JournalEvent;
use vb_core::ActionTicket;
use vb_core::action::compute_action_idempotency_key;
use vb_core::ids::SeqNo;

/// Helper: create a minimal StepStarted event.
fn step_event(step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

/// Helper: create a minimal ActionCompletedEvent.
fn action_completed(step: u16, action: u16) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(2),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt: 1,
    }
}

/// Helper: create a minimal ActionFailedEvent.
fn action_failed(step: u16, action: u16) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(2),
        step: StepIdx::new(step),
        action: ActionId::new(action),
        attempt: 1,
    }
}

/// Helper: create a RunFailedEvent.
fn run_failed() -> JournalEvent {
    JournalEvent::RunFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(10),
        attempt: 1,
    }
}

/// Helper: create a RunCancelled event.
fn run_cancelled() -> JournalEvent {
    JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(10),
        attempt: 1,
        reason: None,
    }
}

// ---- T-001: Empty events ----
#[test]
fn t_001_empty_events() {
    let analysis = analyze_incident_events(&[]);
    assert!(!analysis.failure_found);
    assert_eq!(analysis.failure_code, "");
    assert!(analysis.failed_at_step.is_none());
    assert!(analysis.side_effects.is_empty());
}

// ---- T-002: RunFailedEvent ----
#[test]
fn t_002_run_failed_event() {
    let events = vec![step_event(1), run_failed()];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunFailed");
    assert_eq!(analysis.failed_at_step, Some(1));
}

// ---- T-003: RunCancelled ----
#[test]
fn t_003_run_cancelled() {
    let events = vec![step_event(1), step_event(2), run_cancelled()];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunCancelled");
    assert_eq!(analysis.failed_at_step, Some(2));
}

// ---- T-004: ActionCompletedEvent side effects ----
#[test]
fn t_004_action_completed_side_effects() {
    let events = vec![action_completed(1, 100)];
    let analysis = analyze_incident_events(&events);
    assert!(!analysis.failure_found);
    assert_eq!(analysis.side_effects.len(), 1);
    assert_eq!(analysis.side_effects[0].step, 1);
    assert_eq!(analysis.side_effects[0].action, 100);
    assert!(matches!(
        analysis.side_effects[0].certainty,
        SideEffectCertainty::Confirmed
    ));
}

// ---- T-005: ActionFailedEvent side effects ----
#[test]
fn t_005_action_failed_side_effects() {
    let events = vec![action_failed(2, 200)];
    let analysis = analyze_incident_events(&events);
    assert!(!analysis.failure_found);
    assert_eq!(analysis.side_effects.len(), 1);
    assert!(matches!(
        analysis.side_effects[0].certainty,
        SideEffectCertainty::Failed
    ));
}

// ---- T-006: Multiple events ----
#[test]
fn t_006_multiple_events() {
    let events = vec![
        step_event(1),
        action_completed(1, 10),
        action_failed(1, 20),
        step_event(2),
        action_completed(2, 30),
        run_failed(),
    ];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunFailed");
    assert_eq!(analysis.failed_at_step, Some(2));
    assert_eq!(analysis.side_effects.len(), 3);
}

// ---- T-007: Multiple StepStarted tracking ----
#[test]
fn t_007_multiple_step_started_tracking() {
    let events = vec![
        step_event(1),
        step_event(3),
        step_event(5),
        step_event(7),
        run_failed(),
    ];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failed_at_step, Some(7));
}

// ---- T-008: Mixed events ----
#[test]
fn t_008_mixed_events() {
    let events = vec![
        step_event(1),
        action_completed(1, 10),
        step_event(2),
        action_failed(2, 20),
        step_event(3),
        action_completed(3, 30),
        run_failed(),
    ];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.failure_code, "RunFailed");
    assert_eq!(analysis.failed_at_step, Some(3));
    assert_eq!(analysis.side_effects.len(), 3);
}

// ---- T-009: RunFailed repair hints (1 hint) ----
#[test]
fn t_009_run_failed_1_hint() {
    let hints = build_repair_hints("RunFailed", &[], None);
    assert_eq!(hints.len(), 1);
    assert_eq!(
        hints[0],
        "investigate step output and engine logs for the failed step"
    );
}

// ---- T-010: RunFailed repair hints (3 hints) ----
#[test]
fn t_010_run_failed_3_hints() {
    let side_effects = vec![SideEffect {
        step: 1,
        action: 0,
        certainty: SideEffectCertainty::Confirmed,
    }];
    let hints = build_repair_hints("RunFailed", &side_effects, Some(3));
    assert_eq!(hints.len(), 3);
    assert_eq!(
        hints[0],
        "investigate step output and engine logs for the failed step"
    );
    assert_eq!(
        hints[1],
        "review side effects that completed before failure for compensating actions"
    );
    assert_eq!(
        hints[2],
        "consider retry from step 3 using the retry command"
    );
}

// ---- T-011: RunCancelled repair hints (1 hint) ----
#[test]
fn t_011_run_cancelled_1_hint() {
    let hints = build_repair_hints("RunCancelled", &[], None);
    assert_eq!(hints.len(), 1);
    assert_eq!(
        hints[0],
        "run was cancelled; check if cancellation was intentional"
    );
}

// ---- T-012: RunCancelled repair hints (2 hints) ----
#[test]
fn t_012_run_cancelled_2_hints() {
    let side_effects = vec![SideEffect {
        step: 2,
        action: 0,
        certainty: SideEffectCertainty::Confirmed,
    }];
    let hints = build_repair_hints("RunCancelled", &side_effects, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(
        hints[0],
        "run was cancelled; check if cancellation was intentional"
    );
    assert_eq!(
        hints[1],
        "review completed side effects for partial cleanup needs"
    );
}

// ---- T-013: Unknown failure code (0 hints) ----
#[test]
fn t_013_unknown_failure_code() {
    let hints = build_repair_hints("UnknownError", &[], None);
    assert!(hints.is_empty());
}

/// Helper: create an ActionAbandoned event carrying a ticket for step/action.
fn action_abandoned(step: u16, action: u16) -> JournalEvent {
    let run = RunId::new(1);
    let seq = SeqNo::new(3);
    let action_id = ActionId::new(action);
    JournalEvent::ActionAbandoned {
        run,
        seq: EventSeq::new(4),
        ticket: ActionTicket {
            run,
            step: StepIdx::new(step),
            seq,
            action: action_id,
            run_attempt: 1,
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(run, 1, seq, action_id),
            capacity: 2,
            fencing_token: 0,
        },
    }
}

// ---- T-014: ActionAbandoned keeps ticket identity ----
#[test]
fn t_014_action_abandoned_keeps_ticket_identity() {
    let events = vec![action_abandoned(5, 9), run_cancelled()];
    let analysis = analyze_incident_events(&events);
    assert!(analysis.failure_found);
    assert_eq!(analysis.side_effects.len(), 1);
    assert_eq!(analysis.side_effects[0].step, 5);
    assert_eq!(analysis.side_effects[0].action, 9);
    assert!(matches!(
        analysis.side_effects[0].certainty,
        SideEffectCertainty::Confirmed
    ));
}
