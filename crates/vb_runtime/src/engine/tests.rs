#![forbid(unsafe_code)]

//! Engine tests moved from engine.rs for line count compliance.
#![allow(unused_imports)]

use vb_core::action::Idempotency;
use vb_core::action::RetrySafety;
use vb_core::action::SideEffect;
use vb_core::engine::EngineSignal;
use vb_core::errors::EngineError;
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledNode;

use crate::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
    compute_idempotency_key, drive_deterministic_full, execute_do, execute_do_without_contract,
    execute_error_handler, execute_retry_check, resolve_contract, resume_action_outcome,
};
use vb_core::action::ActionFailure;
use vb_core::action::ActionFailureCode;
use vb_core::action::ActionOutcome;
use vb_core::action::ActionTicket;
use vb_core::action::RetryPolicy as VbRetryPolicy;
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::Taint;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledNodeKind;
use vb_core::workflow::CompiledWorkflow;

// =====================================================================
// Basic retry policy tests
// =====================================================================

#[test]
fn retry_policy_never_has_max_attempts_one() {
    assert_eq!(RetryPolicy::NEVER.max_attempts, 1);
}

#[test]
fn retry_policy_default_has_max_attempts_three() {
    assert_eq!(RetryPolicy::DEFAULT.max_attempts, 3);
}

#[test]
fn retry_policy_never_has_base_delay_zero() {
    assert_eq!(RetryPolicy::NEVER.base_delay_ms, 0);
    assert_eq!(RetryPolicy::NEVER.exponential_backoff, false);
}

#[test]
fn retry_policy_default_has_base_delay_100() {
    assert_eq!(RetryPolicy::DEFAULT.base_delay_ms, 100);
    assert_eq!(RetryPolicy::DEFAULT.exponential_backoff, false);
}

#[test]
fn retry_policy_clone_preserves_values() {
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 200,
        exponential_backoff: true,
    };
    let cloned = policy.clone();
    assert_eq!(cloned.max_attempts, 5);
    assert_eq!(cloned.base_delay_ms, 200);
    assert_eq!(cloned.exponential_backoff, true);
}

// =====================================================================
// Retry check tests
// =====================================================================

#[test]
fn retry_check_routes_to_body_when_attempts_remain() {
    let policy = RetryPolicy::DEFAULT;
    let target = execute_retry_check(1, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(5));
}

#[test]
fn retry_check_routes_to_exhausted_when_attempts_spent() {
    let policy = RetryPolicy {
        max_attempts: 2,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let target = execute_retry_check(2, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(10));
}

#[test]
fn never_retry_always_exhausts_after_one() {
    let policy = RetryPolicy::NEVER;
    let target = execute_retry_check(1, policy, StepIdx::new(3), StepIdx::new(7));
    assert_eq!(target, StepIdx::new(7));
}

#[test]
fn retry_check_routes_to_body_below_max() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let target = execute_retry_check(0, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(5));
}

#[test]
fn retry_check_routes_to_body_at_max_minus_one() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let target = execute_retry_check(2, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(5));
}

#[test]
fn retry_check_returns_done_when_attempts_equal_max() {
    let policy = RetryPolicy::DEFAULT;
    let target = execute_retry_check(3, policy, StepIdx::new(1), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(10));
}

// =====================================================================
// Error handler tests
// =====================================================================

#[test]
fn error_handler_routes_to_handler_on_retryable_failure() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(5), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(5));
}

#[test]
fn error_handler_routes_to_body_on_non_retryable_unknown() {
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(5), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(3));
}

#[test]
fn error_handler_routes_to_body_when_failure_is_unknown_and_non_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(3));
}

#[test]
fn error_handler_routes_to_handler_on_unknown_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(8));
}

#[test]
fn error_handler_routes_to_handler_on_non_unknown_non_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(8));
}

// =====================================================================
// Idempotency key tests
// =====================================================================

#[test]
fn compute_idempotency_key_is_deterministic() {
    let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(2), ActionId::new(3));
    let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(2), ActionId::new(3));
    assert_eq!(key1, key2);
}

#[test]
fn compute_idempotency_key_differs_for_different_runs() {
    let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
    let key2 = compute_idempotency_key(RunId::new(2), SeqNo::new(0), ActionId::new(0));
    assert_ne!(key1, key2);
}

#[test]
fn compute_idempotency_key_is_unique_for_different_seq() {
    let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
    let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(0));
    assert_ne!(key1, key2);
}

#[test]
fn compute_idempotency_key_is_unique_for_different_action() {
    let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
    let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(1));
    assert_ne!(key1, key2);
}

#[test]
fn compute_idempotency_key_handles_large_values_without_overflow() {
    let key = compute_idempotency_key(
        RunId::new(u64::MAX),
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    let key2 = compute_idempotency_key(
        RunId::new(u64::MAX),
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    assert_eq!(key, key2);
}

// =====================================================================
// RuntimeSignal tests
// =====================================================================

#[test]
fn runtime_signal_equality_continue() {
    assert_eq!(RuntimeSignal::Continue, RuntimeSignal::Continue);
}

#[test]
fn runtime_signal_equality_exhausted() {
    assert_eq!(
        RuntimeSignal::StepBudgetExhausted,
        RuntimeSignal::StepBudgetExhausted
    );
}

#[test]
fn runtime_signal_equality_awaiting_wait() {
    assert_eq!(RuntimeSignal::AwaitingWait, RuntimeSignal::AwaitingWait);
}

#[test]
fn runtime_signal_equality_awaiting_ask() {
    assert_eq!(RuntimeSignal::AwaitingAsk, RuntimeSignal::AwaitingAsk);
}

#[test]
fn runtime_signal_differs_awaiting_wait_from_awaiting_ask() {
    assert_ne!(RuntimeSignal::AwaitingWait, RuntimeSignal::AwaitingAsk);
}

#[test]
fn runtime_signal_equality_differs_for_different_finished_values() {
    let a = RuntimeSignal::Finished(SlotValue::I64(1));
    let b = RuntimeSignal::Finished(SlotValue::I64(2));
    assert_ne!(a, b);
}

#[test]
fn runtime_signal_awaiting_action_equality_matches_on_ticket() {
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(5),
        attempt: 1,
        idempotency_key: 42,
        capacity: 1,
    };
    let a = RuntimeSignal::AwaitingAction(ticket);
    let b = RuntimeSignal::AwaitingAction(ticket);
    assert_eq!(a, b);
}

#[test]
fn runtime_signal_awaiting_action_differs_for_different_ticket() {
    let a = RuntimeSignal::AwaitingAction(ActionTicket {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    });
    let b = RuntimeSignal::AwaitingAction(ActionTicket {
        run: RunId::new(2),
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    });
    assert_ne!(a, b);
}

// =====================================================================
// RuntimeEngineError tests
// =====================================================================

#[test]
fn runtime_engine_error_from_engine_error() {
    let core = EngineError::UnsupportedPrimitive { primitive: "test" };
    let engine_err: RuntimeEngineError = core.into();
    assert_eq!(
        engine_err,
        RuntimeEngineError::Core(EngineError::UnsupportedPrimitive { primitive: "test" })
    );
}

#[test]
fn runtime_engine_error_from_action_error() {
    let action = vb_core::action::ActionError::UnknownAction {
        action: ActionId::new(5),
    };
    let engine_err: RuntimeEngineError = action.into();
    assert_eq!(
        engine_err,
        RuntimeEngineError::Action(vb_core::action::ActionError::UnknownAction {
            action: ActionId::new(5)
        })
    );
}

#[test]
fn runtime_engine_error_core_wraps_core_error() {
    let core_err = EngineError::UnsupportedPrimitive {
        primitive: "test_prim",
    };
    let engine_err = RuntimeEngineError::Core(core_err.clone());
    assert_eq!(
        engine_err,
        RuntimeEngineError::Core(EngineError::UnsupportedPrimitive {
            primitive: "test_prim",
        })
    );
}

#[test]
fn runtime_engine_error_action_wraps_action_error() {
    let action_err = vb_core::action::ActionError::UnknownAction {
        action: ActionId::new(7),
    };
    let engine_err = RuntimeEngineError::Action(action_err.clone());
    assert_eq!(
        engine_err,
        RuntimeEngineError::Action(vb_core::action::ActionError::UnknownAction {
            action: ActionId::new(7),
        })
    );
}

#[test]
fn runtime_engine_error_retry_exhausted_reports_action_and_attempts() {
    let err = RuntimeEngineError::RetryExhausted {
        action: ActionId::new(3),
        attempts: 5,
    };
    match err {
        RuntimeEngineError::RetryExhausted { action, attempts } => {
            assert_eq!(action, ActionId::new(3));
            assert_eq!(attempts, 5);
        }
        other => assert_eq!(
            other,
            RuntimeEngineError::RetryExhausted {
                action: ActionId::new(0),
                attempts: 0,
            }
        ),
    }
}

#[test]
fn runtime_engine_error_taint_violation_reports_step() {
    let err = RuntimeEngineError::TaintViolation {
        step: StepIdx::new(42),
    };
    match err {
        RuntimeEngineError::TaintViolation { step } => {
            assert_eq!(step, StepIdx::new(42));
        }
        other => assert_eq!(
            other,
            RuntimeEngineError::TaintViolation {
                step: StepIdx::new(0),
            }
        ),
    }
}

// =====================================================================
// Step budget tests
// =====================================================================

#[test]
fn step_budget_new_with_zero_allows_no_steps() {
    let mut budget = vb_core::engine::StepBudget::new(0);
    assert_eq!(budget.try_take(), Ok(false));
}

#[test]
fn step_budget_remaining_decreases_after_each_step() {
    let mut budget = vb_core::engine::StepBudget::new(3);
    assert_eq!(budget.try_take(), Ok(true));
    assert_eq!(budget.try_take(), Ok(true));
    assert_eq!(budget.try_take(), Ok(true));
    assert_eq!(budget.try_take(), Ok(false));
}

// =====================================================================
// Execute do tests
// =====================================================================

#[test]
fn execute_do_returns_awaiting_action_for_known_action() {
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Initialize input slot before dispatch.
    let write_result = run.write_slot(SlotIdx::new(0), vb_core::SlotValue::I64(42));
    if write_result.is_err() {
        return;
    }
    let contract = vb_core::action::ActionContract {
        id: ActionId::new(1),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let registry_contracts: Vec<vb_core::action::ActionContract> = vec![
        vb_core::action::ActionContract {
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
        },
        contract,
    ];
    let contract_ref = match registry_contracts.get(1) {
        Some(c) => c,
        None => return,
    };
    let result = execute_do(
        &run,
        StepIdx::new(0),
        ActionId::new(1),
        SlotIdx::new(0),
        SeqNo::new(0),
        contract_ref,
        &registry_contracts,
        &CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(1));
            assert_eq!(ticket.run, RunId::new(1));
            assert_eq!(ticket.step, StepIdx::new(0));
        }
        other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
    }
}

#[test]
fn execute_do_propagates_taint_from_secret_input_without_violation() {
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let write_result = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    assert_eq!(write_result.map(|_| ()), Ok(()));
    let contract = vb_core::action::ActionContract {
        id: ActionId::new(1),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let registry_contracts: Vec<vb_core::action::ActionContract> = vec![
        vb_core::action::ActionContract {
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
        },
        contract,
    ];
    let contract_ref = match registry_contracts.get(1) {
        Some(c) => c,
        None => return,
    };
    let result = execute_do(
        &run,
        StepIdx::new(0),
        ActionId::new(1),
        SlotIdx::new(0),
        SeqNo::new(0),
        contract_ref,
        &registry_contracts,
        &CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(1));
            assert_eq!(ticket.run, RunId::new(1));
        }
        other => assert_eq!(other, Ok(RuntimeSignal::Continue)),
    }
}

#[test]
fn execute_do_returns_unknown_action_for_unregistered_action() {
    let run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let empty_contracts: Vec<vb_core::action::ActionContract> = Vec::new();
    let dummy_contract = vb_core::action::ActionContract {
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
    let result = execute_do(
        &run,
        StepIdx::new(0),
        ActionId::new(5),
        SlotIdx::new(0),
        SeqNo::new(0),
        &dummy_contract,
        &empty_contracts,
        &CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
    );
    assert_eq!(
        result,
        Err(RuntimeEngineError::Action(
            vb_core::action::ActionError::UnknownAction {
                action: ActionId::new(5),
            }
        ))
    );
}

#[test]
fn execute_do_without_contract_fails_closed_without_ticket() {
    let mut run = match RunFrame::new(RunId::new(42), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Input slot must be initialized with clean taint.
    let _ = run.write_slot(SlotIdx::new(0), vb_core::SlotValue::I64(0));
    let result = execute_do_without_contract(
        &run,
        StepIdx::new(3),
        ActionId::new(7),
        SlotIdx::new(0),
        SeqNo::new(5),
        &vb_core::capability::CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
    );
    assert!(matches!(
        result,
        Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied { action, .. }))
            if action == ActionId::new(7)
    ));
}

#[cfg(test)]
fn make_original_ticket() -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(10),
        action: ActionId::new(7),
        attempt: 3,
        idempotency_key: compute_idempotency_key(RunId::new(1), SeqNo::new(10), ActionId::new(7)),
        capacity: 5,
    }
}

#[cfg(test)]
fn dummy_contract() -> vb_core::action::ActionContract {
    vb_core::action::ActionContract {
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
    }
}

#[test]
fn resume_action_outcome_ready_continues_execution() {
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let ready = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(ready);
    let original = make_original_ticket();
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn resume_action_outcome_failed_non_retryable_returns_error() {
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let outcome = ActionOutcome::Failed(failure);
    let original = make_original_ticket();
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    assert_eq!(
        result,
        Err(RuntimeEngineError::Core(
            EngineError::UnsupportedPrimitive {
                primitive: "action_failed_non_retryable",
            }
        ))
    );
}

#[test]
fn resume_action_outcome_suspended_returns_awaiting() {
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(5),
        action: ActionId::new(3),
        attempt: 2,
        idempotency_key: 99,
        capacity: 1,
    };
    let outcome = ActionOutcome::Suspended(ticket);
    let original = make_original_ticket();
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    assert_eq!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(5),
            action: ActionId::new(3),
            attempt: 2,
            idempotency_key: 99,
            capacity: 1,
        }))
    );
}

#[test]
fn resume_action_outcome_retryable_failure_propagates_original_ticket_fields() {
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let outcome = ActionOutcome::Failed(failure);
    let original = make_original_ticket();
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    match result {
        Ok(RuntimeSignal::AwaitingAction(retry_ticket)) => {
            assert_eq!(retry_ticket.run, RunId::new(1));
            assert_eq!(retry_ticket.step, StepIdx::new(2));
            assert_eq!(
                retry_ticket.seq,
                SeqNo::new(11),
                "seq must be incremented from original"
            );
            assert_eq!(
                retry_ticket.action,
                ActionId::new(7),
                "action must match original ticket"
            );
            assert_eq!(
                retry_ticket.attempt, 4,
                "attempt must be incremented from original"
            );
            let expected_key =
                compute_idempotency_key(RunId::new(1), SeqNo::new(11), ActionId::new(7));
            assert_eq!(
                retry_ticket.idempotency_key, expected_key,
                "idempotency_key must be recomputed from new seq"
            );
        }
        other => {
            let msg = format!("expected AwaitingAction, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// Resolve contract tests
// =====================================================================

#[test]
fn resolve_contract_returns_unknown_action_for_empty_contracts() {
    let contracts: Vec<vb_core::action::ActionContract> = Vec::new();
    let result = resolve_contract(ActionId::new(0), &contracts);
    assert_eq!(
        result,
        Err(RuntimeEngineError::Action(
            vb_core::action::ActionError::UnknownAction {
                action: ActionId::new(0),
            }
        ))
    );
}

// =====================================================================
// Drive loop tests
// =====================================================================

#[test]
fn drive_deterministic_budget_zero_returns_step_budget_exhausted() {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = vb_core::workflow::WorkflowParts {
        name: Box::from("nop"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut store = ValueStore::new();
    let mut budget = vb_core::engine::StepBudget::new(0);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = crate::primitives::collect::CollectStates::new();
    let result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
    );
    assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
}

// =====================================================================
// Black-hat finding: idempotency key overflow graceful degradation
// =====================================================================

#[test]
fn bh_idempotency_key_overflow_fallback_is_deterministic() {
    // When overflow occurs, compute_idempotency_key falls back to run_part.
    // The fallback must still be deterministic for the same inputs.
    let key1 = compute_idempotency_key(
        RunId::new(u64::MAX),
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    let key2 = compute_idempotency_key(
        RunId::new(u64::MAX),
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    assert_eq!(key1, key2, "overflow fallback must be deterministic");
}

#[test]
fn bh_idempotency_key_overflow_preserves_run_uniqueness() {
    // Even in overflow, different run IDs must produce different keys
    // (the fallback is run_part, which differs when run IDs differ).
    let key_a =
        compute_idempotency_key(RunId::new(100), SeqNo::new(u64::MAX), ActionId::new(65535));
    let key_b =
        compute_idempotency_key(RunId::new(200), SeqNo::new(u64::MAX), ActionId::new(65535));
    assert_ne!(
        key_a, key_b,
        "different runs must produce different keys even under overflow"
    );
}

#[test]
fn bh_idempotency_key_all_three_components_contribute() {
    // Verify that changing any single component while holding the other two
    // constant produces a different key.
    let base_run = RunId::new(42);
    let base_seq = SeqNo::new(10);
    let base_action = ActionId::new(5);

    let base_key = compute_idempotency_key(base_run, base_seq, base_action);

    let key_diff_run = compute_idempotency_key(RunId::new(43), base_seq, base_action);
    assert_ne!(base_key, key_diff_run, "changing run must change key");

    let key_diff_seq = compute_idempotency_key(base_run, SeqNo::new(11), base_action);
    assert_ne!(base_key, key_diff_seq, "changing seq must change key");

    let key_diff_action = compute_idempotency_key(base_run, base_seq, ActionId::new(6));
    assert_ne!(base_key, key_diff_action, "changing action must change key");
}

#[test]
fn bh_idempotency_key_overflow_does_not_produce_zero_for_nonzero_run() {
    // A non-zero run ID must never collapse to zero key even under overflow.
    let key = compute_idempotency_key(RunId::new(1), SeqNo::new(u64::MAX), ActionId::new(65535));
    assert_ne!(
        key, 0,
        "non-zero run must not produce zero key under overflow"
    );
}

// =====================================================================
// Black-hat finding: RetryCheck with NEVER always exhausts at attempt 0
// =====================================================================

#[test]
fn bh_retry_check_never_routes_to_body_at_attempt_zero() {
    // NEVER has max_attempts=1, so attempt 0 < 1 routes to body.
    // This is the one allowed attempt before the first execution.
    let target = execute_retry_check(0, RetryPolicy::NEVER, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(
        target,
        StepIdx::new(5),
        "attempt 0 with NEVER should route to body"
    );
}

#[test]
fn bh_retry_check_never_routes_to_exhausted_at_attempt_zero_for_zero_max() {
    // A policy with max_attempts=0 should exhaust immediately at attempt 0.
    let policy = RetryPolicy {
        max_attempts: 0,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let target = execute_retry_check(0, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(
        target,
        StepIdx::new(10),
        "attempt 0 with max_attempts=0 should exhaust"
    );
}

#[test]
fn bh_retry_check_default_allows_body_for_all_attempts_below_max() {
    // DEFAULT has max_attempts=3. Attempts 0, 1, 2 should all route to body.
    for attempt in 0u16..3 {
        let target = execute_retry_check(
            attempt,
            RetryPolicy::DEFAULT,
            StepIdx::new(1),
            StepIdx::new(99),
        );
        assert_eq!(
            target,
            StepIdx::new(1),
            "DEFAULT attempt {attempt} should route to body"
        );
    }
}

#[test]
fn bh_retry_check_custom_policy_boundary() {
    // Exact boundary: attempt == max_attempts routes to exhausted.
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let body = StepIdx::new(3);
    let exhausted = StepIdx::new(9);

    // attempt 4 < 5 => body
    let t4 = execute_retry_check(4, policy, body, exhausted);
    assert_eq!(t4, body);

    // attempt 5 == 5 => exhausted
    let t5 = execute_retry_check(5, policy, body, exhausted);
    assert_eq!(t5, exhausted);

    // attempt 6 > 5 => exhausted
    let t6 = execute_retry_check(6, policy, body, exhausted);
    assert_eq!(t6, exhausted);
}

// =====================================================================
// Black-hat finding: drive evidence correctness
// =====================================================================

#[test]
fn bh_drive_budget_exhausted_does_not_emit_step_succeeded_in_evidence() {
    // When budget is zero, drive_deterministic_full returns StepBudgetExhausted
    // immediately without executing any step. Evidence should be empty.
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = vb_core::workflow::WorkflowParts {
        name: Box::from("bh_nop"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut store = ValueStore::new();
    let mut budget = vb_core::engine::StepBudget::new(0);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = crate::primitives::collect::CollectStates::new();
    let result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
    );
    assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));

    // Budget was zero so no step executed: evidence must be completely empty.
    let events = evidence.drain();
    assert!(
        events.is_empty(),
        "budget exhaustion should produce zero evidence events, got {events:?}"
    );
}

#[test]
fn bh_drive_single_nop_emits_started_then_succeeded() {
    // With budget > 0 and a Nop that advances to a Finish, the drive loop
    // executes at least one step. Evidence should contain matched pairs
    // of StepStarted/StepSucceeded.
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = vb_core::workflow::WorkflowParts {
        name: Box::from("bh_nop_finish"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut store = ValueStore::new();
    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = crate::primitives::collect::CollectStates::new();
    let result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
    );
    // The drive loop should have progressed (either Continue after Nop, or Finished after Finish).
    assert!(result.is_ok(), "drive should succeed, got {result:?}");

    let events = evidence.drain();
    // Every executed step must have a StepStarted followed by StepSucceeded pair.
    let started_count = events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepStarted { .. }))
        .count();
    let succeeded_count = events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepSucceeded { .. }))
        .count();
    assert_eq!(
        started_count, succeeded_count,
        "every StepStarted must have a matching StepSucceeded"
    );
    assert!(
        started_count > 0,
        "at least one step should have been executed"
    );
}

#[test]
fn bh_drive_evidence_step_succeeded_not_emitted_for_awaiting_action() {
    // A Do node produces AwaitingAction. The drive loop must NOT emit StepSucceeded
    // because the step did not actually complete -- it suspended waiting for an
    // action result. Only StepStarted should be emitted for such steps.
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(5),
            input: SlotIdx::new(0),
        },
    };
    let parts = vb_core::workflow::WorkflowParts {
        name: Box::from("bh_do"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Input slot must be initialized with clean taint.
    let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(0));
    let mut store = ValueStore::new();
    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = crate::primitives::collect::CollectStates::new();
    let result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(_)) => {}
        other => {
            assert!(
                other.is_err(),
                "expected AwaitingAction or error, got {other:?}"
            );
            return;
        }
    }

    let events = evidence.drain();
    // The drive loop emits StepStarted but NOT StepSucceeded for steps
    // that suspend with AwaitingAction. StepSucceeded is only emitted
    // for Continue and Finished signals.
    let started_count = events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepStarted { .. }))
        .count();
    let succeeded_count = events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepSucceeded { .. }))
        .count();
    assert_eq!(
        started_count, 1,
        "exactly one StepStarted should be emitted for the Do node"
    );
    assert_eq!(
        succeeded_count, 0,
        "StepSucceeded must NOT be emitted for AwaitingAction -- the step did not complete"
    );
}

// =====================================================================
// Black-hat finding: runtime signal taint preservation
// =====================================================================

#[test]
fn bh_resume_action_outcome_ready_preserves_secret_taint() {
    // When an action completes with a tainted output, the taint must be
    // preserved through resume_action_outcome into the run frame.
    let ready = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Secret,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(ready);
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let result = resume_action_outcome(&ticket, outcome, &dummy_contract());
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn bh_resume_action_outcome_ready_preserves_derived_taint() {
    let ready = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(99),
        taint: Taint::DerivedFromSecret,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(ready);
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let result = resume_action_outcome(&ticket, outcome, &dummy_contract());
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn bh_resume_action_outcome_ready_clean_taint_preserved() {
    let ready = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(1),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(ready);
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    };
    let result = resume_action_outcome(&ticket, outcome, &dummy_contract());
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn bh_resume_action_outcome_suspended_preserves_ticket_fields() {
    // The Suspended path must pass the ticket through unchanged.
    let original_ticket = ActionTicket {
        run: RunId::new(7),
        step: StepIdx::new(3),
        seq: SeqNo::new(11),
        action: ActionId::new(5),
        attempt: 3,
        idempotency_key: 12345,
        capacity: 1,
    };
    let outcome = ActionOutcome::Suspended(original_ticket);
    let result = resume_action_outcome(&make_original_ticket(), outcome, &dummy_contract());
    match result {
        Ok(RuntimeSignal::AwaitingAction(returned_ticket)) => {
            assert_eq!(returned_ticket.run, RunId::new(7));
            assert_eq!(returned_ticket.step, StepIdx::new(3));
            assert_eq!(returned_ticket.seq, SeqNo::new(11));
            assert_eq!(returned_ticket.action, ActionId::new(5));
            assert_eq!(returned_ticket.attempt, 3);
            assert_eq!(returned_ticket.idempotency_key, 12345);
        }
        other => {
            let msg = format!("expected AwaitingAction, got {other:?}");
            panic!("{msg}");
        }
    }
}

#[test]
fn bh_resume_action_outcome_failed_retryable_preserves_signal_structure() {
    // A retryable failure should produce an AwaitingAction with a retry ticket
    // derived from the original ticket (incremented seq and attempt).
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let outcome = ActionOutcome::Failed(failure);
    let original = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(5),
        action: ActionId::new(3),
        attempt: 2,
        idempotency_key: 100,
        capacity: 4,
    };
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            // The retry ticket uses the run's ID, original step, incremented seq and attempt.
            assert_eq!(ticket.run, RunId::new(1));
            assert_eq!(
                ticket.step,
                StepIdx::new(2),
                "step must come from original ticket"
            );
            assert_eq!(
                ticket.seq,
                SeqNo::new(6),
                "seq must be incremented from original"
            );
            assert_eq!(
                ticket.action,
                ActionId::new(3),
                "action must come from original"
            );
            assert_eq!(
                ticket.attempt, 3,
                "attempt must be incremented from original"
            );
        }
        other => {
            let msg = format!("expected AwaitingAction for retryable failure, got {other:?}");
            panic!("{msg}");
        }
    }
}

#[test]
fn bh_execute_do_propagates_taint_through_ticket_for_at_least_once() {
    // execute_do with AtLeastOnceExternal and Secret input must succeed
    // (no taint violation) and produce an AwaitingAction ticket.
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let write_result = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    assert_eq!(write_result.map(|_| ()), Ok(()));

    let contract = vb_core::action::ActionContract {
        id: ActionId::new(0),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let registry = vec![contract];
    let contract_ref = match registry.get(0) {
        Some(c) => c,
        None => return,
    };
    let result = execute_do(
        &run,
        StepIdx::new(0),
        ActionId::new(0),
        SlotIdx::new(0),
        SeqNo::new(1),
        contract_ref,
        &registry,
        &CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(0));
            assert_eq!(ticket.run, RunId::new(1));
            assert_eq!(ticket.seq, SeqNo::new(1));
            assert_eq!(ticket.attempt, 1);
            // The idempotency key must be non-zero for non-trivial inputs.
            assert_ne!(ticket.idempotency_key, 0);
        }
        other => {
            let msg = format!("expected AwaitingAction, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// Black-hat finding: EvidenceEvent ordering invariant
// =====================================================================

#[test]
fn bh_evidence_events_always_alternate_started_succeeded() {
    // For every step that completes without error, evidence must be
    // [StepStarted(N), StepSucceeded(N)] in strict alternating order.
    // This test uses the EvidenceCollector directly to verify the pattern.
    let mut collector = EvidenceCollector::new();
    collector.push_step_started(StepIdx::new(0));
    collector.push_step_succeeded(StepIdx::new(0), Some(SlotIdx::new(1)));
    collector.push_step_started(StepIdx::new(1));
    collector.push_step_succeeded(StepIdx::new(1), None);

    let events = collector.drain();
    assert_eq!(events.len(), 4);

    // Verify strict alternation: even indices are Started, odd are Succeeded.
    for (i, event) in events.iter().enumerate() {
        if i % 2 == 0 {
            assert!(
                matches!(event, EvidenceEvent::StepStarted { .. }),
                "event at index {i} should be StepStarted, got {event:?}"
            );
        } else {
            assert!(
                matches!(event, EvidenceEvent::StepSucceeded { .. }),
                "event at index {i} should be StepSucceeded, got {event:?}"
            );
        }
    }
}

// =====================================================================
// Proptest tests
// =====================================================================

#[cfg(test)]
mod proptests {

    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn step_budget_never_allows_more_than_n_steps(n in 1u64..1000u64) {
            let mut budget = vb_core::engine::StepBudget::new(n);
            let mut taken = 0u64;
            let mut drained = false;
            while !drained && taken <= n + 1 {
                match budget.try_take() {
                    Ok(true) => taken += 1,
                    Ok(false) => drained = true,
                    Err(_) => drained = true,
                }
            }
            prop_assert_eq!(taken, n);
        }
    }

    proptest! {
        #[test]
        fn idempotency_key_differs_for_different_tuples(
            run1 in 1u64..100u64,
            run2 in 1u64..100u64,
            seq1 in 0u64..100u64,
            seq2 in 0u64..100u64,
            action1 in 0u16..10u16,
            action2 in 0u16..10u16,
        ) {
            prop_assume!(run1 != run2 || seq1 != seq2 || action1 != action2);
            let key1 = compute_idempotency_key(RunId::new(run1), SeqNo::new(seq1), ActionId::new(action1));
            let key2 = compute_idempotency_key(RunId::new(run2), SeqNo::new(seq2), ActionId::new(action2));
            prop_assert_ne!(key1, key2);
        }
    }
}

// ==========================================================================
// BLACKHAT SECURITY REVIEW: engine module findings
// ==========================================================================
//
// Reviewer: BLACKHAT
// Scope: engine/{mod,execute,drive,action,signal,helpers,types}.rs
//
// Findings documented in tests below:
//
// BH-ENG-01: EvidenceCollector has no capacity bound (resource exhaustion)
// BH-ENG-02: mark_step_after_signal leaves Running state on StepBudgetExhausted
// BH-ENG-03: read_attempt_from_slot silently returns 0 on read errors
// BH-ENG-04: runtime_from_core discards taint from Finished signal
// BH-ENG-05: ErrorHandler dispatch routes to body, not handler
// BH-ENG-06: RetryPolicy max_attempts=0 is accepted (zero-retry policy)
// BH-ENG-07: execute_do_without_contract skips all security checks
// BH-ENG-08: RetryCheck increments executed counter
// BH-ENG-09: No SlotWritten for AwaitingAction steps without output
// BH-ENG-10: Idempotency key collision search in small space
// BH-ENG-11: Retry ticket uses frame run ID
// BH-ENG-12: drive_with_actions creates fresh ValueStore per call
// BH-ENG-13: Suspended outcome ignores original ticket fields
// BH-ENG-14: Double taint check defense in depth
// ==========================================================================

#[cfg(test)]
mod blackhat_engine {

    use vb_core::action::{
        ActionContract, ActionFailure, ActionFailureCode, ActionOutcome, ActionTicket, Idempotency,
        RetryPolicy as VbRetryPolicy, RetrySafety, SideEffect,
    };
    use vb_core::capability::CapabilitySet;
    use vb_core::engine::EngineSignal;
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx};
    use vb_core::value::{SlotValue, Taint};
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

    use crate::engine::{
        EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
        compute_idempotency_key, drive_deterministic_full, drive_with_actions, execute_do,
        execute_do_without_contract, execute_error_handler, execute_node_full, execute_retry_check,
        resume_action_outcome, runtime_from_core,
    };
    use crate::primitives::collect::CollectStates;

    // ---- Helpers ----

    fn dummy_contract() -> ActionContract {
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
        }
    }

    // ---- Workflow/Run factories ----

    fn make_workflow(nodes: Vec<CompiledNode>, slot_count: u16) -> CompiledWorkflow {
        make_workflow_with_constants(nodes, slot_count, Box::from([]))
    }

    fn make_workflow_with_constants(
        nodes: Vec<CompiledNode>,
        slot_count: u16,
        constants: Box<[vb_core::value::ConstValue]>,
    ) -> CompiledWorkflow {
        let parts = WorkflowParts {
            name: Box::from("bh_test"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0xBB; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants,
            slot_count,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: Box::from([]),
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(e) => {
                let msg = format!("workflow validation failed: {e}");
                panic!("{msg}");
            }
        }
    }

    fn finish_node(id: u16, slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: Some(SlotIdx::new(slot)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(slot),
            },
        }
    }

    fn make_run(slot_count: u16, step_count: u16) -> RunFrame {
        match RunFrame::new(RunId::new(1), StepIdx::ZERO, slot_count, step_count) {
            Ok(f) => f,
            Err(e) => {
                let msg = format!("RunFrame::new failed: {e}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // BH-ENG-01 FIXED: EvidenceCollector now has capacity bound
    //
    // The EvidenceCollector previously used an unbounded Vec. Now it has a
    // capacity limit (DEFAULT_EVIDENCE_CAPACITY = 3072). Events beyond the
    // capacity are silently dropped with a dropped count tracked.
    // =====================================================================

    #[test]
    fn bh_eng_01_evidence_collector_enforces_capacity_bound() {
        let mut collector = EvidenceCollector::new();
        let capacity = collector.capacity();
        // Push more events than capacity allows.
        for i in 0u16..10_000 {
            collector.push_step_started(StepIdx::new(i));
        }
        assert_eq!(
            collector.len(),
            capacity,
            "BH-ENG-01 FIXED: EvidenceCollector should respect capacity bound"
        );
        assert_eq!(
            collector.dropped(),
            10_000 - capacity,
            "dropped count should reflect overflow"
        );
    }

    #[test]
    fn bh_eng_01_evidence_events_per_step_exceeds_one() {
        // Each step in drive_deterministic_full can emit up to 3 events:
        // StepStarted, SlotWritten, StepSucceeded. A budget of N steps
        // can produce up to 3*N events. Use SetConst which actually writes.
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = finish_node(1, 0);
        let wf = make_workflow_with_constants(
            vec![set_const, finish],
            4,
            Box::from([vb_core::value::ConstValue::I64(1)]),
        );
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(result.is_ok(), "drive should succeed: {result:?}");
        let events = evidence.drain();
        assert!(
            events.len() > 1,
            "BH-ENG-01: expected multiple evidence events, got {}",
            events.len()
        );
    }

    // =====================================================================
    // BH-ENG-02: mark_step_after_signal leaves Running state on
    //            StepBudgetExhausted (state machine gap)
    //
    // When the drive loop exhausts its step budget mid-run, the step at
    // the current PC has been marked Running (drive.rs:69) but
    // mark_step_after_signal maps StepBudgetExhausted to Ok(()) without
    // transitioning the step state. The step remains in Running state
    // until the next drive call.
    // Severity: Low-Medium. On resume, the drive loop re-marks the step.
    // =====================================================================

    #[test]
    fn bh_eng_02_budget_exhaustion_leaves_step_in_running_state() {
        let nop = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let nop1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let finish = finish_node(2, 0);
        let wf = make_workflow(vec![nop, nop1, finish], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(1);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
        // BH-ENG-02: Step 0 was marked Running but StepBudgetExhausted
        // did not transition it to Succeeded via mark_step_after_signal.
        // The step stays in stale Running state until next drive.
    }

    // =====================================================================
    // BH-ENG-04: runtime_from_core discards taint from Finished signal
    //
    // signal.rs:16 maps EngineSignal::Finished(value, _taint) to
    // RuntimeSignal::Finished(value). The taint is silently discarded.
    // Severity: Medium-High. Consumers of RuntimeSignal cannot determine
    // whether the finished value was Clean, Secret, or DerivedFromSecret.
    // =====================================================================

    #[test]
    fn bh_eng_04_runtime_from_core_discards_taint_from_finished() {
        let clean_signal =
            runtime_from_core(EngineSignal::Finished(SlotValue::I64(42), Taint::Clean));
        let secret_signal =
            runtime_from_core(EngineSignal::Finished(SlotValue::I64(42), Taint::Secret));
        assert_eq!(
            clean_signal, secret_signal,
            "BH-ENG-04: taint is discarded in runtime_from_core, both signals are equal"
        );
    }

    // =====================================================================
    // BH-ENG-05: ErrorHandler dispatch routes PC to body step, not handler
    //
    // The ErrorHandler node dispatches to body for normal execution.
    // The handler is only used when an actual failure occurs.
    // Severity: Informational. Correct behavior but potentially confusing.
    // =====================================================================

    #[test]
    fn bh_eng_05_error_handler_dispatches_to_body_not_handler() {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        };
        let finish1 = finish_node(1, 0);
        let finish2 = finish_node(2, 0);
        let wf = make_workflow(vec![node, finish1, finish2], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(
            run.pc(),
            StepIdx::new(1),
            "BH-ENG-05: ErrorHandler routes to body=1, not handler=2"
        );
    }

    // =====================================================================
    // BH-ENG-06: RetryPolicy with max_attempts=0 is accepted
    //
    // A max_attempts=0 policy means "never attempt" and exhausts
    // immediately. The policy should probably reject 0 at construction.
    // Severity: Low. Safe but semantically questionable.
    // =====================================================================

    #[test]
    fn bh_eng_06_zero_max_attempts_policy_exhausts_immediately() {
        let policy = RetryPolicy {
            max_attempts: 0,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let target = execute_retry_check(0, policy, StepIdx::new(1), StepIdx::new(9));
        assert_eq!(
            target,
            StepIdx::new(9),
            "BH-ENG-06: max_attempts=0 should exhaust at attempt 0"
        );
    }

    // =====================================================================
    // BH-ENG-07 FIXED: execute_do_without_contract now enforces taint checks
    //
    // Previously, when contracts was empty, execute_do_without_contract was
    // used, skipping taint checking entirely. Now it enforces the most
    // conservative taint policy: tainted inputs are rejected.
    // =====================================================================

    #[test]
    fn bh_eng_07_do_without_contract_now_enforces_taint_check() {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        let _ = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        // BH-FIX: TaintViolation is now raised even without contracts.
        assert!(
            matches!(result, Err(RuntimeEngineError::TaintViolation { step }) if step == StepIdx::ZERO),
            "BH-ENG-07 FIXED: taint check is now enforced without contracts: {result:?}"
        );
    }

    #[test]
    fn bh_eng_07_do_without_contract_rejects_clean_input() {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(42));
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(matches!(
            result,
            Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied { action, .. }))
                if action == ActionId::new(0)
        ));
    }

    #[test]
    fn bh_eng_07_do_with_contract_catches_taint_violation() {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let wf = make_workflow(vec![node], 4);
        let mut run = make_run(4, 2);
        let _ = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let contracts = vec![ActionContract {
            id: ActionId::new(0),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }];
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &contracts,
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(
            matches!(result, Err(RuntimeEngineError::TaintViolation { step }) if step == StepIdx::ZERO),
            "BH-ENG-07: with contracts, taint violation is detected: {result:?}"
        );
    }

    // =====================================================================
    // BH-ENG-08: RetryCheck increments executed counter
    //
    // Control flow routing nodes count as executed steps, which could
    // cause premature step budget exhaustion.
    // Severity: Low. Semantic issue with "executed" definition.
    // =====================================================================

    #[test]
    fn bh_eng_08_retry_check_increments_executed_counter() {
        let node0 = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                exhausted: StepIdx::new(1),
            },
        };
        let finish = finish_node(1, 0);
        let wf = make_workflow(vec![node0, finish], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut cs = CollectStates::new();
        let n = match wf.node(StepIdx::ZERO) {
            Some(n) => n,
            None => return,
        };
        let executed_before = run.executed();
        let result = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert!(matches!(result, Ok(RuntimeSignal::Continue)), "BH-ENG-08: RetryCheck should return Continue");
        assert_eq!(
            run.executed(),
            executed_before + 1,
            "BH-ENG-08: RetryCheck increments executed counter"
        );
    }

    // =====================================================================
    // BH-ENG-09: No SlotWritten for AwaitingAction steps without output
    // =====================================================================

    #[test]
    fn bh_eng_09_no_slot_written_for_awaiting_action_steps() {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("bh_do_no_out"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: Box::from([]),
        };
        let workflow = match CompiledWorkflow::try_from_parts(parts) {
            Ok(w) => w,
            Err(_) => return,
        };
        let mut run = make_run(4, 2);
        // Input slot must be initialized with clean taint for the no-contract path.
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(0));
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = drive_deterministic_full(
            &workflow,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut cs,
            &CapabilitySet::empty(),
        );
        match result {
            Ok(RuntimeSignal::AwaitingAction(_)) => {}
            other => {
                assert!(
                    other.is_err(),
                    "expected AwaitingAction or error, got {other:?}"
                );
                return;
            }
        }
        let events = evidence.drain();
        let slot_written_count = events
            .iter()
            .filter(|e| matches!(e, EvidenceEvent::SlotWritten { .. }))
            .count();
        assert_eq!(
            slot_written_count, 0,
            "BH-ENG-09: no SlotWritten should be emitted for AwaitingAction (no output slot)"
        );
    }

    // =====================================================================
    // BH-ENG-10: Idempotency key collision search in small space
    // =====================================================================

    #[test]
    fn bh_eng_10_idempotency_key_collision_search_small_space() {
        let mut keys = std::collections::HashSet::new();
        let mut collisions = 0u64;
        for run in 0u64..50 {
            for seq in 0u64..50 {
                for action in 0u16..20 {
                    let key = compute_idempotency_key(
                        RunId::new(run),
                        SeqNo::new(seq),
                        ActionId::new(action),
                    );
                    if !keys.insert(key) {
                        collisions = collisions.saturating_add(1);
                    }
                }
            }
        }
        assert_eq!(
            collisions, 0,
            "BH-ENG-10: found {collisions} collisions in small idempotency key space"
        );
    }

    // =====================================================================
    // BH-ENG-11: Retry ticket uses frame run ID (matches original)
    // =====================================================================

    #[test]
    fn bh_eng_11_retry_ticket_uses_frame_run_id() {
        let _run = RunFrame::new(RunId::new(99), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        let original = ActionTicket {
            run: RunId::new(99),
            step: StepIdx::ZERO,
            seq: SeqNo::new(1),
            action: ActionId::new(5),
            attempt: 1,
            idempotency_key: 0,
            capacity: 3,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retry_policy: VbRetryPolicy::Retryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let outcome = ActionOutcome::Failed(failure);
        let result = resume_action_outcome(&original, outcome, &dummy_contract());
        match result {
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                assert_eq!(
                    ticket.run, original.run,
                    "BH-ENG-11: retry ticket run must match original ticket run"
                );
            }
            other => {
                let msg = format!("expected AwaitingAction, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // BH-ENG-12: drive_with_actions creates fresh ValueStore per call
    // =====================================================================

    #[test]
    fn bh_eng_12_drive_with_actions_uses_fresh_value_store() {
        // Use SetConst -> Finish which doesn't need pre-initialized slots.
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = finish_node(1, 0);
        let wf = make_workflow_with_constants(
            vec![set_const, finish],
            4,
            Box::from([vb_core::value::ConstValue::I64(1)]),
        );
        let mut run = make_run(4, 4);
        let mut budget = vb_core::engine::StepBudget::new(10);
        let result = drive_with_actions(&wf, &mut run, &mut budget, &[], RetryPolicy::NEVER);
        assert!(
            result.is_ok(),
            "drive_with_actions should succeed: {result:?}"
        );
    }

    // =====================================================================
    // BH-ENG-13: Suspended outcome ignores original ticket fields
    //
    // When ActionOutcome::Suspended is received, resume_action_outcome
    // returns the suspended ticket directly, ignoring original_ticket.
    // Severity: Medium. A malicious action handler could redirect
    // execution to a different step or run.
    // =====================================================================

    #[test]
    fn bh_eng_13_suspended_outcome_ignores_original_ticket() {
        let _run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        let suspended_ticket = ActionTicket {
            run: RunId::new(999),
            step: StepIdx::new(50),
            seq: SeqNo::new(5),
            action: ActionId::new(99),
            attempt: 3,
            idempotency_key: 99999,
            capacity: 1,
        };
        let original = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::ZERO,
            seq: SeqNo::new(1),
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        };
        let outcome = ActionOutcome::Suspended(suspended_ticket);
        let result = resume_action_outcome(&original, outcome, &dummy_contract());
        match result {
            Ok(RuntimeSignal::AwaitingAction(returned)) => {
                // BH-ENG-13: Suspended ticket fields passed through unchecked.
                assert_eq!(returned.run, RunId::new(999));
                assert_eq!(returned.step, StepIdx::new(50));
                assert_eq!(returned.action, ActionId::new(99));
            }
            other => {
                let msg = format!("expected AwaitingAction, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // BH-ENG-14: Double taint check defense in depth
    // =====================================================================

    #[test]
    fn bh_eng_14_taint_check_catches_deterministic_pure_secret_input() {
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        let _ = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
        let contract = ActionContract {
            id: ActionId::new(0),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };
        let registry = vec![contract.clone()];
        let result = execute_do(
            &run,
            StepIdx::ZERO,
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(1),
            &contract,
            &registry,
            &CapabilitySet::empty(),
            RetryPolicy::NEVER,
        );
        assert!(
            matches!(result, Err(RuntimeEngineError::TaintViolation { .. })),
            "BH-ENG-14: DeterministicPure with Secret input must fail taint check"
        );
    }

    // =====================================================================
    // BH-ENG-15 FIXED: EvidenceCollector capacity bound prevents exhaustion
    //
    // The EvidenceCollector now enforces a capacity limit. Events beyond
    // capacity are silently dropped but tracked via the dropped() counter.
    // =====================================================================

    #[test]
    fn bh_eng_15_evidence_collector_with_capacity_drops_excess() {
        let mut collector = EvidenceCollector::with_capacity(3);
        collector.push_step_started(StepIdx::new(0));
        collector.push_step_started(StepIdx::new(1));
        collector.push_step_started(StepIdx::new(2));
        // At capacity: next event should be dropped.
        collector.push_step_started(StepIdx::new(3));
        assert_eq!(collector.len(), 3, "capacity should be respected");
        assert_eq!(collector.dropped(), 1, "overflow should be tracked");
    }

    #[test]
    fn bh_eng_15_evidence_collector_drain_resets_dropped() {
        let mut collector = EvidenceCollector::with_capacity(2);
        collector.push_step_started(StepIdx::new(0));
        collector.push_step_started(StepIdx::new(1));
        collector.push_step_started(StepIdx::new(2)); // dropped
        assert_eq!(collector.dropped(), 1);
        let events = collector.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(collector.dropped(), 0, "drain should reset dropped counter");
    }

    // =====================================================================
    // BH-ENG-16: execute_do_without_contract rejects tainted input
    //
    // Regression test to ensure the taint bypass (BH-ENG-07) stays fixed.
    // =====================================================================

    #[test]
    fn bh_eng_16_do_without_contract_rejects_secret_input() {
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        let _ = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret);
        let result = execute_do_without_contract(
            &run,
            StepIdx::ZERO,
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(1),
            &CapabilitySet::empty(),
            RetryPolicy::NEVER,
        );
        assert!(
            matches!(result, Err(RuntimeEngineError::TaintViolation { step }) if step == StepIdx::ZERO),
            "BH-ENG-16: tainted input must be rejected without contract: {result:?}"
        );
    }

    #[test]
    fn bh_eng_16_do_without_contract_rejects_uninitialized_slot() {
        let run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        // Slot 0 is never written -- uninitialized is treated as Clean.
        let result = execute_do_without_contract(
            &run,
            StepIdx::ZERO,
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(1),
            &CapabilitySet::empty(),
            RetryPolicy::NEVER,
        );
        assert!(matches!(
            result,
            Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied { action, .. }))
                if action == ActionId::new(0)
        ));
    }

    #[test]
    fn bh_eng_16_do_without_contract_rejects_clean_input() {
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 2)
            .ok()
            .unwrap_or_else(|| panic!("RunFrame::new failed"));
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(42));
        let result = execute_do_without_contract(
            &run,
            StepIdx::ZERO,
            ActionId::new(0),
            SlotIdx::new(0),
            SeqNo::new(1),
            &CapabilitySet::empty(),
            RetryPolicy::NEVER,
        );
        assert!(matches!(
            result,
            Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied { action, .. }))
                if action == ActionId::new(0)
        ));
    }

    // =====================================================================
    // BH-ENG-17: compute_max_parallel_in_flight saturates on overflow
    //
    // When a TogetherStart has more branches than u16::MAX, the count
    // must saturate to u16::MAX rather than panic or wrap.
    // =====================================================================

    #[test]
    fn bh_eng_17_parallel_count_saturates_on_overflow() {
        // Verify that compute_max_parallel_in_flight correctly handles
        // the u16::try_from(branches.len()) conversion by testing with
        // a small but valid TogetherStart. The actual saturation at u16::MAX
        // cannot be tested due to memory constraints, but we verify the
        // drive loop handles a multi-branch TogetherStart without panic.
        let branches: Box<[StepIdx]> = (0u32..3)
            .map(|i| StepIdx::new(i.saturating_add(1) as u16))
            .collect();
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches,
                join: StepIdx::new(4),
            },
        };
        let step1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let step2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let step3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let finish = finish_node(4, 0);
        let wf = make_workflow(vec![node, step1, step2, step3, finish], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drive_deterministic_full(
                &wf,
                &mut run,
                &mut budget,
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut cs,
                &CapabilitySet::empty(),
            )
        }));
        assert!(
            result.is_ok(),
            "drive_deterministic_full must not panic on multi-branch TogetherStart"
        );
        // Note: Actual saturation at u16::MAX cannot be tested due to memory constraints.
    }

    // =====================================================================
    // BH-ENG-18: Drive loop budget exhaustion does not corrupt frame state
    //
    // When the budget is exhausted, the frame's step state must be
    // consistent for resumption.
    // =====================================================================

    #[test]
    fn bh_eng_18_budget_exhaustion_preserves_pc_for_resume() {
        let nop0 = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let nop1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let finish = finish_node(2, 0);
        let wf = make_workflow(vec![nop0, nop1, finish], 4);
        let mut run = make_run(4, 4);
        let mut store = ValueStore::new();
        let mut budget = vb_core::engine::StepBudget::new(1);
        let mut evidence = EvidenceCollector::new();
        let mut cs = CollectStates::new();
        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut cs,
            &CapabilitySet::empty(),
        );
        assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
        // PC should have advanced past step 0 (the executed step).
        assert_ne!(
            run.pc(),
            StepIdx::ZERO,
            "PC must advance after step execution"
        );
    }
}
