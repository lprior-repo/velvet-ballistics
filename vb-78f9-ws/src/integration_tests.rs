#![forbid(unsafe_code)]
#![cfg(test)]

use vb_core::action::{
    ActionContract, ActionId, ActionInput, ActionOutcome, ActionOutputReady,
    ActionFailure, ActionFailureCode, ActionTicket, Idempotency, SideEffect,
    RetrySafety, RetryPolicy, RunId, SeqNo, StepIdx, SlotIdx, SlotValue, Taint,
};
use vb_runtime::action::ActionRegistry;
use vb_runtime::engine::action::{execute_do, resume_action_outcome};

fn make_contract(id: ActionId, idempotency: Idempotency, retry_safety: RetrySafety, capacity: u16) -> ActionContract {
    ActionContract {
        id,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency,
        side_effect: if idempotency == Idempotency::DeterministicPure {
            SideEffect::None
        } else {
            SideEffect::Writes
        },
        retry_safety,
        required_capabilities: Box::new([]),
    }
}

#[test]
fn test_full_action_lifecycle() {
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(1), Idempotency::DeterministicPure, RetrySafety::Safe, 1);
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(1);
    let input_taint = Taint::Clean;
    let granted: Vec<_> = vec![];
    let result = execute_do(run_id, step, seq, action, input_taint, &granted, &registry);
    assert!(result.is_ok(), "execute_do should succeed");
    let signal = result.unwrap();
    let ticket = match signal {
        vb_core::engine::RuntimeSignal::AwaitingAction(t) => t,
        _ => panic!("expected AwaitingAction"),
    };
    let ready = ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let resume_result = resume_action_outcome(&ticket, ActionOutcome::Ready(ready), &contract);
    assert!(resume_result.is_ok(), "resume should succeed");
    let tracker = vb_runtime::action::IdempotencyTracker::new(100);
    let mark_result = tracker.mark_completed(ticket);
    assert!(mark_result.is_ok(), "mark_completed should succeed");
    assert!(tracker.is_completed(&ticket), "is_completed should be true");
}

#[test]
fn test_retry_flow_within_capacity() {
    let registry = ActionRegistry::new();
    let contract = make_contract(
        ActionId::new(2),
        Idempotency::AtLeastOnceExternal,
        RetrySafety::KeyRequired,
        3,
    );
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(2);
    let input_taint = Taint::Clean;
    let granted: Vec<_> = vec![];
    let result = execute_do(run_id, step, seq, action, input_taint, &granted, &registry);
    assert!(result.is_ok(), "execute_do should succeed");
    let ticket1 = match result.unwrap() {
        vb_core::engine::RuntimeSignal::AwaitingAction(t) => t,
        _ => panic!("expected AwaitingAction"),
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let retry_result = resume_action_outcome(&ticket1, ActionOutcome::Failed(failure), &contract);
    assert!(retry_result.is_ok(), "first retry should succeed");
    let ticket2 = match retry_result.unwrap() {
        vb_core::engine::RuntimeSignal::AwaitingAction(t) => t,
        _ => panic!("expected AwaitingAction for retry"),
    };
    assert_eq!(ticket2.attempt, 2, "second attempt should be 2");
    let second_failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let retry_result2 = resume_action_outcome(&ticket2, ActionOutcome::Failed(second_failure), &contract);
    assert!(retry_result2.is_ok(), "second retry should succeed");
    let ticket3 = match retry_result2.unwrap() {
        vb_core::engine::RuntimeSignal::AwaitingAction(t) => t,
        _ => panic!("expected AwaitingAction for retry"),
    };
    assert_eq!(ticket3.attempt, 3, "third attempt should be 3");
    let ready = ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let final_result = resume_action_outcome(&ticket3, ActionOutcome::Ready(ready), &contract);
    assert!(final_result.is_ok(), "final resume should succeed");
}

#[test]
fn test_execute_do_capability_check_blocks_ungranted() {
    use vb_core::capability::Capability;
    let registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(3),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([Capability::Network]),
    };
    registry.register(contract).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(3);
    let input_taint = Taint::Clean;
    let granted = vec![Capability::Disk];
    let result = execute_do(run_id, step, seq, action, input_taint, &granted, &registry);
    assert!(result.is_err(), "execute_do should fail when capability not granted");
}

#[test]
fn test_execute_do_capability_check_passes_with_matching_grant() {
    use vb_core::capability::Capability;
    let registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(4),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([Capability::Network]),
    };
    registry.register(contract).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(4);
    let input_taint = Taint::Clean;
    let granted = vec![Capability::Network, Capability::Disk];
    let result = execute_do(run_id, step, seq, action, input_taint, &granted, &registry);
    assert!(result.is_ok(), "execute_do should succeed when capability is granted");
}

#[test]
fn test_taint_propagates_through_multiple_at_least_once_actions() {
    let registry = ActionRegistry::new();
    let pure_contract = make_contract(ActionId::new(10), Idempotency::DeterministicPure, RetrySafety::Safe, 1);
    let at_least_once_contract = make_contract(
        ActionId::new(11),
        Idempotency::AtLeastOnceExternal,
        RetrySafety::KeyRequired,
        1,
    );
    registry.register(pure_contract).expect("pure register should succeed");
    registry.register(at_least_once_contract).expect("at_least_once register should succeed");
    let run_id = RunId::new(1);
    let granted: Vec<_> = vec![];
    let result1 = execute_do(run_id, StepIdx::new(0), SeqNo::new(1), ActionId::new(10), Taint::Clean, &granted, &registry);
    assert!(result1.is_ok(), "pure action with clean input should succeed");
    let result2 = execute_do(run_id, StepIdx::new(1), SeqNo::new(2), ActionId::new(11), Taint::Secret, &granted, &registry);
    assert!(result2.is_ok(), "at_least_once action should handle secret input");
    let result3 = execute_do(run_id, StepIdx::new(2), SeqNo::new(3), ActionId::new(10), Taint::DerivedFromSecret, &granted, &registry);
    assert!(result3.is_err(), "pure action with derived secret input should fail");
}

#[test]
fn test_secret_input_blocks_pure_action() {
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(20), Idempotency::DeterministicPure, RetrySafety::Safe, 1);
    registry.register(contract).expect("register should succeed");
    let run_id = RunId::new(1);
    let granted: Vec<_> = vec![];
    let result = execute_do(run_id, StepIdx::new(0), SeqNo::new(1), ActionId::new(20), Taint::Secret, &granted, &registry);
    assert!(result.is_err(), "pure action with secret input should fail");
}

#[test]
fn test_clean_input_passes_through_pure_action() {
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(21), Idempotency::DeterministicPure, RetrySafety::Safe, 1);
    registry.register(contract).expect("register should succeed");
    let run_id = RunId::new(1);
    let granted: Vec<_> = vec![];
    let result = execute_do(run_id, StepIdx::new(0), SeqNo::new(1), ActionId::new(21), Taint::Clean, &granted, &registry);
    assert!(result.is_ok(), "pure action with clean input should succeed");
    let signal = result.unwrap();
    match signal {
        vb_core::engine::RuntimeSignal::AwaitingAction(_) => {}
        _ => panic!("expected AwaitingAction"),
    }
}
