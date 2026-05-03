#![forbid(unsafe_code)]

//! Action execution helpers for runtime engine.

use vb_core::action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionOutcome, ActionTicket,
    Idempotency, propagate_action_taint,
};
use vb_core::engine::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::Taint;

use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

/// Backward-compatible execute_do.
pub fn execute_do(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    input: SlotIdx,
    seq: SeqNo,
    _contract: &ActionContract,
    registry_contracts: &[ActionContract],
) -> RuntimeEngineResult<RuntimeSignal> {
    let action_index = usize::from(action.get());
    let resolved = registry_contracts
        .get(action_index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })?;

    let input_taint = run.read_taint(input).map_err(RuntimeEngineError::Core)?;
    if resolved.idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    let output_taint = propagate_action_taint(resolved.idempotency, input_taint);

    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
    };

    if output_taint == Taint::Clean && input_taint != Taint::Clean {
        return Err(RuntimeEngineError::TaintViolation { step });
    }

    Ok(RuntimeSignal::AwaitingAction(ticket))
}

#[allow(clippy::unnecessary_wraps)]
pub fn execute_do_without_contract(
    run: &RunFrame,
    step: StepIdx,
    action: ActionId,
    _input: SlotIdx,
    seq: SeqNo,
) -> RuntimeEngineResult<RuntimeSignal> {
    let ticket = ActionTicket {
        run: run.run_id(),
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run.run_id(), seq, action),
    };
    Ok(RuntimeSignal::AwaitingAction(ticket))
}

/// Backward-compatible execute_retry_check.
pub fn execute_retry_check(
    current_attempt: u16,
    policy: RetryPolicy,
    body: StepIdx,
    exhausted: StepIdx,
) -> StepIdx {
    if current_attempt < policy.max_attempts {
        body
    } else {
        exhausted
    }
}

/// Backward-compatible execute_error_handler.
pub fn execute_error_handler(failure: &ActionFailure, handler: StepIdx, body: StepIdx) -> StepIdx {
    if failure.retry_policy == vb_core::action::RetryPolicy::Retryable
        || failure.code != ActionFailureCode::Unknown
    {
        handler
    } else {
        body
    }
}

/// Resumes an action outcome into the run frame.
pub fn resume_action_outcome(
    run: &mut RunFrame,
    outcome: &ActionOutcome,
) -> RuntimeEngineResult<RuntimeSignal> {
    match outcome {
        ActionOutcome::Ready(ready) => {
            run.write_slot_with_taint(ready.output_slot, ready.value, ready.taint)
                .map_err(RuntimeEngineError::Core)?;
            Ok(RuntimeSignal::Continue)
        }
        ActionOutcome::Suspended(ticket) => Ok(RuntimeSignal::AwaitingAction(*ticket)),
        ActionOutcome::Failed(failure) => {
            let step = run.pc();
            if failure.retry_policy == vb_core::action::RetryPolicy::Retryable {
                Ok(RuntimeSignal::AwaitingAction(ActionTicket {
                    run: run.run_id(),
                    step,
                    seq: SeqNo::ZERO,
                    action: ActionId::new(0),
                    attempt: 1,
                    idempotency_key: 0,
                }))
            } else {
                Err(RuntimeEngineError::Core(
                    EngineError::UnsupportedPrimitive {
                        primitive: "action_failed_non_retryable",
                    },
                ))
            }
        }
    }
}

/// Computes a deterministic idempotency key from run, sequence, and action.
pub fn compute_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.get());
    let seq_part = u128::from(seq.get()) << 64;
    let action_part = u128::from(u32::from(action.get())) << 80;
    match run_part.checked_add(seq_part) {
        Some(combined) => match combined.checked_add(action_part) {
            Some(key) => key,
            None => run_part,
        },
        None => run_part,
    }
}

/// Resolves an action contract from the registry.
pub fn resolve_contract(
    action: ActionId,
    contracts: &[ActionContract],
) -> RuntimeEngineResult<&ActionContract> {
    let index = usize::from(action.get());
    contracts
        .get(index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })
        .map_err(RuntimeEngineError::Action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::action::{Idempotency, RetrySafety, SideEffect};
    use vb_core::ids::{RunId, SeqNo};

    // =====================================================================
    // compute_idempotency_key
    // =====================================================================

    #[test]
    fn idempotency_key_is_deterministic() {
        let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(2), ActionId::new(3));
        let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(2), ActionId::new(3));
        assert_eq!(key1, key2);
    }

    #[test]
    fn idempotency_key_differs_for_different_runs() {
        let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
        let key2 = compute_idempotency_key(RunId::new(2), SeqNo::new(0), ActionId::new(0));
        assert_ne!(key1, key2);
    }

    #[test]
    fn idempotency_key_differs_for_different_seq() {
        let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
        let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(1), ActionId::new(0));
        assert_ne!(key1, key2);
    }

    #[test]
    fn idempotency_key_differs_for_different_action() {
        let key1 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(0));
        let key2 = compute_idempotency_key(RunId::new(1), SeqNo::new(0), ActionId::new(1));
        assert_ne!(key1, key2);
    }

    #[test]
    fn idempotency_key_with_zero_inputs() {
        let key = compute_idempotency_key(RunId::new(0), SeqNo::new(0), ActionId::new(0));
        assert_eq!(key, 0);
    }

    #[test]
    fn idempotency_key_with_large_values() {
        let key1 =
            compute_idempotency_key(RunId::new(u64::MAX), SeqNo::new(u64::MAX), ActionId::new(65535));
        let key2 =
            compute_idempotency_key(RunId::new(u64::MAX), SeqNo::new(u64::MAX), ActionId::new(65535));
        assert_eq!(key1, key2);
    }

    // =====================================================================
    // execute_retry_check
    // =====================================================================

    #[test]
    fn retry_check_routes_to_body_when_below_max() {
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
    fn retry_check_routes_to_exhausted_at_max() {
        let policy = RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let target = execute_retry_check(3, policy, StepIdx::new(5), StepIdx::new(10));
        assert_eq!(target, StepIdx::new(10));
    }

    #[test]
    fn retry_check_routes_to_exhausted_above_max() {
        let policy = RetryPolicy {
            max_attempts: 2,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let target = execute_retry_check(5, policy, StepIdx::new(1), StepIdx::new(9));
        assert_eq!(target, StepIdx::new(9));
    }

    #[test]
    fn retry_check_never_policy_always_exhausts_after_one() {
        let target = execute_retry_check(1, RetryPolicy::NEVER, StepIdx::new(3), StepIdx::new(7));
        assert_eq!(target, StepIdx::new(7));
    }

    #[test]
    fn retry_check_default_policy_allows_two_retries() {
        let target = execute_retry_check(2, RetryPolicy::DEFAULT, StepIdx::new(1), StepIdx::new(8));
        assert_eq!(target, StepIdx::new(1));
    }

    #[test]
    fn retry_check_default_policy_exhausts_at_three() {
        let target = execute_retry_check(3, RetryPolicy::DEFAULT, StepIdx::new(1), StepIdx::new(8));
        assert_eq!(target, StepIdx::new(8));
    }

    // =====================================================================
    // execute_error_handler
    // =====================================================================

    #[test]
    fn error_handler_routes_to_handler_on_retryable_failure() {
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retry_policy: vb_core::action::RetryPolicy::Retryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let target = execute_error_handler(&failure, StepIdx::new(5), StepIdx::new(3));
        assert_eq!(target, StepIdx::new(5));
    }

    #[test]
    fn error_handler_routes_to_handler_on_non_unknown_code() {
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retry_policy: vb_core::action::RetryPolicy::NonRetryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
        assert_eq!(target, StepIdx::new(8));
    }

    #[test]
    fn error_handler_routes_to_body_on_unknown_non_retryable() {
        let failure = ActionFailure {
            code: ActionFailureCode::Unknown,
            retry_policy: vb_core::action::RetryPolicy::NonRetryable,
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
            retry_policy: vb_core::action::RetryPolicy::Retryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
        assert_eq!(target, StepIdx::new(8));
    }

    // =====================================================================
    // resolve_contract
    // =====================================================================

    fn make_contract(id: u16) -> ActionContract {
        ActionContract {
            id: ActionId::new(id),
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
    fn resolve_contract_returns_unknown_for_empty_registry() {
        let contracts: Vec<ActionContract> = Vec::new();
        let result = resolve_contract(ActionId::new(0), &contracts);
        assert_eq!(
            result,
            Err(RuntimeEngineError::Action(ActionError::UnknownAction {
                action: ActionId::new(0),
            }))
        );
    }

    #[test]
    fn resolve_contract_finds_matching_contract_by_index_and_id() {
        let contracts = vec![make_contract(0), make_contract(1), make_contract(2)];
        let result = resolve_contract(ActionId::new(1), &contracts);
        match result {
            Ok(contract) => assert_eq!(contract.id, ActionId::new(1)),
            Err(e) => {
                let msg = format!("expected Ok, got {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn resolve_contract_rejects_id_mismatch() {
        // Contract at index 0 has id=0, but we request id=99 at index 99
        let contracts = vec![make_contract(0)];
        let result = resolve_contract(ActionId::new(99), &contracts);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_contract_rejects_when_index_matches_but_id_differs() {
        // Contract at index 0 has id=5 (not 0), so index lookup returns it
        // but the filter rejects it because c.id != action(0)
        let mut c = make_contract(5);
        c.id = ActionId::new(5);
        let contracts = vec![c];
        // ActionId::new(0) -> index 0, but contract there has id=5, mismatch
        let result = resolve_contract(ActionId::new(0), &contracts);
        assert!(
            result.is_err(),
            "expected error when id at index does not match requested action"
        );
    }

    #[test]
    fn resolve_contract_returns_first_contract() {
        let contracts = vec![make_contract(0)];
        let result = resolve_contract(ActionId::new(0), &contracts);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_contract_returns_last_contract() {
        let contracts = vec![make_contract(0), make_contract(1), make_contract(2)];
        let result = resolve_contract(ActionId::new(2), &contracts);
        assert!(result.is_ok());
    }
}
