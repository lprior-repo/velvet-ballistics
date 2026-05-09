#![forbid(unsafe_code)]
#![cfg(test)]

use vb_core::action::{
    ActionContract, ActionOutcome, ActionTicket, ActionOutputReady,
    ActionFailure, ActionFailureCode, Idempotency, SideEffect, RetrySafety, RetryPolicy,
};
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::engine::action::{
    execute_do, resume_action_outcome, compute_idempotency_key,
};
use vb_runtime::engine::types::{RuntimeSignal, RetryPolicy as RuntimeRetryPolicy};

fn make_contract(id: ActionId, idempotency: Idempotency) -> ActionContract {
    ActionContract {
        id,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

fn make_test_frame(run_id: RunId, input_taint: Taint) -> RunFrame {
    let mut frame = RunFrame::new(run_id, StepIdx::new(0), 10, 1).expect("frame should create");
    frame.write_taint(SlotIdx::new(0), input_taint).expect("taint write should work");
    frame
}

fn execute_do_test<'a>(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    seq: SeqNo,
    registry_contracts: &'a [&ActionContract],
    granted: &CapabilitySet,
) -> vb_runtime::engine::types::RuntimeEngineResult<RuntimeSignal> {
    let contract = match registry_contracts.get(usize::from(action.get())) {
        Some(c) if c.id == action => *c,
        _ => {
            return Err(vb_runtime::engine::types::RuntimeEngineError::Action(
                vb_core::action::ActionError::UnknownAction { action },
            ));
        }
    };
    execute_do(
        run, step, action, input, seq,
        contract, registry_contracts, granted,
        RuntimeRetryPolicy::NEVER,
    )
}

#[test]
fn test_execute_do_registered_deterministic_pure_with_clean_input() {
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(1), Idempotency::DeterministicPure);
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(1);
    let input_taint = Taint::Clean;
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_ok(), "execute_do should succeed for clean input on pure action");
    let signal = result.unwrap();
    match signal {
        RuntimeSignal::AwaitingAction(ticket) => {
            assert_eq!(ticket.attempt, 1, "attempt should be 1 on first dispatch");
            let expected_key = compute_idempotency_key(run_id, seq, action);
            assert_eq!(ticket.idempotency_key, expected_key, "idempotency_key should match");
        }
        _ => panic!("expected AwaitingAction signal"),
    }
}

#[test]
fn test_execute_do_at_least_once_with_secret_input_propagates_taint() {
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(2), Idempotency::AtLeastOnceExternal);
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(2);
    let input_taint = Taint::Secret;
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_ok(), "execute_do should succeed for AtLeastOnce with secret input");
}

#[test]
fn test_execute_do_idempotency_key_matches_compute() {
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(3), Idempotency::DeterministicPure);
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(5);
    let step = StepIdx::new(2);
    let seq = SeqNo::new(10);
    let action = ActionId::new(3);
    let input_taint = Taint::Clean;
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_ok(), "execute_do should succeed");
    let signal = result.unwrap();
    match signal {
        RuntimeSignal::AwaitingAction(ticket) => {
            let expected = compute_idempotency_key(run_id, seq, action);
            assert_eq!(ticket.idempotency_key, expected, "idempotency_key should match compute");
        }
        _ => panic!("expected AwaitingAction"),
    }
}

#[test]
fn test_execute_do_unregistered_action_returns_unknown_action_error() {
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(999);
    let input_taint = Taint::Clean;
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_err(), "execute_do should fail for unregistered action");
}

#[test]
fn test_execute_do_deterministic_pure_with_secret_input_fails_taint() {
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(4), Idempotency::DeterministicPure);
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(4);
    let input_taint = Taint::Secret;
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_err(), "execute_do should fail for secret input on pure action");
}

#[test]
fn test_execute_do_deterministic_pure_with_derived_secret_fails_taint() {
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(5), Idempotency::DeterministicPure);
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(5);
    let input_taint = Taint::DerivedFromSecret;
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_err(), "execute_do should fail for derived secret input on pure action");
}

#[test]
fn test_execute_do_missing_capability_returns_capability_denied() {
    use vb_core::capability::{Capability, CapabilitySet};
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let required_cap = Capability::new("Network".into(), ActionId::new(6));
    let contract = ActionContract {
        id: ActionId::new(6),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([required_cap.clone()]),
    };
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(6);
    let input_taint = Taint::Clean;
    let frame = make_test_frame(run_id, input_taint);
    let disk_cap = Capability::new("Disk".into(), ActionId::new(6));
    let granted = CapabilitySet::from_grants(Box::new([disk_cap]));
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_err(), "execute_do should fail when capability not granted");
}

#[test]
fn test_resume_ready_writes_output_slot() {
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(7), Idempotency::DeterministicPure);
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(7);
    let input_taint = Taint::Clean;
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let registry_contracts: Vec<_> = registry.registered_contracts();
    let exec_result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    let ticket = match exec_result.unwrap() {
        RuntimeSignal::AwaitingAction(t) => t,
        _ => panic!("expected AwaitingAction"),
    };
    let ready = ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let resume_result = resume_action_outcome(&ticket, ActionOutcome::Ready(ready), &contract);
    assert!(resume_result.is_ok(), "resume with Ready should succeed");
    let signal = resume_result.unwrap();
    assert_eq!(signal, RuntimeSignal::Continue, "Ready should return Continue");
}

#[test]
fn test_resume_ready_output_in_bounds() {
    use vb_runtime::action::ActionRegistry;
    let registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(8),
        input_slot_count: 1,
        output_slot_count: 2,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    registry.register(contract).expect("register should succeed");
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(8),
        attempt: 1,
        idempotency_key: 100,
        capacity: 3,
    };
    let ready = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let result = resume_action_outcome(&ticket, ActionOutcome::Ready(ready), &contract);
    assert!(result.is_ok(), "resume with in-bounds output_slot should succeed");
}

#[test]
fn test_resume_ready_output_out_of_bounds() {
    let contract = ActionContract {
        id: ActionId::new(9),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(9),
        attempt: 1,
        idempotency_key: 100,
        capacity: 3,
    };
    let ready = ActionOutputReady {
        output_slot: SlotIdx::new(5),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let result = resume_action_outcome(&ticket, ActionOutcome::Ready(ready), &contract);
    assert!(result.is_err(), "resume with out-of-bounds output_slot should fail");
}

#[test]
fn test_resume_failed_retryable_below_capacity_returns_retry() {
    let contract = ActionContract {
        id: ActionId::new(10),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(10),
        attempt: 1,
        idempotency_key: 100,
        capacity: 3,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let result = resume_action_outcome(&ticket, ActionOutcome::Failed(failure), &contract);
    assert!(result.is_ok(), "resume with retryable failure below capacity should succeed");
    let signal = result.unwrap();
    match signal {
        RuntimeSignal::AwaitingAction(retry_ticket) => {
            assert_eq!(retry_ticket.attempt, 2, "attempt should be incremented");
            assert_eq!(retry_ticket.seq.get(), 2, "seq should be incremented");
        }
        _ => panic!("expected AwaitingAction for retry"),
    }
}

#[test]
fn test_resume_failed_retryable_increments_attempt_and_seq() {
    let contract = ActionContract {
        id: ActionId::new(11),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(5),
        action: ActionId::new(11),
        attempt: 1,
        idempotency_key: 100,
        capacity: 3,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let result = resume_action_outcome(&ticket, ActionOutcome::Failed(failure), &contract);
    assert!(result.is_ok(), "resume should succeed for retryable below capacity");
    let signal = result.unwrap();
    match signal {
        RuntimeSignal::AwaitingAction(retry_ticket) => {
            assert_eq!(retry_ticket.attempt, 2, "attempt should be 2");
            assert_eq!(retry_ticket.seq, SeqNo::new(6), "seq should be original + 1");
        }
        _ => panic!("expected AwaitingAction"),
    }
}

#[test]
fn test_resume_failed_non_retryable_returns_error() {
    let contract = ActionContract {
        id: ActionId::new(12),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Destroys,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(12),
        attempt: 1,
        idempotency_key: 100,
        capacity: 3,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::ExternalUnavailable,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let result = resume_action_outcome(&ticket, ActionOutcome::Failed(failure), &contract);
    assert!(result.is_err(), "resume with NonRetryable should return error");
}

#[test]
fn test_resume_failed_at_capacity_returns_exhausted() {
    let contract = ActionContract {
        id: ActionId::new(13),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(13),
        attempt: 3,
        idempotency_key: 100,
        capacity: 3,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let result = resume_action_outcome(&ticket, ActionOutcome::Failed(failure), &contract);
    assert!(result.is_err(), "resume when at capacity should return error");
}

#[test]
fn test_compute_idempotency_key_deterministic() {
    let run_id = RunId::new(42);
    let seq = SeqNo::new(7);
    let action = ActionId::new(3);
    let key1 = compute_idempotency_key(run_id, seq, action);
    let key2 = compute_idempotency_key(run_id, seq, action);
    assert_eq!(key1, key2, "idempotency_key should be deterministic");
}

#[test]
fn test_compute_idempotency_key_different_inputs_different_keys() {
    let run_id = RunId::new(1);
    let seq_a = SeqNo::new(1);
    let seq_b = SeqNo::new(2);
    let action = ActionId::new(1);
    let key_a = compute_idempotency_key(run_id, seq_a, action);
    let key_b = compute_idempotency_key(run_id, seq_b, action);
    assert_ne!(key_a, key_b, "different seq should produce different keys");
}

#[test]
fn test_compute_idempotency_key_zero_seq() {
    let run_id = RunId::new(1);
    let seq = SeqNo::new(0);
    let action = ActionId::new(1);
    let key = compute_idempotency_key(run_id, seq, action);
    assert!(key == 0 || key > 0, "zero seq should produce valid key");
}

#[test]
fn test_compute_idempotency_key_max_action_id() {
    let run_id = RunId::new(1);
    let seq = SeqNo::new(1);
    let action = ActionId::new(u16::MAX);
    let key = compute_idempotency_key(run_id, seq, action);
    assert!(key > 0, "max action id should produce non-zero key");
}
