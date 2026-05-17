//! Tests for runtime engine types.
//!
//! These tests verify invariants across a range of inputs,
//! ensuring the engine types behave correctly under edge cases and boundary
//! conditions.

#![forbid(unsafe_code)]

use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};

use crate::engine::types::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
};

// =============================================================================
// EvidenceCollector tests
// =============================================================================

#[test]
fn evidence_collector_zero_capacity_drops_all() {
    let mut collector = EvidenceCollector::with_capacity(0);
    collector.push_step_started(StepIdx::ZERO);
    assert_eq!(collector.len(), 0);
    assert_eq!(collector.dropped(), 1);

    collector.push_step_succeeded(StepIdx::ZERO, None);
    assert_eq!(collector.dropped(), 2);

    collector.push_slot_written(SlotIdx::ZERO, SlotValue::I64(42));
    assert_eq!(collector.dropped(), 3);
}

#[test]
fn evidence_collector_capacity_one_tracks_dropped() {
    let mut collector = EvidenceCollector::with_capacity(1);
    collector.push_step_started(StepIdx::ZERO);
    assert_eq!(collector.len(), 1);
    assert_eq!(collector.dropped(), 0);

    collector.push_step_started(StepIdx::new(1));
    assert_eq!(collector.len(), 1);
    assert!(collector.dropped() >= 1);
}

#[test]
fn evidence_collector_double_drain_returns_empty_second_time() {
    let mut collector = EvidenceCollector::new();
    collector.push_step_started(StepIdx::ZERO);
    let first = collector.drain();
    assert_eq!(first.len(), 1);
    let second = collector.drain();
    assert_eq!(second.len(), 0);
}

#[test]
fn evidence_collector_slot_written_with_taint_preserves_values() {
    let mut collector = EvidenceCollector::new();
    let slot = SlotIdx::new(5);
    let val = SlotValue::I64(123);
    let taint = Taint::Secret;

    collector.push_slot_written_with_taint(slot, val, taint);
    let events = collector.drain();
    assert_eq!(events.len(), 1);

    match &events[0] {
        EvidenceEvent::SlotWritten {
            slot: s,
            value: v,
            taint: t,
            ..
        } => {
            assert_eq!(*s, slot);
            assert_eq!(*v, val);
            assert_eq!(*t, taint);
        }
        other => panic!("expected SlotWritten event, got {:?}", other),
    }
}

// =============================================================================
// RetryPolicy tests
// =============================================================================

#[test]
fn retry_policy_fields_are_bounded() {
    let policy = RetryPolicy {
        max_attempts: 50,
        base_delay_ms: 5000,
        exponential_backoff: true,
    };
    assert_eq!(policy.max_attempts, 50);
    assert_eq!(policy.base_delay_ms, 5000);
    assert!(policy.exponential_backoff);
}

#[test]
fn retry_policy_never_has_correct_values() {
    let policy = RetryPolicy::NEVER;
    assert_eq!(policy.max_attempts, 1);
    assert_eq!(policy.base_delay_ms, 0);
    assert!(!policy.exponential_backoff);
}

#[test]
fn retry_policy_default_has_correct_values() {
    let policy = RetryPolicy::DEFAULT;
    assert_eq!(policy.max_attempts, 3);
    assert_eq!(policy.base_delay_ms, 100);
    assert!(!policy.exponential_backoff);
}

// =============================================================================
// RuntimeEngineError tests
// =============================================================================

#[test]
fn runtime_engine_error_retry_exhausted_fields() {
    let error = RuntimeEngineError::RetryExhausted {
        action: ActionId::new(42),
        attempts: 5,
    };
    assert_eq!(error.runtime_code(), Some("RETRY_EXHAUSTED"));
}

#[test]
fn runtime_engine_error_taint_violation_fields() {
    let error = RuntimeEngineError::TaintViolation {
        step: StepIdx::new(7),
    };
    assert_eq!(error.runtime_code(), None);
}

#[test]
fn runtime_engine_error_branch_limit_fields() {
    let error = RuntimeEngineError::BranchLimitExceeded {
        max: 100,
        requested: 200,
    };
    assert_eq!(error.runtime_code(), Some("BRANCH_LIMIT_EXCEEDED"));
}

// =============================================================================
// RuntimeSignal tests
// =============================================================================

#[test]
fn runtime_signal_continue_is_not_finished() {
    assert_ne!(
        RuntimeSignal::Continue,
        RuntimeSignal::Finished(SlotValue::Null)
    );
}

#[test]
fn runtime_signal_finished_equality() {
    let signal1 = RuntimeSignal::Finished(SlotValue::I64(42));
    let signal2 = RuntimeSignal::Finished(SlotValue::I64(42));
    assert_eq!(signal1, signal2);
}

#[test]
fn runtime_signal_finished_inequality() {
    let signal1 = RuntimeSignal::Finished(SlotValue::I64(1));
    let signal2 = RuntimeSignal::Finished(SlotValue::I64(2));
    assert_ne!(signal1, signal2);
}

#[test]
fn runtime_signal_step_budget_exhausted_is_not_finished() {
    assert_ne!(
        RuntimeSignal::StepBudgetExhausted,
        RuntimeSignal::Finished(SlotValue::Null)
    );
}

#[test]
fn runtime_signal_awaiting_wait_is_not_finished() {
    assert_ne!(
        RuntimeSignal::AwaitingWait,
        RuntimeSignal::Finished(SlotValue::Null)
    );
}

#[test]
fn runtime_signal_awaiting_ask_is_not_finished() {
    assert_ne!(
        RuntimeSignal::AwaitingAsk,
        RuntimeSignal::Finished(SlotValue::Null)
    );
}

// =============================================================================
// EvidenceEvent tests
// =============================================================================

#[test]
fn evidence_event_step_started_step_preserved() {
    let event = EvidenceEvent::StepStarted {
        step: StepIdx::new(99),
    };

    match event {
        EvidenceEvent::StepStarted { step: s } => {
            assert_eq!(s, StepIdx::new(99));
        }
        other => panic!("expected StepStarted, got {:?}", other),
    }
}

#[test]
fn evidence_event_step_succeeded_output_preserved() {
    let event = EvidenceEvent::StepSucceeded {
        step: StepIdx::new(3),
        output: Some(SlotIdx::new(7)),
    };

    match &event {
        EvidenceEvent::StepSucceeded { step: s, output: o } => {
            assert_eq!(*s, StepIdx::new(3));
            assert_eq!(*o, Some(SlotIdx::new(7)));
        }
        other => panic!("expected StepSucceeded, got {:?}", other),
    }
}

#[test]
fn evidence_event_slot_written_all_fields_preserved() {
    let event = EvidenceEvent::SlotWritten {
        slot: SlotIdx::new(2),
        value: SlotValue::I64(999),
        taint: Taint::DerivedFromSecret,
        extra: None,
    };

    match &event {
        EvidenceEvent::SlotWritten {
            slot: s,
            value: v,
            taint: t,
            ..
        } => {
            assert_eq!(*s, SlotIdx::new(2));
            assert_eq!(*v, SlotValue::I64(999));
            assert_eq!(*t, Taint::DerivedFromSecret);
        }
        other => panic!("expected SlotWritten, got {:?}", other),
    }
}
