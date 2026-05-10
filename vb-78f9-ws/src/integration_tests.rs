#![forbid(unsafe_code)]
#![cfg(test)]

use vb_core::action::{
    ActionContract, ActionInput, ActionOutcome, ActionOutputReady,
    ActionFailure, ActionFailureCode, ActionTicket, Idempotency, SideEffect,
    RetrySafety, RetryPolicy,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::action::ActionRegistry;
use vb_runtime::engine::action::{execute_do, resume_action_outcome};
use vb_runtime::engine::types::{RuntimeSignal, RetryPolicy as RuntimeRetryPolicy};

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

fn make_test_frame(run_id: RunId, input_taint: Taint) -> RunFrame {
    let mut frame = RunFrame::new(run_id, StepIdx::new(0), 10, 1).expect("frame should create");
    frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), input_taint)
        .expect("slot init should work");
    frame
}

fn build_indexed_registry(contracts: &[ActionContract]) -> Vec<ActionContract> {
    let max_id = contracts.iter().map(|c| c.id.get()).max().unwrap_or(0) as usize;
    let mut indexed = vec![
        ActionContract {
            id: ActionId::new(0),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        max_id + 1
    ];
    for contract in contracts {
        let idx = contract.id.get() as usize;
        indexed[idx] = contract.clone();
    }
    indexed
}

fn execute_do_test(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    seq: SeqNo,
    registry_contracts: &[ActionContract],
    granted: &CapabilitySet,
) -> vb_runtime::engine::types::RuntimeEngineResult<RuntimeSignal> {
    execute_do_test_with_retry(run, step, action, input, seq, registry_contracts, granted, RuntimeRetryPolicy::NEVER)
}

fn execute_do_test_with_retry(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    seq: SeqNo,
    registry_contracts: &[ActionContract],
    granted: &CapabilitySet,
    retry_policy: RuntimeRetryPolicy,
) -> vb_runtime::engine::types::RuntimeEngineResult<RuntimeSignal> {
    let contract = match registry_contracts.iter().find(|c| c.id == action) {
        Some(c) => c,
        None => {
            return Err(vb_runtime::engine::types::RuntimeEngineError::Action(
                vb_core::action::ActionError::UnknownAction { action },
            ));
        }
    };
    execute_do(
        run, step, action, input, seq,
        contract, registry_contracts, granted,
        retry_policy,
    )
}

#[test]
fn test_full_action_lifecycle() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(1), Idempotency::DeterministicPure, RetrySafety::Safe, 1);
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(1);
    let input_taint = Taint::Clean;
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let registry_contracts = build_indexed_registry(&contracts);
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_ok(), "execute_do should succeed");
    let signal = result.unwrap();
    let ticket = match signal {
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
    assert!(resume_result.is_ok(), "resume should succeed");
    let mut tracker = vb_runtime::action::IdempotencyTracker::new(100);
    let mark_result = tracker.mark_completed(&ticket);
    assert!(mark_result.is_ok(), "mark_completed should succeed");
    assert!(tracker.is_completed(&ticket), "is_completed should be true");
}

#[test]
fn test_retry_flow_within_capacity() {
    let mut registry = ActionRegistry::new();
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
    let frame = make_test_frame(run_id, input_taint);
    let granted = CapabilitySet::empty();
    let contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let registry_contracts = build_indexed_registry(&contracts);
    let result = execute_do_test_with_retry(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted, RuntimeRetryPolicy::DEFAULT);
    assert!(result.is_ok(), "execute_do should succeed");
    let ticket1 = match result.unwrap() {
        RuntimeSignal::AwaitingAction(t) => t,
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
        RuntimeSignal::AwaitingAction(t) => t,
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
        RuntimeSignal::AwaitingAction(t) => t,
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
    let mut registry = ActionRegistry::new();
    let network_cap = Capability::new("Network".into(), ActionId::new(3));
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
        required_capabilities: Box::new([network_cap.clone()]),
    };
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(3);
    let input_taint = Taint::Clean;
    let frame = make_test_frame(run_id, input_taint);
    let disk_cap = Capability::new("Disk".into(), ActionId::new(3));
    let granted = CapabilitySet::from_grants(Box::new([disk_cap]));
    let contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let registry_contracts = build_indexed_registry(&contracts);
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_err(), "execute_do should fail when capability not granted");
}

#[test]
fn test_execute_do_capability_check_passes_with_matching_grant() {
    use vb_core::capability::Capability;
    let mut registry = ActionRegistry::new();
    let network_cap = Capability::new("Network".into(), ActionId::new(4));
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
        required_capabilities: Box::new([network_cap.clone()]),
    };
    registry.register(contract.clone()).expect("register should succeed");
    let run_id = RunId::new(1);
    let step = StepIdx::new(0);
    let seq = SeqNo::new(1);
    let action = ActionId::new(4);
    let input_taint = Taint::Clean;
    let frame = make_test_frame(run_id, input_taint);
    let network_grant = Capability::new("Network".into(), ActionId::new(4));
    let disk_grant = Capability::new("Disk".into(), ActionId::new(4));
    let granted = CapabilitySet::from_grants(Box::new([network_grant, disk_grant]));
    let contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let registry_contracts = build_indexed_registry(&contracts);
    let result = execute_do_test(&frame, step, action, SlotIdx::new(0), seq, &registry_contracts, &granted);
    assert!(result.is_ok(), "execute_do should succeed when capability is granted");
}

#[test]
fn test_taint_propagates_through_multiple_at_least_once_actions() {
    let mut registry = ActionRegistry::new();
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
    let granted = CapabilitySet::empty();
    let contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let registry_contracts = build_indexed_registry(&contracts);
    let frame1 = make_test_frame(run_id, Taint::Clean);
    let result1 = execute_do_test(&frame1, StepIdx::new(0), ActionId::new(10), SlotIdx::new(0), SeqNo::new(1), &registry_contracts, &granted);
    assert!(result1.is_ok(), "pure action with clean input should succeed");
    let frame2 = make_test_frame(run_id, Taint::Secret);
    let result2 = execute_do_test(&frame2, StepIdx::new(1), ActionId::new(11), SlotIdx::new(0), SeqNo::new(2), &registry_contracts, &granted);
    assert!(result2.is_ok(), "at_least_once action should handle secret input");
    let frame3 = make_test_frame(run_id, Taint::DerivedFromSecret);
    let result3 = execute_do_test(&frame3, StepIdx::new(2), ActionId::new(10), SlotIdx::new(0), SeqNo::new(3), &registry_contracts, &granted);
    assert!(result3.is_err(), "pure action with derived secret input should fail");
}

#[test]
fn test_secret_input_blocks_pure_action() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(20), Idempotency::DeterministicPure, RetrySafety::Safe, 1);
    registry.register(contract).expect("register should succeed");
    let run_id = RunId::new(1);
    let granted = CapabilitySet::empty();
    let contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let registry_contracts = build_indexed_registry(&contracts);
    let frame = make_test_frame(run_id, Taint::Secret);
    let result = execute_do_test(&frame, StepIdx::new(0), ActionId::new(20), SlotIdx::new(0), SeqNo::new(1), &registry_contracts, &granted);
    assert!(result.is_err(), "pure action with secret input should fail");
}

#[test]
fn test_clean_input_passes_through_pure_action() {
    let mut registry = ActionRegistry::new();
    let contract = make_contract(ActionId::new(21), Idempotency::DeterministicPure, RetrySafety::Safe, 1);
    registry.register(contract).expect("register should succeed");
    let run_id = RunId::new(1);
    let granted = CapabilitySet::empty();
    let contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let registry_contracts = build_indexed_registry(&contracts);
    let frame = make_test_frame(run_id, Taint::Clean);
    let result = execute_do_test(&frame, StepIdx::new(0), ActionId::new(21), SlotIdx::new(0), SeqNo::new(1), &registry_contracts, &granted);
    assert!(result.is_ok(), "pure action with clean input should succeed");
    let signal = result.unwrap();
    match signal {
        RuntimeSignal::AwaitingAction(_) => {}
        _ => panic!("expected AwaitingAction"),
    }
}
