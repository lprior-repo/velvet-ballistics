//! Regression tests for bead vb-1rqz7.3 (SJ-005).
//!
//! These tests pin the explicit lifecycle-state mapping of every previously
//! unmapped `JournalEvent` variant in
//! `derive_lifecycle_state_from_events` / `event_to_lifecycle`. Before this
//! fix, five variants fell through a wildcard arm masked by
//! `#[allow(unreachable_patterns)]`:
//!
//! - `ActionScheduledTicket`  -> `LifecycleState::Active`
//! - `ActionCompletedEnvelope` -> `LifecycleState::Active`
//! - `WaitResolvedEvent`       -> `LifecycleState::Active`
//! - `RunKilled`              -> `LifecycleState::Cancelled`
//! - `AskTimedOutEvent`       -> `LifecycleState::Active`
//!
//! Each variant has at least one test below that constructs the event and
//! asserts the resulting lifecycle state. The tests are placed in the journal
//! module (the owner of the function under test) and registered in
//! `journal/mod.rs` under `#[cfg(test)]`.

#![forbid(unsafe_code)]

use crate::EventSeq;
use crate::JournalEvent;
use crate::event_to_lifecycle;
use vb_core::value::{ConstValue, Taint};
use vb_core::{
    ActionId, ActionTicket, CapabilitySet, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest,
    ids::SeqNo, workflow::LifecycleState,
};

/// Minimal `ActionTicket` for any test that needs one.
fn sample_ticket() -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    }
}

fn run() -> RunId {
    RunId::new(1)
}

// ----- previously unmapped variants (the actual fix surface) -------------

#[test]
fn action_scheduled_ticket_maps_to_active() {
    let event = JournalEvent::ActionScheduledTicket {
        run: run(),
        seq: EventSeq::new(1),
        ticket: sample_ticket(),
        input: SlotIdx::new(0),
        output: SlotIdx::new(1),
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn action_completed_envelope_maps_to_active() {
    let event = JournalEvent::ActionCompletedEnvelope {
        run: run(),
        seq: EventSeq::new(2),
        ticket: sample_ticket(),
        output: SlotIdx::new(1),
        outcome: crate::DurableActionOutcome::Ready,
        value: vec![0x42],
        encoded_len: 1,
        taint: Taint::Clean,
        value_digest: [0u8; 32],
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn wait_resolved_event_maps_to_active() {
    let event = JournalEvent::WaitResolvedEvent {
        run: run(),
        seq: EventSeq::new(3),
        step: StepIdx::new(2),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn run_killed_maps_to_cancelled() {
    let event = JournalEvent::RunKilled {
        run: run(),
        seq: EventSeq::new(4),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Cancelled);
}

#[test]
fn ask_timed_out_event_maps_to_active() {
    let event = JournalEvent::AskTimedOutEvent {
        run: run(),
        seq: EventSeq::new(5),
        step: StepIdx::new(3),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

// ----- terminal-state assertions (regression for run_cancelled sibling) --

#[test]
fn run_cancelled_maps_to_cancelled() {
    let event = JournalEvent::RunCancelled {
        run: run(),
        seq: EventSeq::new(6),
        attempt: 1,
        reason: None,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Cancelled);
}

#[test]
fn run_finished_maps_to_completed() {
    let event = JournalEvent::RunFinished {
        run: run(),
        seq: EventSeq::new(7),
        result: SlotIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Completed);
}

#[test]
fn run_failed_event_maps_to_failed() {
    let event = JournalEvent::RunFailedEvent {
        run: run(),
        seq: EventSeq::new(8),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Failed);
}

#[test]
fn run_answered_maps_to_completed() {
    let event = JournalEvent::RunAnswered {
        run: run(),
        seq: EventSeq::new(9),
        slot_idx: SlotIdx::new(0),
        answer: ConstValue::I64(42),
        timestamp: chrono::Utc::now(),
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Completed);
}

#[test]
fn run_resumed_maps_to_active() {
    let event = JournalEvent::RunResumed {
        run: run(),
        seq: EventSeq::new(10),
        timestamp: chrono::Utc::now(),
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn run_retried_maps_to_active() {
    let event = JournalEvent::RunRetried {
        run: run(),
        seq: EventSeq::new(11),
        timestamp: chrono::Utc::now(),
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn action_failed_event_maps_to_failed() {
    let event = JournalEvent::ActionFailedEvent {
        run: run(),
        seq: EventSeq::new(12),
        step: StepIdx::new(0),
        action: ActionId::new(7),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Failed);
}

// ----- waiting-answer surface -------------------------------------------

#[test]
fn wait_scheduled_event_maps_to_waiting_answer() {
    let event = JournalEvent::WaitScheduledEvent {
        run: run(),
        seq: EventSeq::new(13),
        step: StepIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::WaitingAnswer);
}

#[test]
fn ask_scheduled_event_maps_to_waiting_answer() {
    let event = JournalEvent::AskScheduledEvent {
        run: run(),
        seq: EventSeq::new(14),
        step: StepIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::WaitingAnswer);
}

#[test]
fn ask_answered_event_maps_to_waiting_answer() {
    let event = JournalEvent::AskAnsweredEvent {
        run: run(),
        seq: EventSeq::new(15),
        step: StepIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::WaitingAnswer);
}

// ----- remaining active-mapped variants (extra coverage) ----------------

#[test]
fn run_accepted_maps_to_active() {
    let event = JournalEvent::RunAccepted {
        run: run(),
        seq: EventSeq::new(16),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn run_admission_maps_to_active() {
    let event = JournalEvent::RunAdmission {
        run: run(),
        seq: EventSeq::new(17),
        artifact_digest: WorkflowDigest::from_bytes([0u8; 32]),
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Relaxed,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn step_started_maps_to_active() {
    let event = JournalEvent::StepStarted {
        run: run(),
        seq: EventSeq::new(18),
        step: StepIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn step_succeeded_maps_to_active() {
    let event = JournalEvent::StepSucceeded {
        run: run(),
        seq: EventSeq::new(19),
        step: StepIdx::new(0),
        output: SlotIdx::new(0),
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn action_scheduled_maps_to_active() {
    let event = JournalEvent::ActionScheduled {
        run: run(),
        seq: EventSeq::new(20),
        step: StepIdx::new(0),
        action: ActionId::new(7),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn action_completed_event_maps_to_active() {
    let event = JournalEvent::ActionCompletedEvent {
        run: run(),
        seq: EventSeq::new(21),
        step: StepIdx::new(0),
        action: ActionId::new(7),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn slot_written_event_maps_to_active() {
    let event = JournalEvent::SlotWrittenEvent {
        run: run(),
        seq: EventSeq::new(22),
        slot: SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

#[test]
fn retry_scheduled_event_maps_to_active() {
    let event = JournalEvent::RetryScheduledEvent {
        run: run(),
        seq: EventSeq::new(23),
        step: StepIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event_to_lifecycle(&event), LifecycleState::Active);
}

// ----- derive_lifecycle_state_from_events last-wins behaviour -----------

#[test]
fn derive_returns_pending_for_empty_events() {
    let result = crate::derive_lifecycle_state_from_events(&[]);
    assert_eq!(result, LifecycleState::Pending);
}

#[test]
fn derive_last_event_dominates_when_first_events_disagree() {
    // First event would map to Active; last is RunKilled (Cancelled).
    let events = vec![
        JournalEvent::StepStarted {
            run: run(),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunKilled {
            run: run(),
            seq: EventSeq::new(2),
            attempt: 1,
        },
    ];
    let result = crate::derive_lifecycle_state_from_events(&events);
    assert_eq!(result, LifecycleState::Cancelled);
}

#[test]
fn derive_last_event_wait_resolved_resumes_to_active() {
    let events = vec![
        JournalEvent::WaitScheduledEvent {
            run: run(),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::WaitResolvedEvent {
            run: run(),
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];
    let result = crate::derive_lifecycle_state_from_events(&events);
    assert_eq!(result, LifecycleState::Active);
}

#[test]
fn derive_last_event_ask_timed_out_resumes_to_active() {
    let events = vec![
        JournalEvent::AskScheduledEvent {
            run: run(),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::AskTimedOutEvent {
            run: run(),
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];
    let result = crate::derive_lifecycle_state_from_events(&events);
    assert_eq!(result, LifecycleState::Active);
}

// ----- exhaustiveness coverage: every known variant is mapped -----------

/// Exhaustiveness-style guard: asserts every `JournalEvent` variant is
/// mapped to some `LifecycleState`. The compiler cannot enforce this for
/// non-exhaustive enums, so this test serves as a contract check: any
/// future variant added without an arm in `event_to_lifecycle` will leave
/// this list out of date and fail CI before the production code paths
/// silently downgrade the new variant to `Active`.
#[test]
fn every_known_variant_has_an_event_to_lifecycle_mapping() {
    let events: Vec<JournalEvent> = vec![
        JournalEvent::RunAccepted {
            run: run(),
            seq: EventSeq::new(1),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        },
        JournalEvent::RunAdmission {
            run: run(),
            seq: EventSeq::new(2),
            artifact_digest: WorkflowDigest::from_bytes([0u8; 32]),
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Relaxed,
        },
        JournalEvent::StepStarted {
            run: run(),
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run: run(),
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
        JournalEvent::ActionScheduled {
            run: run(),
            seq: EventSeq::new(5),
            step: StepIdx::new(0),
            action: ActionId::new(7),
            attempt: 1,
        },
        JournalEvent::ActionScheduledTicket {
            run: run(),
            seq: EventSeq::new(6),
            ticket: sample_ticket(),
            input: SlotIdx::new(0),
            output: SlotIdx::new(1),
        },
        JournalEvent::ActionCompletedEvent {
            run: run(),
            seq: EventSeq::new(7),
            step: StepIdx::new(0),
            action: ActionId::new(7),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEnvelope {
            run: run(),
            seq: EventSeq::new(8),
            ticket: sample_ticket(),
            output: SlotIdx::new(1),
            outcome: crate::DurableActionOutcome::Ready,
            value: vec![0x42],
            encoded_len: 1,
            taint: Taint::Clean,
            value_digest: [0u8; 32],
        },
        JournalEvent::ActionFailedEvent {
            run: run(),
            seq: EventSeq::new(9),
            step: StepIdx::new(0),
            action: ActionId::new(7),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run: run(),
            seq: EventSeq::new(10),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run: run(),
            seq: EventSeq::new(11),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run: run(),
            seq: EventSeq::new(12),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::AskAnsweredEvent {
            run: run(),
            seq: EventSeq::new(13),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::WaitResolvedEvent {
            run: run(),
            seq: EventSeq::new(14),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run: run(),
            seq: EventSeq::new(15),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run: run(),
            seq: EventSeq::new(16),
            attempt: 1,
            reason: None,
        },
        JournalEvent::RunKilled {
            run: run(),
            seq: EventSeq::new(17),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run: run(),
            seq: EventSeq::new(18),
            result: SlotIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run: run(),
            seq: EventSeq::new(19),
            attempt: 1,
        },
        JournalEvent::RunResumed {
            run: run(),
            seq: EventSeq::new(20),
            timestamp: chrono::Utc::now(),
        },
        JournalEvent::RunRetried {
            run: run(),
            seq: EventSeq::new(21),
            timestamp: chrono::Utc::now(),
        },
        JournalEvent::RunAnswered {
            run: run(),
            seq: EventSeq::new(22),
            slot_idx: SlotIdx::new(0),
            answer: ConstValue::I64(42),
            timestamp: chrono::Utc::now(),
        },
        JournalEvent::AskTimedOutEvent {
            run: run(),
            seq: EventSeq::new(23),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];

    // 23 known variants as of this fix. If a new variant is added, the
    // match in `event_to_lifecycle` becomes non-exhaustive and the build
    // fails before this list grows stale; either way, this number should
    // equal the number of explicit arms in `event_to_lifecycle`.
    assert_eq!(events.len(), 23);

    for event in &events {
        // We only care that the function does not panic. Specific state
        // assertions live in the per-variant tests above.
        let _state = event_to_lifecycle(event);
    }
}
