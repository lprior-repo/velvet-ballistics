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
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeSignal, compute_idempotency_key,
    drive_deterministic_full, execute_do, execute_do_without_contract, execute_error_handler,
    execute_retry_check, resolve_contract, resume_action_outcome,
};
use vb_core::action::ActionFailure;
use vb_core::action::ActionFailureCode;
use vb_core::action::ActionOutcome;
use vb_core::action::ActionTicket;
use vb_core::action::RetryPolicy as VbRetryPolicy;
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
    });
    let b = RuntimeSignal::AwaitingAction(ActionTicket {
        run: RunId::new(2),
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
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
fn execute_do_without_contract_returns_valid_ticket_for_any_action() {
    let run = match RunFrame::new(RunId::new(42), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let result = execute_do_without_contract(
        &run,
        StepIdx::new(3),
        ActionId::new(7),
        SlotIdx::new(0),
        SeqNo::new(5),
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            assert_eq!(ticket.run, RunId::new(42));
            assert_eq!(ticket.step, StepIdx::new(3));
            assert_eq!(ticket.action, ActionId::new(7));
            assert_eq!(ticket.seq, SeqNo::new(5));
            assert_eq!(ticket.attempt, 1);
        }
        other => {
            assert_eq!(other, Ok(RuntimeSignal::Continue));
        }
    }
}

// =====================================================================
// Resume action outcome tests
// =====================================================================

#[test]
fn resume_action_outcome_ready_continues_execution() {
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
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
    let result = resume_action_outcome(&mut run, &outcome);
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn resume_action_outcome_failed_non_retryable_returns_error() {
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
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
    let result = resume_action_outcome(&mut run, &outcome);
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
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
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
    };
    let outcome = ActionOutcome::Suspended(ticket);
    let result = resume_action_outcome(&mut run, &outcome);
    assert_eq!(
        result,
        Ok(RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(5),
            action: ActionId::new(3),
            attempt: 2,
            idempotency_key: 99,
        }))
    );
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
    );
    assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
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
